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
/// Namespace prefix for a normalised operator leaf
/// ([PIPELINE-NORMALIZE-AST-OPERATOR]).
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
/// The operator is kept as a leaf **carrying its own token**, and that
/// is the whole point: the kind is `__op__+`, never a shared `__op__`.
/// Collapsing operators to one placeholder — the way [`IDENTIFIER_KIND`]
/// and [`LITERAL_KIND`] collapse theirs — breaks the premise the digest
/// rests on. Identifiers and literals collapse because a rename and a
/// constant edit preserve behaviour, so equal hashes mean "the same code
/// up to renames" and unequal hashes mean the code itself differs
/// ([`crate::cluster_filters`] states that premise and elects on it). An
/// operator swap is neither a rename nor a literal edit, so a shared
/// placeholder makes `alpha + beta` and `alpha - beta` hash identically
/// and the fingerprint asserts sameness that does not exist. Everything
/// reading the digest then inherits it: `structural` saturates at 1.00,
/// `token_jaccard` echoes it, the LSH bands collide, subsumption elects
/// between views of code that computes different answers, and
/// [FUSION-CONTENT-GATE] is left pricing four disagreeing frontier
/// positions out of twenty as a ten-percent discount — `fused = 0.90`,
/// over the [FUSED-THRESHOLD] act-now line.
///
/// Type-2 recall is untouched. A consistently-renamed clone changes
/// identifiers and literals, which still collapse; it does not change
/// its operators, so the operator leaves stay equal and the subtree
/// still hashes identically.
///
/// The prefix keeps the namespace disjoint from every grammar kind, so
/// an operator can never be confused with a named production that
/// happens to spell itself `in`, `is` or `not`.
pub const OPERATOR_KIND_PREFIX: &str = "__op__";

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

/// Anonymous tokens that carry behaviour, as `(token, normalised
/// kind)` ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// The kind is written out rather than built from
/// [`OPERATOR_KIND_PREFIX`] at runtime so normalisation allocates
/// nothing per operator: `operator_leaf` runs on every anonymous child
/// of every node of every file. `tests::every_row_is_its_own_token`
/// proves each row's kind is exactly the prefix plus its own token and
/// that no two rows collide — a typo pairing `<<=` with `__op__<<`
/// would silently make two different operators one subtree, which is
/// the defect this whole section exists to remove.
///
/// Tree-sitter names an anonymous node by its own token text, so this
/// is one list for every grammar rather than a per-language table. It
/// is an allowlist and not a punctuation denylist on purpose: brackets,
/// commas, semicolons, colons, dots, arrows and the plain `=` of an
/// assignment are *framing* — the parent production already says what
/// they mean, and keeping them would inflate every subtree with nodes
/// no two members can ever disagree on. Everything here changes what
/// the code computes.
const BEHAVIOUR_BEARING_TOKENS: &[(&str, &str)] = &[
    ("+", "__op__+"),
    ("-", "__op__-"),
    ("*", "__op__*"),
    ("/", "__op__/"),
    ("%", "__op__%"),
    ("**", "__op__**"),
    ("//", "__op__//"),
    ("==", "__op__=="),
    ("!=", "__op__!="),
    ("===", "__op__==="),
    ("!==", "__op__!=="),
    ("<", "__op__<"),
    (">", "__op__>"),
    ("<=", "__op__<="),
    (">=", "__op__>="),
    ("<>", "__op__<>"),
    ("&&", "__op__&&"),
    ("||", "__op__||"),
    ("!", "__op__!"),
    ("and", "__op__and"),
    ("or", "__op__or"),
    ("not", "__op__not"),
    ("in", "__op__in"),
    ("is", "__op__is"),
    ("&", "__op__&"),
    ("|", "__op__|"),
    ("^", "__op__^"),
    ("~", "__op__~"),
    ("<<", "__op__<<"),
    (">>", "__op__>>"),
    (">>>", "__op__>>>"),
    ("+=", "__op__+="),
    ("-=", "__op__-="),
    ("*=", "__op__*="),
    ("/=", "__op__/="),
    ("%=", "__op__%="),
    ("**=", "__op__**="),
    ("//=", "__op__//="),
    ("&=", "__op__&="),
    ("|=", "__op__|="),
    ("^=", "__op__^="),
    ("<<=", "__op__<<="),
    (">>=", "__op__>>="),
    (">>>=", "__op__>>>="),
    ("&&=", "__op__&&="),
    ("||=", "__op__||="),
    ("??=", "__op__??="),
    ("??", "__op__??"),
    ("?.", "__op__?."),
    ("..", "__op__.."),
    ("..=", "__op__..="),
    ("...", "__op__..."),
];

