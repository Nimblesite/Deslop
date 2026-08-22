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

/// The tree-sitter field names the supported grammars give the token that
/// decides what a production computes
/// ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// `binary_expression`, `unary_expression`, `update_expression`,
/// `augmented_assignment` and their per-grammar spellings all name their
/// operator `operator`. Python's `comparison_operator` is the one
/// exception and names it **`operators`**, plural, because the production
/// is variadic — `low < mid < high` is one node carrying two operators.
/// Both are read: accepting only the singular silently dropped every
/// Python comparison, so `alpha < beta` and `alpha > beta` normalised to
/// the same subtree, which is precisely the defect this section exists to
/// remove.
///
/// Framing punctuation — brackets, commas, semicolons, tag delimiters,
/// generic angle brackets, closure pipes — carries no field name at all,
/// and a declaration keyword carries an unrelated one (`let` and `const`
/// are the `kind` field of `lexical_declaration`, not operators).
/// `tests::the_grammars_mark_operators_and_only_operators` measures every
/// half of that against the pinned grammars.
pub const OPERATOR_FIELDS: [&str; 2] = ["operator", "operators"];

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
/// A zero-allocation fast path for the common spellings, not the set of
/// operators — the grammar decides that ([`operator_field_leaf`]). The
/// kind is written out rather than built from [`OPERATOR_KIND_PREFIX`] at
/// runtime so the operators seen on nearly every line cost no allocation.
/// A spelling missing from this table is interned, never dropped.
/// `tests::every_row_is_its_own_token`
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
///
/// Kept for callers that classify a bare token with no tree to consult.
/// Normalisation does not use it: a token's spelling cannot say whether
/// it is an operator, only the production it sits in can, which is what
/// [`operator_field_leaf`] reads.
#[must_use]
pub fn operator_kind(token: &str) -> Option<&'static str> {
    BEHAVIOUR_BEARING_TOKENS
        .iter()
        .find(|(candidate, _)| *candidate == token)
        .map(|(_, kind)| *kind)
}

/// The normalised kind for a token the grammar has already identified as
/// an operator, for *any* spelling.
///
/// [`BEHAVIOUR_BEARING_TOKENS`] is consulted first so the common
/// operators cost a table scan and no allocation. A spelling the table
/// does not carry is interned instead of dropped, which is what makes the
/// rule fail *closed*: before this, `operator_kind` returned `None` for
/// every operator missing from the table and normalisation silently
/// erased it, so `value++` and `value--` — and `delete v`, `typeof v`,
/// `void v` — hashed identically. An operator the table has never seen
/// now still reaches the digest as its own leaf.
fn operator_kind_for_field(token: &str) -> &'static str {
    operator_kind(token).unwrap_or_else(|| intern_kind(&format!("{OPERATOR_KIND_PREFIX}{token}")))
}

/// True when a normalised kind is an operator leaf.
///
/// Matched by [`OPERATOR_KIND_PREFIX`] rather than against
/// [`BEHAVIOUR_BEARING_TOKENS`]. The table is a zero-allocation fast path,
/// not the set of operators: the grammar decides what an operator is
/// ([`operator_field_leaf`]), so an operator the table has never seen
/// still normalises to a prefixed kind and must still answer `true` here.
/// The prefix is disjoint from every grammar kind, so nothing can satisfy
/// this by accident.
#[must_use]
pub fn is_operator_kind(kind: &str) -> bool {
    kind.starts_with(OPERATOR_KIND_PREFIX)
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
/// [`normalise_node`], anonymous ones through [`operator_field_leaf`]
/// ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// `parent_kind` is the kind `node` *normalised to*, never its raw
/// grammar kind. [`operator_field_leaf`] reads it to recognise a literal
/// in any language at once, rather than enumerating `regex`, `string`,
/// `template_string` and their equivalent in every grammar.
///
/// A literal's *named* parts are deliberately kept. Collapsing them into
/// the parent looks tidy and is an accuracy defect: [FUSION-CONTENT-GATE]
/// reads the frontier's literal leaves as content evidence, so erasing
/// them erases the only thing separating "same shape, different content"
/// from "same code". Measured on `js-async`, where two functions calling
/// different endpoints were promoted from `structural_only` to
/// `nearly_identical` at `token_jaccard = 1.00` once their route literals
/// stopped reaching the frontier. What must not survive is the framing
/// *punctuation* between those parts, which `operator_field_leaf` drops.
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
    for (index, child) in node.children(&mut cursor).enumerate() {
        let normalised = if child.is_named() {
            normalise_node(
                child,
                file_id,
                normalise_kind,
                language,
                depth.saturating_add(1),
            )?
        } else {
            operator_field_leaf(node, parent_kind, child, index, file_id)
        };
        if let Some(child_node) = normalised {
            children.push(child_node);
        }
    }
    Ok(children)
}

