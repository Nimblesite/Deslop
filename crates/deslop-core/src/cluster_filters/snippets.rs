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
}

impl ParseCache {
    /// Creates an empty cache scoped to one report render.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
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
        _ => None,
    }
}