/// True when an anonymous token changes what the code computes and must
/// therefore survive normalisation as an operator leaf.
#[must_use]
pub fn is_behaviour_bearing_token(token: &str) -> bool {
    operator_kind(token).is_some()
}

/// The normalised kind for one behaviour-bearing `token` —
/// [`OPERATOR_KIND_PREFIX`] followed by the token itself — or `None`
/// when the token is framing rather than behaviour.
#[must_use]
pub fn operator_kind(token: &str) -> Option<&'static str> {
    BEHAVIOUR_BEARING_TOKENS
        .iter()
        .find(|(candidate, _)| *candidate == token)
        .map(|(_, kind)| *kind)
}

/// True when a normalised kind is an operator leaf.
///
/// Matched against the table rather than by prefix alone, so the answer
/// is exact: no grammar kind can satisfy it by accident, and no
/// operator kind can be missed.
#[must_use]
pub fn is_operator_kind(kind: &str) -> bool {
    BEHAVIOUR_BEARING_TOKENS
        .iter()
        .any(|(_, candidate)| *candidate == kind)
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
    let children = normalise_children(root, FILE_KIND, file_id, normalise_kind, language, 1)?;
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
    let children = normalise_children(node, kind, file_id, normalise_kind, language, depth)?;
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
/// `parent_kind` is the kind `node` *normalised to*, never its raw
/// grammar kind. [`OPERATOR_FRAMING_PARENTS`] is read against it, so the
/// list can name a normalised kind such as [`LITERAL_KIND`] — every
/// language maps its own literal productions onto that one kind, and a
/// raw-kind list would have to enumerate `regex`, `string`,
/// `template_string` and their equivalent in every grammar to say the
/// same thing.
///
/// The two are collected in one pass over `children`, not two, so an
/// operator keeps its position between its operands. A frontier that
/// listed `alpha`, `beta`, `+` rather than `alpha`, `+`, `beta` would
/// still detect the change, but it would stop aligning positionally
/// with a member whose operand count differs.
fn normalise_children(
    node: Node<'_>,
    parent_kind: &'static str,
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
            operator_leaf(child, parent_kind, file_id)
        };
        if let Some(child_node) = normalised {
            if !is_literal_fragment(parent_kind, child_node.kind) {
                children.push(child_node);
            }
        }
    }
    Ok(children)
}

/// True when `child_kind` is a *fragment* of the literal it sits inside
/// rather than code embedded in it ([PIPELINE-NORMALIZE-AST]).
///
/// That section collapses every literal to one `__literal__`, but
/// tree-sitter models some literals as a wrapper over parts: `/[a-z]+/i`
/// is a `regex` holding a `regex_pattern` and a `regex_flags`, and each
/// part normalises to [`LITERAL_KIND`] in its own right. The one literal
/// therefore arrived as three leaves, and since normalisation erases the
/// text, *every* flagged regex in the corpus carried the same three-node
/// shape — enough for two regexes matching completely different text to
/// publish as duplication.
///
/// Only literal-inside-literal collapses. An expression interpolated into
/// a literal normalises to its own expression kind, never to
/// [`LITERAL_KIND`], so real code inside a template string survives.
fn is_literal_fragment(parent_kind: &str, child_kind: &str) -> bool {
    parent_kind == LITERAL_KIND && child_kind == LITERAL_KIND
}

