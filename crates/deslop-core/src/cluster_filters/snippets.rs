//! Snippet construction and per-report CST caching for the cluster-noise
//! filters.
//!
//! A [`Snippet`] pairs a cluster member's raw source with its byte range
//! and a shared, lazily-parsed tree-sitter CST. [`ParseCache`] guarantees
//! each source file is parsed at most once per report, so a large
//! generated file clustered hundreds of ways is never re-parsed per
//! cluster ([CLONE-NOISE-REPARSE-CACHE]). The orchestration that walks
//! these snippets lives in the parent [`super`] module.

use std::{cell::RefCell, collections::HashMap, hash::BuildHasher, rc::Rc};

use super::contract_index::ContractIndex;
use crate::{ast::ByteRange, fingerprint::Fingerprint, lang::shared::parse_source, state::FileId};

/// One re-parsed cluster member: language, raw bytes, the byte range
/// inside `source` that the fingerprint covered, and the originating
/// [`FileId`] so cross-file uniqueness checks do not depend on
/// pointer identity.
pub(crate) struct Snippet<'a> {
    /// Language id used to select the tree-sitter grammar.
    pub(crate) language: &'static str,
    /// Full file source bytes for the member.
    pub(crate) source: &'a [u8],
    /// Byte range covered by the member fingerprint.
    pub(crate) range: ByteRange,
    /// Registry id of the source file containing this member.
    pub(crate) file_id: FileId,
    /// CST for `source`, parsed once per file and shared (via `Rc`) across
    /// every member from the same file. A large file (e.g. a 30k-line
    /// generated FFI binding clustered hundreds of ways) is therefore
    /// parsed at most once per cluster instead of once per filter per
    /// member. `None` when the language has no registered grammar here.
    tree: Option<Rc<tree_sitter::Tree>>,
}

/// Per-report cache of parsed tree-sitter CSTs keyed by file. A file is
/// parsed at most once per report regardless of how many clusters
/// reference it. Without this, a large generated file — e.g. a 30k-line
/// FFI binding clustered hundreds of ways — would be re-parsed once per
/// cluster and dominate analysis time ([CLONE-NOISE-REPARSE-CACHE]).
#[derive(Default)]
pub(crate) struct ParseCache {
    /// Lazily-populated map from file id to its parsed CST (or `None`
    /// when the language has no grammar / parsing failed).
    trees: RefCell<HashMap<FileId, Option<Rc<tree_sitter::Tree>>>>,
    /// Lazily-built corpus-wide contract index per language
    /// ([CLONE-NOISE-POLYMORPHIC-CONTRACT]). Built only when a cluster
    /// reaches the contract question, so a report with no same-named
    /// cross-file candidate never pays for it.
    contracts: RefCell<HashMap<&'static str, Rc<ContractIndex>>>,
    /// Kind membership per `(file, byte range)`, fused into one walk
    /// ([PERF-FLUTTER-TODO-CORPUS]). A corpus-scale report asks the
    /// same member ranges repeatedly — across clusters and across the
    /// noise, category, and ranking passes — and each ask used to walk
    /// the member subtree once per kind. One memoised walk per distinct
    /// range replaces all of them.
    field_kinds: RefCell<HashMap<(FileId, usize, usize), FieldKinds>>,
}

/// Which shape-defining kinds a member subtree contains — the fused
/// answer to the four membership questions the Dart field filter used
/// to ask with four separate walks.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FieldKinds {
    /// Any `function_body` in the subtree.
    pub has_body: bool,
    /// Any `function_expression` in the subtree.
    pub has_function_expression: bool,
    /// Any `static_final_declaration_list` in the subtree.
    pub has_static_final_list: bool,
    /// Any `initialized_identifier_list` in the subtree.
    pub has_initialized_identifier_list: bool,
}

impl ParseCache {
    /// Creates an empty cache scoped to one report render.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Which shape-defining kinds `node`'s subtree contains, memoised by
    /// `(file, range)` — one walk per distinct member range, however
    /// many clusters and passes ask ([PERF-FLUTTER-TODO-CORPUS]).
    pub(crate) fn dart_field_kinds(
        &self,
        file_id: FileId,
        node: tree_sitter::Node<'_>,
    ) -> FieldKinds {
        let key = (file_id, node.start_byte(), node.end_byte());
        if let Some(hit) = self.field_kinds.borrow().get(&key) {
            return *hit;
        }
        let mut kinds = FieldKinds::default();
        collect_field_kinds(node, &mut kinds);
        let _previous = self.field_kinds.borrow_mut().insert(key, kinds);
        kinds
    }

