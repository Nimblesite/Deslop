//! Cluster-level false-positive filters applied during report rendering.
//!
//! Noise patterns that survive structural fingerprinting because Type-2
//! normalisation collapses identifiers and literals. We re-parse each
//! cluster member's original source bytes and look at the *real* tree —
//! the same way a reviewer would — so we can drop clusters that the
//! pipeline cannot tell apart but a human instantly would.
//!
//! Issues addressed:
//! - **#69** — abstract method signatures cluster across every concrete
//!   subclass that implements an `ABC`. The 37-node subtree is the
//!   parameter list; identical-by-contract is not duplication.
//! - **#70** — `_tool_call_response("write_file", {...}, "id")` and
//!   peers cluster when only string literal args vary. Test fixture
//!   data, not duplication.
//! - **#71** — REST endpoint tests cluster on the HTTP-call shape but
//!   the f-string templates encode different routes. Different routes
//!   are different tests, not duplication.
//! - **#72** — `monkeypatch.setenv("KEY", "VAL")` test scaffolding
//!   clusters because every two-setenv test looks the same after
//!   normalisation. Scaffolding is not logic.
//!
//! The filter is purely additive: it never re-routes a `nearly_identical`
//! cluster as `identical`, only suppresses noise. Any cluster whose
//! member sources cannot be parsed (missing language plug-in, partial
//! source bytes) falls through unchanged.

use std::{collections::HashMap, hash::BuildHasher};

use tree_sitter::Node;

use crate::{ast::ByteRange, fingerprint::Fingerprint, lang::shared::parse_source, state::FileId};

/// Decides whether `cluster` is a known noise pattern that must not be
/// surfaced as duplication. Returns `true` when the cluster should be
/// hidden from the ranked report.
pub(crate) fn is_noise_pattern<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let Some(language) = uniform_language(members, file_languages) else {
        return false;
    };
    let Some(snippets) = collect_snippets(members, sources, language) else {
        return false;
    };
    is_polymorphic_signature_cluster(&snippets) || is_literal_variation_call_cluster(&snippets)
}

/// One re-parsed cluster member: language, raw bytes, the byte range
/// inside `source` that the fingerprint covered, and the originating
/// [`FileId`] so cross-file uniqueness checks do not depend on
/// pointer identity.
struct Snippet<'a> {
    language: &'static str,
    source: &'a [u8],
    range: ByteRange,
    file_id: FileId,
}

/// Returns a single language id when every member shares it.
fn uniform_language<S: BuildHasher>(
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

/// Collects `(language, source, range)` tuples for every member, returning
/// `None` if any member's source bytes are unavailable.
fn collect_snippets<'a>(
    members: &[Fingerprint],
    sources: &'a HashMap<FileId, Vec<u8>>,
    language: &'static str,
) -> Option<Vec<Snippet<'a>>> {
    members
        .iter()
        .map(|member| {
            sources.get(&member.file_id).map(|source| Snippet {
                language,
                source: source.as_slice(),
                range: member.byte_range,
                file_id: member.file_id,
            })
        })
        .collect()
}

