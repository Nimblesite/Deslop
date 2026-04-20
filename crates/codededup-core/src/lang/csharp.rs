//! C# language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for C# using the `tree-sitter-c-sharp`
//! grammar. Normalisation rules (mapping to [PIPELINE-NORMALIZE-AST]):
//!
//! - `identifier`, `predefined_type` → collapsed to `"__ident__"` so renamed
//!   variables / type names hash identically (Type-2 invariance).
//! - String, integer, real, character, boolean, null literals → collapsed to
//!   `"__literal__"` so changed constants do not perturb the fingerprint.
//! - `comment` nodes and pure-whitespace tokens are dropped.
//! - All other named node kinds pass through with their grammar name
//!   preserved.
//!
//! Anonymous tree-sitter nodes (punctuation / keywords without named kinds)
//! are dropped — their structural signal is already carried by the parent
//! node's named children.

use tree_sitter::{Node, Parser, Tree};

use crate::{
    ast::{ByteRange, NormalizedNode},
    error::CoreError,
    lang::LanguageParser,
    state::FileId,
};

/// Stable language identifier reported by [`CSharpParser::id`].
const LANGUAGE_ID: &str = "csharp";
/// Placeholder kind used for every identifier-like tree-sitter node so that
/// renamed-identifier clones fingerprint identically.
const IDENTIFIER_KIND: &str = "__ident__";
/// Placeholder kind used for every literal tree-sitter node so that changed
/// constants do not perturb the fingerprint.
const LITERAL_KIND: &str = "__literal__";
/// Synthetic kind assigned to the root node so top-level trees are always
/// distinguishable from inner subtrees.
const FILE_KIND: &str = "__file__";

/// C# implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct CSharpParser;

impl CSharpParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for CSharpParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["cs"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_c_sharp::language()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        let tree = parse_source(&self.grammar(), source)?;
        let root = tree.root_node();
        let mut children = Vec::new();
        let mut cursor = root.walk();
        for child in root.named_children(&mut cursor) {
            if let Some(node) = normalize_node(child, file_id) {
                children.push(node);
            }
        }
        Ok(NormalizedNode {
            kind: FILE_KIND,
            children,
            byte_range: ByteRange {
                start: root.start_byte(),
                end: root.end_byte(),
            },
            file_id,
        })
    }
}

/// Parses `source` with `language`. Returns a tree-sitter [`Tree`] on
/// success.
fn parse_source(language: &tree_sitter::Language, source: &[u8]) -> Result<Tree, CoreError> {
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .map_err(|source| CoreError::GrammarLoad {
            language: LANGUAGE_ID,
            source,
        })?;
    parser
        .parse(source, None)
        .ok_or(CoreError::ParseFailed {
            language: LANGUAGE_ID,
        })
}

/// Converts a tree-sitter [`Node`] and its descendants into a
/// [`NormalizedNode`] subtree. Returns `None` when the node itself should be
/// dropped from the normalised AST (comments, ignored trivia).
fn normalize_node(node: Node<'_>, file_id: FileId) -> Option<NormalizedNode> {
    let kind = normalize_kind(node.kind())?;
    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(child_node) = normalize_node(child, file_id) {
            children.push(child_node);
        }
    }
    Some(NormalizedNode {
        kind,
        children,
        byte_range: ByteRange {
            start: node.start_byte(),
            end: node.end_byte(),
        },
        file_id,
    })
}

/// Maps a tree-sitter C# node kind to its normalised form. Returns `None`
/// when the node should be dropped entirely (pure trivia). The returned
/// `&'static str` is taken from a fixed set so hashing is cheap and stable.
fn normalize_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" => None,
        "identifier" | "predefined_type" | "type_parameter" => Some(IDENTIFIER_KIND),
        "string_literal"
        | "verbatim_string_literal"
        | "interpolated_string_text"
        | "integer_literal"
        | "real_literal"
        | "character_literal"
        | "boolean_literal"
        | "null_literal" => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}

/// Interns a node-kind string into a `&'static str`. Tree-sitter already
/// returns kinds from a fixed table compiled into the grammar — the strings
/// live for the life of the loaded grammar, so it is safe to promote them to
/// `&'static` here.
fn intern_kind(raw: &str) -> &'static str {
    // SAFETY rationale (no `unsafe` used): the tree-sitter C# grammar stores
    // kind strings in a static table that the library keeps alive for the
    // duration of the process. We copy those strings into a leaked
    // `String` so ownership is explicit and independent of the grammar
    // lifetime, at the cost of a one-time allocation per unique kind.
    KIND_INTERNER.with(|cache| cache.borrow_mut().intern(raw))
}

thread_local! {
    static KIND_INTERNER: std::cell::RefCell<KindInterner> =
        const { std::cell::RefCell::new(KindInterner::new()) };
}

/// Small deterministic string interner keyed on tree-sitter node kind.
struct KindInterner {
    /// Previously interned kinds. Linear scan is fine — the C# grammar has
    /// on the order of 200 distinct node kinds.
    entries: Vec<&'static str>,
}

impl KindInterner {
    /// Creates an empty interner. `const` so it can back a `thread_local!`
    /// without a runtime initialiser.
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Returns the canonical `&'static str` for `raw`, allocating once per
    /// previously unseen kind and caching it for reuse.
    fn intern(&mut self, raw: &str) -> &'static str {
        if let Some(existing) = self.entries.iter().find(|stored| **stored == raw) {
            return existing;
        }
        let leaked: &'static str = Box::leak(raw.to_owned().into_boxed_str());
        self.entries.push(leaked);
        leaked
    }
}