    /// Returns the cached CST for `file_id`, parsing `source` with the
    /// `language` grammar on first request. `None` when the language has
    /// no registered grammar here or parsing fails.
    pub(crate) fn tree_for(
        &self,
        file_id: FileId,
        language: &'static str,
        source: &[u8],
    ) -> Option<Rc<tree_sitter::Tree>> {
        if let Some(cached) = self.trees.borrow().get(&file_id) {
            return cached.clone();
        }
        let parsed = grammar_for(language)
            .as_ref()
            .and_then(|grammar| parse_source(language, grammar, source).ok())
            .map(Rc::new);
        let _previous = self.trees.borrow_mut().insert(file_id, parsed.clone());
        parsed
    }

    /// Returns the corpus-wide contract index for `language`, building it
    /// on first request from every same-language file in the report and
    /// reusing the per-file trees this cache already holds.
    pub(super) fn contracts<S: BuildHasher>(
        &self,
        sources: &HashMap<FileId, Vec<u8>>,
        file_languages: &HashMap<FileId, &'static str, S>,
        language: &'static str,
    ) -> Rc<ContractIndex> {
        let cached = self.contracts.borrow().get(language).map(Rc::clone);
        if let Some(index) = cached {
            return index;
        }
        let built = Rc::new(ContractIndex::build(
            sources,
            file_languages,
            language,
            self,
        ));
        let _previous = self
            .contracts
            .borrow_mut()
            .insert(language, Rc::clone(&built));
        built
    }
}

/// Returns a single language id when every member shares it.
pub(crate) fn uniform_language<S: BuildHasher>(
    members: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Option<&'static str> {
    let first = file_languages.get(&members.first()?.file_id)?;
    if members
        .iter()
        .all(|member| file_languages.get(&member.file_id) == Some(first))
    {
        Some(*first)
    } else {
        None
    }
}

/// Collects one [`Snippet`] per member, returning `None` if any member's
/// source bytes are unavailable. Each distinct source file is parsed at
/// most once for the whole report via `cache`, so downstream filters
/// re-walk a cached CST rather than re-parsing per member or per cluster.
pub(crate) fn collect_snippets<'a>(
    members: &[Fingerprint],
    sources: &'a HashMap<FileId, Vec<u8>>,
    language: &'static str,
    cache: &ParseCache,
) -> Option<Vec<Snippet<'a>>> {
    members
        .iter()
        .map(|member| {
            let source = sources.get(&member.file_id)?;
            let tree = cache.tree_for(member.file_id, language, source);
            Some(Snippet {
                language,
                source: source.as_slice(),
                range: member.byte_range,
                file_id: member.file_id,
                tree,
            })
        })
        .collect()
}

/// Returns the snippet's pre-parsed tree-sitter CST so filters can walk a
/// real CST instead of the normalised one. The tree is parsed once per
/// file in [`collect_snippets`]; this is a cheap `Rc` clone. Returns
/// `None` when the language has no registered grammar here.
pub(crate) fn parse_for(snippet: &Snippet<'_>) -> Option<Rc<tree_sitter::Tree>> {
    snippet.tree.clone()
}

/// Maps a language id to its tree-sitter grammar.
fn grammar_for(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "csharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "dart" => Some(tree_sitter_dart::LANGUAGE.into()),
        "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "fsharp" => Some(tree_sitter_fsharp::LANGUAGE_FSHARP.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        _ => None,
    }
}


/// Folds the shape-defining kind membership of `node`'s subtree into
/// `kinds` — the single walk that replaces four per-kind walks.
fn collect_field_kinds(node: tree_sitter::Node<'_>, kinds: &mut FieldKinds) {
    match node.kind() {
        "function_body" => kinds.has_body = true,
        "function_expression" => kinds.has_function_expression = true,
        "static_final_declaration_list" => kinds.has_static_final_list = true,
        "initialized_identifier_list" => kinds.has_initialized_identifier_list = true,
        _ => {}
    }
    if kinds.has_body
        && kinds.has_function_expression
        && kinds.has_static_final_list
        && kinds.has_initialized_identifier_list
    {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_field_kinds(child, kinds);
    }
}
