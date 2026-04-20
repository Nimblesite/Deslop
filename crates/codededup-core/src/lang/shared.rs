//! Shared language-parser plumbing.
//!
//! Every `LanguageParser` implementation ([PIPELINE-LANG-TRAIT]) walks the
//! tree-sitter tree, applies a per-language kind normalisation rule, and
//! produces a [`NormalizedNode`]. The walking, kind interning, and
//! normalised-root assembly are identical across languages — only the
//! `raw_kind -> normalised_kind` mapping differs. This module centralises
//! the shared machinery so a new language is one file with one `match`
//! arm (see [`crate::lang::csharp`], [`crate::lang::rust_lang`],
//! [`crate::lang::python`]).

use tree_sitter::{Language, Node, Parser, Tree};

use crate::{
    ast::{ByteRange, NormalizedNode},
    error::CoreError,
    state::FileId,
};

/// Synthetic kind assigned to the normalised root of every parse tree.
/// Keeps per-file roots visibly distinct from any grammar-produced kind,
/// which matters because the root is the only node whose `kind` is
/// language-independent.
pub const FILE_KIND: &str = "__file__";
/// Normalised identifier placeholder. Language parsers collapse
/// identifier-like raw kinds to this value so Type-2 renamed clones
/// fingerprint identically.
pub const IDENTIFIER_KIND: &str = "__ident__";
/// Normalised literal placeholder. Language parsers collapse string /
/// numeric / char / bool / null literal raw kinds to this value so
/// constant edits do not perturb the fingerprint.
pub const LITERAL_KIND: &str = "__literal__";

/// Parses `source` with `language` and returns the tree-sitter
/// [`Tree`]. Wraps the two possible failure modes in [`CoreError`]
/// variants so language plug-ins never call into `panic!`.
///
/// # Errors
///
/// - [`CoreError::GrammarLoad`] if the compiled grammar cannot be bound
///   to a new [`Parser`].
/// - [`CoreError::ParseFailed`] if tree-sitter returns `None`.
pub fn parse_source(
    language_id: &'static str,
    language: &Language,
    source: &[u8],
) -> Result<Tree, CoreError> {
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .map_err(|source| CoreError::GrammarLoad {
            language: language_id,
            source,
        })?;
    parser.parse(source, None).ok_or(CoreError::ParseFailed {
        language: language_id,
    })
}

/// Walks `tree`'s named children, applies `normalise_kind` to each, and
/// wraps the result in a `__file__`-rooted [`NormalizedNode`]. Returns a
/// complete normalised AST ready for fingerprinting
/// ([PIPELINE-NORMALIZE-AST]).
#[must_use]
pub fn build_normalised_root(
    tree: &Tree,
    file_id: FileId,
    normalise_kind: fn(&str) -> Option<&'static str>,
) -> NormalizedNode {
    let root = tree.root_node();
    let mut children = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if let Some(node) = normalise_node(child, file_id, normalise_kind) {
            children.push(node);
        }
    }
    NormalizedNode {
        kind: FILE_KIND,
        children,
        byte_range: ByteRange {
            start: root.start_byte(),
            end: root.end_byte(),
        },
        file_id,
    }
}

/// Recursively normalises one tree-sitter [`Node`]. Returns `None` when
/// `normalise_kind` drops the node (trivia / comments / per-language
/// noise).
fn normalise_node(
    node: Node<'_>,
    file_id: FileId,
    normalise_kind: fn(&str) -> Option<&'static str>,
) -> Option<NormalizedNode> {
    let kind = normalise_kind(node.kind())?;
    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(child_node) = normalise_node(child, file_id, normalise_kind) {
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

/// Interns `raw` into a `&'static str` backed by a thread-local cache.
/// Tree-sitter returns grammar kind strings that live for the duration
/// of the loaded grammar, but we promote them to `&'static` via an
/// explicit leak so ownership is independent of any grammar instance.
#[must_use]
pub fn intern_kind(raw: &str) -> &'static str {
    KIND_INTERNER.with(|cache| intern(&mut cache.borrow_mut(), raw))
}

thread_local! {
    static KIND_INTERNER: std::cell::RefCell<Vec<&'static str>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Returns the canonical `&'static str` for `raw`, allocating once
/// per previously unseen kind and caching it for reuse.
fn intern(entries: &mut Vec<&'static str>, raw: &str) -> &'static str {
    if let Some(existing) = entries.iter().find(|stored| **stored == raw) {
        return existing;
    }
    let leaked: &'static str = Box::leak(raw.to_owned().into_boxed_str());
    entries.push(leaked);
    leaked
}
