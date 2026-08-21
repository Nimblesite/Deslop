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
/// Normalised operator placeholder ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// Every grammar here spells the operator of a binary, unary or
/// compound-assignment expression as an *anonymous* token, and the walk
/// below reads named children only. `alpha + beta` and `alpha - beta`
/// therefore produced the same normalised subtree with the same
/// identifier frontier: the pipeline held no evidence at all that they
/// differ, and rendered `structural = 1.00`, `token_jaccard = 1.00`,
/// `agreement = 1.00`, `fused = 1.00` over code that computes a
/// different answer.
///
/// Operators collapse to *one* kind, exactly as identifiers and
/// literals do, so a consistently-renamed clone still fingerprints
/// identically and Type-2 recall is untouched. What the placeholder
/// adds is a position on the content frontier
/// ([`crate::tokens::collapsed_leaves`]) whose raw bytes are the
/// operator itself — so `+` and `-` disagree where they always should
/// have, and [FUSION-CONTENT-GATE] prices the difference.
pub const OPERATOR_KIND: &str = "__op__";

/// Maximum nesting depth of a normalised AST. Files whose tree-sitter tree
/// nests deeper than this are rejected with [`CoreError::AstTooDeep`], so a
/// pathologically deep file is skipped rather than analysed (#168).
///
/// This is a bound on work, not a stack-safety mechanism, and it must not be
/// used as one. It was: the merkle walk recursed with a ~1.9 KB
/// `blake3::Hasher` live per frame, so ~450 accepted levels exhausted a 1 MB
/// stack and killed the process — the guard sat *above* the overflow point
/// and admitted exactly the files that crashed. Lowering it to 150 hid that
/// by skipping 36 real `dotnet/fsharp` files, converting a crash into silent
/// false negatives. The walks are iterative instead
/// ([`crate::fingerprint`]), so depth no longer consumes stack and this
/// value is free to bound work alone.
pub const MAX_AST_DEPTH: usize = 500;

/// Anonymous tokens that carry behaviour, kept as [`OPERATOR_KIND`]
/// leaves ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// Tree-sitter names an anonymous node by its own token text, so this
/// is one list for every grammar rather than a per-language table. It
/// is an allowlist and not a punctuation denylist on purpose: brackets,
/// commas, semicolons, colons, dots, arrows and the plain `=` of an
/// assignment are *framing* — the parent production already says what
/// they mean, and keeping them would inflate every subtree with nodes
/// no two members can ever disagree on. Everything here changes what
/// the code computes.
const BEHAVIOUR_BEARING_TOKENS: &[&str] = &[
    // Arithmetic, including Python's floor-divide and power.
    "+", "-", "*", "/", "%", "**", "//",
    // Comparison, including the strict forms and the legacy `<>`.
    "==", "!=", "===", "!==", "<", ">", "<=", ">=", "<>",
    // Boolean, symbolic and worded.
    "&&", "||", "!", "and", "or", "not",
    // Membership and identity — Python spells these as bare tokens
    // inside a comparison, and `x in xs` versus `x is xs` is not a
    // rename.
    "in", "is",
    // Bitwise and shifts.
    "&", "|", "^", "~", "<<", ">>", ">>>",
    // Compound assignment: the operator is the whole behaviour.
    "+=", "-=", "*=", "/=", "%=", "**=", "//=", "&=", "|=", "^=", "<<=", ">>=", ">>>=", "&&=",
    "||=", "??=",
    // Null handling.
    "??", "?.",
    // Ranges — an inclusive bound is not the same loop as an exclusive
    // one.
    "..", "..=", "...",
];

/// True when an anonymous token changes what the code computes and must
/// therefore survive normalisation as an [`OPERATOR_KIND`] leaf.
#[must_use]
pub fn is_behaviour_bearing_token(token: &str) -> bool {
    BEHAVIOUR_BEARING_TOKENS.contains(&token)
}

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
    let children = normalise_children(root, file_id, normalise_kind, language, 1)?;
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
    let children = normalise_children(node, file_id, normalise_kind, language, depth)?;
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

/// Normalises every child of `node` — named children through
/// [`normalise_node`], anonymous ones through
/// [`is_behaviour_bearing_token`] ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// The two are collected in one pass over `children`, not two, so an
/// operator keeps its position between its operands. A frontier that
/// listed `alpha`, `beta`, `+` rather than `alpha`, `+`, `beta` would
/// still detect the change, but it would stop aligning positionally
/// with a member whose operand count differs.
fn normalise_children(
    node: Node<'_>,
    file_id: FileId,
    normalise_kind: fn(&str) -> Option<&'static str>,
    language: &'static str,
    depth: usize,
) -> Result<Vec<NormalizedNode>, CoreError> {
    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let normalised = if child.is_named() {
            normalise_node(
                child,
                file_id,
                normalise_kind,
                language,
                depth.saturating_add(1),
            )?
        } else {
            operator_leaf(child, file_id)
        };
        if let Some(child_node) = normalised {
            children.push(child_node);
        }
    }
    Ok(children)
}

/// The [`OPERATOR_KIND`] leaf for one anonymous token, or `None` when
/// the token is framing rather than behaviour.
fn operator_leaf(token: Node<'_>, file_id: FileId) -> Option<NormalizedNode> {
    is_behaviour_bearing_token(token.kind()).then(|| NormalizedNode {
        kind: OPERATOR_KIND,
        children: Vec::new(),
        byte_range: ByteRange {
            start: token.start_byte(),
            end: token.end_byte(),
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