/// The operator leaf for one anonymous token, or `None` when the token is
/// framing rather than behaviour ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// **The grammar decides the role; the token is only a label.**
/// Tree-sitter names an anonymous node by its own token text, so the same
/// bytes appear in both roles: the `<` of `alpha < beta` compares, the `<`
/// of `Vec<T>` opens a type argument list; the `|` of `left | right` is
/// bitwise-or, the `|` of `|x| x + 1` delimits a closure's bindings; the
/// `*` of `total * rate` multiplies, the `*` of `p *int` is a pointer.
/// Text cannot separate them and is never asked to. Every decision here
/// reads AST metadata — the field name the grammar assigned the child, and
/// the parent's production kind. `token.kind()` is carried through to the
/// leaf so two operators stay distinguishable; it never selects the role.
///
/// That answers the case a spelling table cannot: an operator whose
/// spelling the table has never seen is still kept, so `value++` and
/// `value--`, and `delete v` / `typeof v` / `void v`, stop normalising
/// into one another.
///
/// **Nothing inside a literal is behaviour.** A literal's delimiters
/// cannot distinguish anything once its text collapses:
/// `/[a-z]+@[a-z]+/i` and `/[0-9]{3}-[0-9]{4}/g` match completely
/// different text, and the slashes are all that would be left to compare.
/// Emitting them made a regex subtree clear the `--min-nodes` floor and
/// two unrelated regex constants published at `duplication_percent:
/// 40.0`. The check is on the parent's *normalised* kind, so it holds for
/// every grammar's literal productions at once rather than enumerating
/// `regex`, `string` and `template_string` in each.
///
/// **Where the grammar left no field**, the production decides
/// ([`UNFIELDED_OPERATOR_PRODUCTIONS`]).
fn operator_field_leaf(
    parent: Node<'_>,
    parent_normalised_kind: &str,
    token: Node<'_>,
    index: usize,
    file_id: FileId,
) -> Option<NormalizedNode> {
    if parent_normalised_kind == LITERAL_KIND {
        return None;
    }
    let field = u32::try_from(index)
        .ok()
        .and_then(|position| parent.field_name_for_child(position));
    // A field the grammar gave another meaning — `let` and `const` are the
    // `kind` of a `lexical_declaration` — is never an operator, so an
    // unrecognised field name suppresses the leaf exactly as framing does.
    let is_operator = match field {
        Some(name) => OPERATOR_FIELDS.contains(&name),
        None => is_unfielded_operator(parent.kind()),
    };
    if !is_operator {
        return None;
    }
    Some(NormalizedNode {
        kind: operator_kind_for_field(token.kind()),
        children: Vec::new(),
        byte_range: ByteRange {
            start: token.start_byte(),
            end: token.end_byte(),
        },
        file_id,
    })
}

/// Productions that carry an operator the grammar forgot to field-name
/// ([PIPELINE-NORMALIZE-AST-OPERATOR]).
///
/// Every grammar the engine ships names the token that decides what a
/// production computes ([`OPERATOR_FIELDS`]) — except `tree-sitter-rust`'s
/// `unary_expression`, which spells `-x`, `!x` and `*x` as a bare
/// alternation with no field. Without this row all three collapse into
/// one subtree and negation, dereference and arithmetic negation certify
/// as the same code.
///
/// **This is a list of productions, never of token text.** A production
/// kind is AST metadata the grammar assigns; a token's spelling is not,
/// and the earlier attempt to decide the residual case by spelling was
/// exactly the axis this section exists to abolish — `is_unfielded_operator`
/// asked `operator_kind(token)`, which is a literal-text match against
/// [`BEHAVIOUR_BEARING_TOKENS`], to classify a role. It cannot: the `|`
/// of `left | right` and the `|` of `|x| x + 1` are the same bytes.
///
/// Inverting the list from a framing *denylist* to this operator
/// *allowlist* also removes the per-grammar chase. A denylist had to name
/// `type_arguments`, `reference_type`, `pointer_type`,
/// `variadic_parameter_declaration`, `macro_invocation`,
/// `closure_parameters` and the JSX tags, and would have had to keep
/// naming the next one; anything it had not thought of kept inflating
/// silently. Here an unfielded token in any production not named below is
/// dropped, which is the safe direction: framing that survives
/// manufactures false positives, and the operators are proven present by
/// `tests::the_grammars_mark_operators_and_only_operators` across all
/// eight grammars.
const UNFIELDED_OPERATOR_PRODUCTIONS: &[&str] = &[
    // `tree-sitter-rust`: `-x`, `!x` and `*x` are a bare alternation with
    // no field, so without this row negation, dereference and arithmetic
    // negation all collapse into one subtree.
    "unary_expression",
    // `tree-sitter-python`: `not x` is its own production and leaves the
    // keyword unfielded. Dropping it makes `not ready` and `ready` the
    // same shape — an inverted condition certified as duplication.
    "not_operator",
    // `tree-sitter-c-sharp`: prefix `-x`, `!x`, `~x` and `&x` share one
    // unfielded production, so all four collapse together without this.
    "prefix_unary_expression",
    // `tree-sitter-dart` fields only `assignment_expression`. Every other
    // operator sits in a per-precedence production that names nothing, so
    // each level needs its own row or the operators at that level collapse
    // into one another — `a + b` and `a - b`, `a < b` and `a >= b`, `a &&
    // b` and `a || b`. Read off the grammar by the contract test below,
    // not guessed: a name that does not exist is a silently dead row.
    "additive_expression",
    "multiplicative_expression",
    "shift_expression",
    "relational_operator",
    "equality_expression",
    "logical_and_expression",
    "logical_or_expression",
    "bitwise_and_expression",
    "bitwise_or_expression",
    "bitwise_xor_expression",
    "if_null_expression",
    "prefix_operator",
    "postfix_expression",
    "negate_operator",
];