/// Detects **issue #69**: every cluster member is a function definition
/// (signature or whole `def`) whose declared name is the same identifier
/// and the members span at least two distinct files. That is the
/// abstract/interface implementation pattern — the contract forces
/// identity, no extraction is possible. We additionally require that
/// the enclosing function bodies are not byte-equivalent so a genuine
/// copy-pasted helper that happens to share a name (e.g. private
/// `_helper` reused in two modules) still fires as a cluster.
fn is_polymorphic_signature_cluster(snippets: &[Snippet<'_>]) -> bool {
    let names: Option<Vec<&[u8]>> = snippets.iter().map(enclosing_function_name).collect();
    let Some(names) = names else { return false };
    let Some(first_name) = names.first() else {
        return false;
    };
    if !names.iter().all(|name| name == first_name) {
        return false;
    }
    let mut files = std::collections::BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    if files.len() < 2 {
        return false;
    }
    enclosing_function_bodies_differ(snippets)
}

/// Returns true when at least two cluster members' enclosing function
/// bodies differ in raw source bytes — distinguishes polymorphism
/// (different implementations of one signature) from genuinely
/// duplicated helper functions that share a name.
fn enclosing_function_bodies_differ(snippets: &[Snippet<'_>]) -> bool {
    let bodies: Option<Vec<Vec<u8>>> = snippets
        .iter()
        .map(|snippet| {
            let tree = parse_for(snippet)?;
            let function = enclosing_kind(
                tree.root_node(),
                snippet.range,
                function_kinds(snippet.language),
            )?;
            let body = function.child_by_field_name("body")?;
            snippet
                .source
                .get(body.start_byte()..body.end_byte())
                .map(<[u8]>::to_vec)
        })
        .collect();
    let Some(bodies) = bodies else { return false };
    let Some(first) = bodies.first() else {
        return false;
    };
    bodies.iter().any(|body| body != first)
}

/// Returns the name of the `function_definition` (or `method_declaration`
/// for C#) that contains `snippet.range`, when one exists.
fn enclosing_function_name<'a>(snippet: &'a Snippet<'_>) -> Option<&'a [u8]> {
    let tree = parse_for(snippet)?;
    let function = enclosing_kind(
        tree.root_node(),
        snippet.range,
        function_kinds(snippet.language),
    )?;
    let name_node = function.child_by_field_name("name")?;
    snippet
        .source
        .get(name_node.start_byte()..name_node.end_byte())
}

/// Returns the set of tree-sitter node kinds that count as function
/// declarations for the purpose of polymorphism detection.
const fn function_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["function_definition"],
        b"csharp" => &["method_declaration", "local_function_statement"],
        b"rust" => &["function_item"],
        _ => &[],
    }
}

/// Detects **issues #70 / #71 / #72**: every cluster member is an
/// expression (or expression statement) whose top-level call has the
/// same callee chain across the cluster but **at least one string
/// literal argument differs**. That is the test-data / scaffolding
/// shape — extraction would erase intentional variation.
fn is_literal_variation_call_cluster(snippets: &[Snippet<'_>]) -> bool {
    let calls: Option<Vec<CallShape>> = snippets.iter().map(call_shape).collect();
    let Some(calls) = calls else { return false };
    let Some(first) = calls.first() else {
        return false;
    };
    if !calls.iter().all(|call| call.callee == first.callee) {
        return false;
    }
    if !calls.iter().all(|call| call.arity == first.arity) {
        return false;
    }
    has_differing_string_literals(&calls)
}

/// Distilled view of a call expression used to compare cluster members.
struct CallShape {
    /// Concrete callee string (e.g. `"client.delete"`,
    /// `"monkeypatch.setenv"`). Captured from the raw source so it
    /// retains identifier text that the normalised AST collapses.
    callee: Vec<u8>,
    /// Number of arguments to the call.
    arity: usize,
    /// Per-argument summary used for literal-variation detection.
    arguments: Vec<ArgShape>,
}

/// Per-argument summary recorded for each call.
#[derive(Clone)]
enum ArgShape {
    /// Raw bytes of a string-literal argument (or string content of an
    /// f-string / interpolated string).
    StringLiteral(Vec<u8>),
    /// Anything else — non-string literal, identifier, sub-expression.
    /// We only compare string literals for variation detection.
    Other,
}

/// Extracts the [`CallShape`] for the call expression covering
/// `snippet.range`. Returns `None` when no call is present.
fn call_shape(snippet: &Snippet<'_>) -> Option<CallShape> {
    let tree = parse_for(snippet)?;
    let call = enclosing_kind(
        tree.root_node(),
        snippet.range,
        call_kinds(snippet.language),
    )?;
    let callee_node = call.child_by_field_name("function")?;
    let callee = snippet
        .source
        .get(callee_node.start_byte()..callee_node.end_byte())?
        .to_vec();
    let arguments = collect_argument_shapes(call, snippet.source);
    Some(CallShape {
        arity: arguments.len(),
        callee,
        arguments,
    })
}

/// Returns the set of tree-sitter node kinds that count as call
/// expressions per language.
const fn call_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["call"],
        b"csharp" => &["invocation_expression"],
        b"rust" => &["call_expression", "macro_invocation"],
        _ => &[],
    }
}

