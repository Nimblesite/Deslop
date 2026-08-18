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

/// Maximum nesting depth of a normalised AST. Files whose tree-sitter
/// tree nests deeper than this are rejected with [`CoreError::AstTooDeep`]
/// so the deep structure never reaches the pipeline's recursive tree
/// walks (fingerprinting, sibling windows, token extraction), which would
/// otherwise overflow the stack and abort the whole run.
///
/// Real source ASTs are at most low-hundreds deep, so this leaves ample
/// headroom while staying well under the overflow threshold on both 1 MB
/// default thread stacks (Windows) and async worker threads (the
/// fingerprint walk runs only on accepted files at depth `< MAX_AST_DEPTH`).
pub const MAX_AST_DEPTH: usize = 150;

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
///
/// # Errors
///
/// Returns [`CoreError::AstTooDeep`] when the tree nests deeper than
/// [`MAX_AST_DEPTH`], so a pathologically deep file is skipped rather than
/// overflowing the pipeline's recursive walks.
pub fn build_normalised_root(
    tree: &Tree,
    file_id: FileId,
    normalise_kind: fn(&str) -> Option<&'static str>,
    language: &'static str,
) -> Result<NormalizedNode, CoreError> {
    let root = tree.root_node();
    let mut children = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if let Some(node) = normalise_node(child, file_id, normalise_kind, language, 1)? {
            children.push(node);
        }
    }
    Ok(NormalizedNode {
        kind: FILE_KIND,
        byte_range: retained_span(&children).unwrap_or(ByteRange {
            start: root.start_byte(),
            end: root.end_byte(),
        }),
        children,
        file_id,
    })
}

/// The extent of the nodes normalisation actually kept
/// ([PIPELINE-NORMALIZE-AST]).
///
/// `__file__` is a synthetic root, not real syntax, so its span must be
/// what it contains. Tree-sitter's parse root spans leading and trailing
/// trivia — a licence header, a padding comment block — that
/// [`normalise_node`] has already dropped, so inheriting it reports
/// bytes contributing zero nodes to the match: a whole-file occurrence
/// then opens on comments instead of the code it duplicates, and its
/// range no longer tracks the edit that moved that code.
///
/// Real nodes keep their own span. A class's braces belong to the
/// duplication even when a comment sits between them, so this narrowing
/// applies to the synthetic root alone. `None` when the file normalised
/// to nothing.
fn retained_span(children: &[NormalizedNode]) -> Option<ByteRange> {
    let start = children.iter().map(|child| child.byte_range.start).min()?;
    let end = children.iter().map(|child| child.byte_range.end).max()?;
    Some(ByteRange { start, end })
}

/// Recursively normalises one tree-sitter [`Node`] at nesting `depth`.
/// Returns `Ok(None)` when `normalise_kind` drops the node (trivia /
/// comments / per-language noise) and [`CoreError::AstTooDeep`] when
/// `depth` exceeds [`MAX_AST_DEPTH`] — the depth guard bounds every
/// downstream recursive walk by rejecting the file at its single
/// construction chokepoint.
fn normalise_node(
    node: Node<'_>,
    file_id: FileId,
    normalise_kind: fn(&str) -> Option<&'static str>,
    language: &'static str,
    depth: usize,
) -> Result<Option<NormalizedNode>, CoreError> {
    if depth > MAX_AST_DEPTH {
        return Err(CoreError::AstTooDeep {
            language,
            limit: MAX_AST_DEPTH,
        });
    }
    let Some(kind) = normalise_kind(node.kind()) else {
        return Ok(None);
    };
    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(child_node) = normalise_node(
            child,
            file_id,
            normalise_kind,
            language,
            depth.saturating_add(1),
        )? {
            children.push(child_node);
        }
    }
    Ok(Some(NormalizedNode {
        kind,
        children,
        byte_range: ByteRange {
            start: node.start_byte(),
            end: node.end_byte(),
        },
        file_id,
    }))
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