/// True when the grammar left this production's operator unfielded, so
/// the anonymous token in it is behaviour rather than framing.
///
/// Reads the parent production kind alone. The token is a label carried
/// through to the leaf; it never decides the role.
#[must_use]
pub fn is_unfielded_operator(parent_kind: &str) -> bool {
    UNFIELDED_OPERATOR_PRODUCTIONS.contains(&parent_kind)
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
    use tree_sitter::{Node, Parser};

    use super::{
        is_unfielded_operator, BEHAVIOUR_BEARING_TOKENS, OPERATOR_FIELDS, OPERATOR_KIND_PREFIX,
    };

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

    /// [PIPELINE-NORMALIZE-AST-OPERATOR] The contract normalisation rests
    /// on, measured against all eight pinned grammars rather than assumed:
    /// the token that decides what a production computes carries one of
    /// [`OPERATOR_FIELDS`], and framing punctuation carries none of them.
    ///
    /// This is the completeness check a spelling table could never be. The
    /// old table enumerated operators by hand and silently dropped the
    /// ones it missed — `++`/`--` under `update_expression` and
    /// `delete`/`typeof`/`void` under `unary_expression` were all absent,
    /// so `value++` and `value--` normalised to the same subtree. Reading
    /// the grammar instead is only safe if the grammars agree, and they
    /// very nearly do not: Python alone spells the field `operators`.
    /// Every row below is a spelling that spelling-only or denylist
    /// classification got wrong, so a grammar bump that moves any of them
    /// off the field trips here rather than silently in a golden.
    #[test]
    fn the_grammars_mark_operators_and_only_operators() {
        for (name, language, source, behaviour, framing) in operator_field_cases() {
            let mut parser = Parser::new();
            assert!(
                parser.set_language(&language).is_ok(),
                "{name}: the pinned grammar must bind — this table is the only \
                 proof the classifier matches the grammars actually shipped"
            );
            let parsed = parser.parse(source.as_bytes(), None);
            assert!(
                parsed.is_some(),
                "{name}: the probe source must parse, or the rows below are \
                 measured against nothing"
            );
            let Some(tree) = parsed else { continue };
            let fields = anonymous_token_fields(tree.root_node());
            let is_operator = |token: &str| {
                fields.iter().any(|(parent, spelling, field)| {
                    spelling == token
                        && match field.as_deref() {
                            Some(name) => OPERATOR_FIELDS.contains(&name),
                            None => is_unfielded_operator(parent),
                        }
                })
            };
            for token in behaviour {
                assert!(
                    is_operator(token),
                    "{name}: `{token}` decides what its production computes, so it \
                     must survive normalisation — dropped, two different \
                     computations hash alike: {fields:?}"
                );
            }
            for token in framing {
                assert!(
                    !is_operator(token),
                    "{name}: `{token}` is framing — the production already says what \
                     it means. Emitting it inflates every subtree with a position no \
                     two members can disagree on: {fields:?}"
                );
            }
        }
    }

    /// One grammar's row: its id, the pinned grammar, a source exercising
    /// its operators, the spellings that must survive normalisation, and
    /// the framing tokens that must not.
    type OperatorFieldCase = (
        &'static str,
        tree_sitter::Language,
        &'static str,
        Vec<&'static str>,
        Vec<&'static str>,
    );

    /// `(language, source, behaviour spellings, framing spellings)` for
    /// every grammar the engine ships.
    ///
    /// Behaviour entries are tokens that must never collapse into one
    /// another; framing entries are the exact tokens whose emission
    /// inflated the committed goldens.
    fn operator_field_cases() -> Vec<OperatorFieldCase> {
        vec![
            (
                "javascript",
                tree_sitter_javascript::LANGUAGE.into(),
                "let a = b + c; a++; a--; a -= 2; delete o.k; typeof v; void v; \
                 const t = x < y && p ?? q; const j = <Tag attr={v}>hi</Tag>;",
                vec![
                    "+", "++", "--", "-=", "delete", "typeof", "void", "<", "&&", "??",
                ],
                vec![";", ".", "{", "}", "</", ">", "let", "const"],
            ),
            (
                "typescript",
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                "const a: number = b + c; let d = e < f && g !== h; d ||= i; j++; \
                 const k: Array<number> = [];",
                vec!["+", "<", "&&", "!==", "||=", "++"],
                vec![":", ";", ",", "[", "]", "const", "let"],
            ),
            (
                "rust",
                tree_sitter_rust::LANGUAGE.into(),
                "fn f<T>(v: &Vec<T>) -> usize { let g = |x: usize| x + 1; \
                 println!(\"{}\", g(v.len())); if v.len() < 2 { v.len() * 2 } else { !0 } }",
                vec!["+", "*", "<", "!"],
                vec![">", "&", "|", "->", ":", ";", ".", "{", "}"],
            ),
            (
                "go",
                tree_sitter_go::LANGUAGE.into(),
                "package m\nfunc f(p *int, xs ...int) int { c := *p + len(xs); c -= 1; \
                 if c < 2 && c != 0 { c *= 2 }; return -c }\n",
                vec!["+", "-=", "<", "&&", "!=", "*=", "-"],
                vec!["...", "(", ")", ",", "{", "}", ":="],
            ),
            (
                "python",
                tree_sitter_python::LANGUAGE.into(),
                "def f(a, b):\n    c = a + b\n    c -= 1\n    \
                 if a < b and a != b and a is not b and a in [b]:\n        c *= 2\n    \
                 return not c\n",
                vec!["+", "-=", "<", "and", "!=", "is not", "in", "*=", "not"],
                vec![":", ",", "(", ")", "[", "]", "="],
            ),
            (
                "csharp",
                tree_sitter_c_sharp::LANGUAGE.into(),
                "class K { int F(int a, int b) { var c = a + b; c -= 1; \
                 if (a < b && a != b) c *= 2; return -c; } }",
                vec!["+", "-=", "<", "&&", "!=", "*=", "-"],
                vec![";", ",", "(", ")", "{", "}"],
            ),
            (
                "php",
                tree_sitter_php::LANGUAGE_PHP.into(),
                "<?php function f($a, $b) { $c = $a + $b; $c -= 1; \
                 if ($a < $b && $a !== $b) { $c *= 2; } return !$c; }",
                vec!["+", "-=", "<", "&&", "!==", "*=", "!"],
                vec![";", ",", "(", ")", "{", "}"],
            ),
            (
                "dart",
                tree_sitter_dart::LANGUAGE.into(),
                "int f(int a, int b) { var c = a + b - 1; c *= 2; c ~/= 3; \
                 var d = a % b; var e = a << 2 | b >> 1 & 3 ^ 7; \
                 var g = a / b; var h = a ?? b; \
                 if (a < b && a != b || a >= b) { c = ~c; } \
                 var i = !(a > b); c++; c--; return -c; }",
                vec![
                    "+", "-", "*=", "~/=", "%", "<<", "|", ">>", "&", "^", "/", "??", "<", "&&",
                    "!=", "||", ">=", "~", "!", ">", "++", "--",
                ],
                vec![";", ",", "(", ")", "{", "}", "var"],
            ),
        ]
    }

    /// Every anonymous token in the tree as
    /// `(parent kind, spelling, field name)`.
    fn anonymous_token_fields(root: Node<'_>) -> Vec<(String, String, Option<String>)> {
        let mut found = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let mut cursor = node.walk();
            for (index, child) in node.children(&mut cursor).enumerate() {
                if !child.is_named() {
                    let field = u32::try_from(index)
                        .ok()
                        .and_then(|position| node.field_name_for_child(position))
                        .map(str::to_owned);
                    found.push((node.kind().to_owned(), child.kind().to_owned(), field));
                }
                stack.push(child);
            }
        }
        found
    }
}