/// Walks the named children of the call's `arguments`/`argument_list`
/// node and produces one [`ArgShape`] per argument.
fn collect_argument_shapes(call: Node<'_>, source: &[u8]) -> Vec<ArgShape> {
    let Some(args) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    else {
        return Vec::new();
    };
    let mut shapes = Vec::new();
    let mut cursor = args.walk();
    for arg in args.named_children(&mut cursor) {
        shapes.push(arg_shape(arg, source));
    }
    shapes
}

/// Classifies one argument node into [`ArgShape`].
fn arg_shape(node: Node<'_>, source: &[u8]) -> ArgShape {
    let inner = unwrap_argument(node);
    if let Some(bytes) = string_literal_bytes(inner, source) {
        return ArgShape::StringLiteral(bytes);
    }
    ArgShape::Other
}

/// Strips a C# `argument` wrapper down to its inner expression so the
/// literal-detection match arms below see the same shapes regardless of
/// language.
fn unwrap_argument(node: Node<'_>) -> Node<'_> {
    if node.kind() == "argument" {
        let mut cursor = node.walk();
        let child = node.named_children(&mut cursor).next();
        if let Some(child) = child {
            return child;
        }
    }
    node
}

/// Returns the bytes of `node` when it is a string-literal-like leaf.
/// Covers Python plain `string`, f-string, and C# `string_literal` /
/// `interpolated_string_expression` so f-string template differences in
/// issue #71 are captured.
fn string_literal_bytes<'a>(node: Node<'_>, source: &'a [u8]) -> Option<Vec<u8>> {
    let kind = node.kind();
    let is_string = matches!(
        kind,
        "string"
            | "concatenated_string"
            | "string_literal"
            | "verbatim_string_literal"
            | "interpolated_string_expression"
    );
    if !is_string {
        return None;
    }
    source
        .get(node.start_byte()..node.end_byte())
        .map(<[u8]>::to_vec)
}

/// Returns true when at least one positional argument index has
/// differing string-literal bytes across the cluster. Non-string
/// arguments are ignored — the heuristic only fires when the
/// distinguishing variation is in literal text.
fn has_differing_string_literals(calls: &[CallShape]) -> bool {
    let Some(first) = calls.first() else {
        return false;
    };
    let mut saw_difference = false;
    let mut saw_string_arg = false;
    for index in 0..first.arguments.len() {
        let baseline = match first.arguments.get(index) {
            Some(ArgShape::StringLiteral(bytes)) => bytes,
            _ => continue,
        };
        saw_string_arg = true;
        for call in calls.iter().skip(1) {
            match call.arguments.get(index) {
                Some(ArgShape::StringLiteral(bytes)) if bytes != baseline => {
                    saw_difference = true;
                }
                Some(ArgShape::StringLiteral(_)) => {}
                _ => {
                    return false;
                }
            }
        }
    }
    saw_string_arg && saw_difference
}

/// Walks `root` looking for the smallest descendant of `kinds` whose
/// byte range encloses `range`.
fn enclosing_kind<'tree>(
    root: Node<'tree>,
    range: ByteRange,
    kinds: &[&str],
) -> Option<Node<'tree>> {
    let mut best: Option<Node<'tree>> = None;
    let mut stack: Vec<Node<'tree>> = vec![root];
    while let Some(node) = stack.pop() {
        if node.start_byte() > range.start || node.end_byte() < range.end {
            continue;
        }
        if kinds.contains(&node.kind()) {
            best = Some(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }
    best
}

/// Parses the snippet's full source so we can walk a real tree-sitter
/// CST instead of the normalised one. Returns `None` when the language
/// has no registered grammar here.
fn parse_for(snippet: &Snippet<'_>) -> Option<tree_sitter::Tree> {
    let language = grammar_for(snippet.language)?;
    parse_source(snippet.language, &language, snippet.source).ok()
}

/// Maps a language id to its tree-sitter grammar.
fn grammar_for(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "csharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        _ => None,
    }
}