/// Grammar productions in which a behaviour-bearing token is **framing**
/// and must not become a leaf ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// [`BEHAVIOUR_BEARING_TOKENS`] is keyed on token *text*, because
/// tree-sitter names an anonymous node by its own token, and text alone
/// cannot separate the `|` of `left | right` from the `|` that delimits a
/// Rust closure's binding list. Only the parent production tells them
/// apart, which is what this list is for.
///
/// Every entry must be justified by a test that fails without it, and the
/// list is deliberately as short as that rule allows. Suppressing a leaf
/// is not free: it erases a difference two members could otherwise
/// disagree on, and an entry that erases a *real* difference manufactures
/// a false positive. That is not hypothetical — `type_arguments` and
/// `type_parameters` were in this list and had to come out. They promoted
/// the seven sibling Dart API methods in `rank_structural_only_policy`
/// from `structural_only` to `nearly_identical`: a shape-only family
/// relabelled as near-duplicate code, which is the #134/#197 defect that
/// fixture exists to pin. Removing them cost nothing —
/// `rust_issue_147_iter_collect_idiom` still passes, and the committed AST
/// and report goldens stop drifting.
///
/// The direction of the list is deliberate. A production missing from an
/// *allowlist* of operator productions would drop its operator and let
/// `alpha + beta` and `alpha - beta` hash identically again — the defect
/// this whole section exists to fix. A production missing from this
/// denylist leaves a subtree larger than it should be. Both are worth
/// fixing; only one certifies an operator swap as duplication.
const OPERATOR_FRAMING_PARENTS: &[&str] = &[
    // Rust closure parameter lists: the pipes delimit the binding list,
    // they are not bitwise-or. Emitting them changed the hash and the LSH
    // bands of every closure-bearing function in gh #147, which fragmented
    // the single component `main` forms into sixteen and left
    // [CLONE-NOISE-RUST-ITER-COLLECT] with no cluster whose every member
    // holds the idiom. Pinned by `rust_issue_147_iter_collect_idiom`.
    "closure_parameters",
    // A delimiter *inside* a literal: a JavaScript regex's `/`. Without
    // this the literal stops being one leaf and becomes
    // `__op__/ __literal__ __op__/`, a three-node shape every regex in the
    // corpus shares — so two unrelated regex constants publish as
    // duplication. Pinned by `regex_literal_delimiters`.
    LITERAL_KIND,
];

/// True when `parent_kind` is a production in which a behaviour-bearing
/// token is framing — see [`OPERATOR_FRAMING_PARENTS`].
fn is_framing_parent(parent_kind: &str) -> bool {
    OPERATOR_FRAMING_PARENTS.contains(&parent_kind)
}

/// The operator leaf for one anonymous token, or `None` when the token
/// is framing rather than behaviour. Tree-sitter names an anonymous
/// node by its own token text, so the node kind *is* the operator — but
/// the same text is framing in some productions, which is why
/// `parent_kind` decides too ([`OPERATOR_FRAMING_PARENTS`]).
fn operator_leaf(token: Node<'_>, parent_kind: &str, file_id: FileId) -> Option<NormalizedNode> {
    if is_framing_parent(parent_kind) {
        return None;
    }
    operator_kind(token.kind()).map(|kind| NormalizedNode {
        kind,
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

#[cfg(test)]
mod tests {
    use super::{BEHAVIOUR_BEARING_TOKENS, OPERATOR_KIND_PREFIX};

    /// [PIPELINE-NORMALIZE-AST-OPERATOR] Each row's normalised kind must
    /// be exactly its own token behind the namespace prefix, and no two
    /// rows may name the same kind.
    ///
    /// The kinds are written out by hand so normalisation allocates
    /// nothing, and a hand-written table can pair `<<=` with `__op__<<`.
    /// Nothing downstream could ever notice: two different operators
    /// would simply share a leaf, `alpha <<= beta` and `alpha << beta`
    /// would hash identically, and the fingerprint would certify them as
    /// the same code — the exact defect this section removes, reinstated
    /// for one row by a typo.
    #[test]
    fn every_row_is_its_own_token_behind_the_prefix() {
        for (token, kind) in BEHAVIOUR_BEARING_TOKENS {
            assert_eq!(
                *kind,
                format!("{OPERATOR_KIND_PREFIX}{token}"),
                "operator `{token}` is normalised to `{kind}`, which is not \
                 its own token behind the prefix — the leaf cannot \
                 discriminate the operator it stands for"
            );
        }
        let mut kinds: Vec<&str> = BEHAVIOUR_BEARING_TOKENS
            .iter()
            .map(|(_, kind)| *kind)
            .collect();
        let total = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(
            kinds.len(),
            total,
            "two operators share a normalised kind, so a swap between them \
             leaves the subtree hashing identically"
        );
        let mut tokens: Vec<&str> = BEHAVIOUR_BEARING_TOKENS
            .iter()
            .map(|(token, _)| *token)
            .collect();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(
            tokens.len(),
            total,
            "a token appears twice in the table, so which kind it \
             normalises to depends on lookup order"
        );
    }
}
