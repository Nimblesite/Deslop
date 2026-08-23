//! [CORPUS-PRECISION] The ranking rules the real-repository gate applies to
//! the head of a report.
//!
//! Ranking *is* the product: a false positive at rank 1 is the one a user
//! acts on. `must_not_rank_first` names the shapes a framework *mandates* —
//! Flutter requires every `StatefulWidget` to declare its own
//! `createState()` — which cannot be extracted or merged and so must never
//! outrank genuine copy-paste (gh #331).
//!
//! The rule is stated as an AST predicate, never as source text. A shape
//! matched by substring is unsound in both directions: it fires on a comment
//! or string literal that merely *mentions* the supertype, and it misses a
//! declaration whose `extends` clause is wrapped across lines or spaced
//! differently. Both directions are pinned in this module's tests.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use deslop_core::{lang::shared::parse_source, pipeline::default_parsers};
use serde_json::Value;

use crate::{
    corpus::{
        array, cluster_shows_span, field_u64, u64_field, visible_clusters, CorpusRun, Failure,
    },
    enclosure::{span_of, Span},
};

/// [CORPUS-PRECISION-CURATED] `precision` — code a human confirmed is
/// **not** duplicated must never be reported as one cluster.
///
/// Seven of this repository's open false-positive issues say the same
/// thing: *these are not duplicates and Deslop clustered them*. Until this
/// check existed no manifest field could express that, so not one of them
/// could be pinned on the repository it was reported against. `must_find`
/// is recall-only, and `must_not_rank_first` guards the head of the report
/// by base type, on one repository.
///
/// This is [CORPUS-RECALL]'s predicate read backwards: the same
/// "does a shown cluster span every curated path" question, with the
/// opposite verdict. Visibility works the same way and for the same reason
/// — a false positive nobody is shown is not a false positive — so a
/// cluster whose curated side is entirely hidden does not breach the entry.
///
/// An empty list asserts nothing, and an entry naming fewer than two files
/// fails rather than passing vacuously: one path cannot describe a pair the
/// engine wrongly joined.
pub fn check_curated_precision(manifest: &Value, report: &Value, failures: &mut Vec<Failure>) {
    for entry in array(manifest, "must_not_cluster") {
        check_one_curated_non_duplicate(entry, report, failures);
    }
}

/// Judges one curated `must_not_cluster` entry against the rendered report.
fn check_one_curated_non_duplicate(entry: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let files: Vec<String> = array(entry, "files")
        .iter()
        .filter_map(|file| file.as_str().map(ToOwned::to_owned))
        .collect();
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    if files.len() < 2 {
        failures.push(Failure::new(
            "precision",
            format!(
                "`must_not_cluster` entry names {} file(s); a non-duplication claim needs \
                 at least two paths or it asserts nothing. Curated: {why}",
                files.len()
            ),
        ));
        return;
    }
    let Some(breach) = visible_clusters(report)
        .into_iter()
        .find(|cluster| cluster_shows_span(cluster, &files))
    else {
        return;
    };
    failures.push(Failure::new(
        "precision",
        format!(
            "cluster {id} ({bucket}, {size} occurrences, fused {fused:.3}) is shown spanning \
             {files:?}, which a human verified is not duplication. Curated: {why}",
            id = breach
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("<unlabelled>"),
            bucket = breach
                .get("bucket")
                .and_then(Value::as_str)
                .unwrap_or("<unlabelled>"),
            size = field_u64(breach, "size"),
            fused = breach
                .pointer("/signals/fused")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
        ),
    ));
}

/// [CORPUS-PRECISION] Language- or framework-mandated scaffolding must never
/// outrank genuine copy-paste. Such a cluster is unactionable by
/// construction, so it must not sit at the head of a "worst offenders first"
/// report.
///
/// # Errors
///
/// Returns an error when a ranked occurrence cannot be read, or when the
/// manifest names a language this module carries no heritage grammar for —
/// an unsupported language must fail the gate loudly, never pass it silently.
pub fn check_boilerplate_not_ranked_first(
    manifest: &Value,
    root: &Path,
    run: &CorpusRun,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let Some(rule) = manifest.get("must_not_rank_first") else {
        return Ok(());
    };
    // Saturating up, never down: a `top_n` too large for the host widens the
    // check to every cluster, where narrowing it to zero would silently switch
    // the precision gate off.
    let top_n = usize::try_from(u64_field(rule, "top_n")?).unwrap_or(usize::MAX);
    let forbidden: Vec<&str> = array(rule, "forbidden_top_supertypes")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    for (rank, cluster) in array(&run.report, "clusters")
        .iter()
        .take(top_n)
        .enumerate()
    {
        for supertype in &forbidden {
            judge_cluster(root, cluster, rank, supertype, failures)?;
        }
    }
    Ok(())
}

/// Records a failure when the ranked cluster's first occurrence declares
/// `supertype` as a base type.
fn judge_cluster(
    root: &Path,
    cluster: &Value,
    rank: usize,
    supertype: &str,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let language = cluster
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("rank {rank}: cluster carries no language"))?;
    let (source, span) = first_occurrence_source(root, cluster)?;
    if !declares_forbidden_supertype(language, &source, &span, supertype)? {
        return Ok(());
    }
    failures.push(Failure::new(
        "boilerplate_rank",
        format!(
            "rank {rank}: cluster of {} occurrences declares `{supertype}`, a \
             framework-mandated base type that cannot be deduplicated. First \
             occurrence: {}:{}",
            field_u64(cluster, "size"),
            span.path,
            span.start,
        ),
    ));
    Ok(())
}

/// The whole source of the file the cluster's first occurrence lives in,
/// alongside that occurrence's span.
///
/// The *file* is read, not the occurrence slice, because a slice of a
/// declaration does not parse into the declaration it came from: the
/// heritage clause the rule is about may sit outside the reported range.
fn first_occurrence_source(scan_root: &Path, cluster: &Value) -> Result<(String, Span)> {
    let occurrence = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .and_then(|occurrences| occurrences.first())
        .ok_or_else(|| anyhow!("cluster has no occurrences"))?;
    let span = span_of(occurrence).ok_or_else(|| anyhow!("occurrence has no span"))?;
    let source = std::fs::read_to_string(scan_root.join(&span.path))
        .with_context(|| format!("occurrence source unreadable: {}", span.path))?;
    Ok((source, span))
}

/// Per-language heritage grammar: the declaration kinds that can carry base
/// types, and the direct-child clause kinds that hold them.
///
/// Every pair here was read off the grammar's own `node-types.json` and is
/// exercised by this module's tests, so nothing is asserted about a grammar
/// that was not checked. Languages without a base-type clause — Rust, Go,
/// F# — are deliberately absent, and a manifest that names one fails the
/// gate rather than passing it.
struct Heritage {
    /// Node kinds that introduce a type which may declare base types.
    declarations: &'static [&'static str],
    /// Kinds of the direct named child holding those base types.
    clauses: &'static [&'static str],
    /// How the clause's children divide into base types and type arguments.
    base_types: BaseTypes,
}

/// Which children of a heritage clause name a base type.
///
/// Grammars disagree, and the disagreement is not cosmetic: reading it wrong
/// makes one manifest entry condemn every generic subclass in a repository.
enum BaseTypes {
    /// Every named child names a base type, and the grammar gives type
    /// arguments their own node — C#'s `type_argument_list`, TypeScript's
    /// `type_arguments`, Python's `subscript` field.
    EveryChild,
    /// Only the *first* named child names a base type, because the grammar
    /// flattens the type arguments into the siblings that follow it. Dart
    /// renders `extends State<LedgerView>` as `superclass type: (type
    /// (type_identifier)) type: (type (type (type_identifier)))` — two
    /// `type` fields, no `type_arguments` node anywhere.
    FirstChildOnly,
}

/// The heritage grammar of every language a `must_not_rank_first` rule may
/// name.
const HERITAGE: &[(&str, Heritage)] = &[
    (
        "dart",
        Heritage {
            declarations: &["class_declaration"],
            clauses: &["superclass"],
            base_types: BaseTypes::FirstChildOnly,
        },
    ),
    (
        "csharp",
        Heritage {
            declarations: &[
                "class_declaration",
                "record_declaration",
                "struct_declaration",
                "interface_declaration",
            ],
            clauses: &["base_list"],
            base_types: BaseTypes::EveryChild,
        },
    ),
    (
        "javascript",
        Heritage {
            declarations: &["class_declaration", "class"],
            clauses: &["class_heritage"],
            base_types: BaseTypes::EveryChild,
        },
    ),
    (
        "typescript",
        Heritage {
            declarations: &["class_declaration", "class"],
            clauses: &["class_heritage"],
            base_types: BaseTypes::EveryChild,
        },
    ),
    (
        "tsx",
        Heritage {
            declarations: &["class_declaration", "class"],
            clauses: &["class_heritage"],
            base_types: BaseTypes::EveryChild,
        },
    ),
    (
        "python",
        Heritage {
            declarations: &["class_definition"],
            // `class_definition`'s `superclasses` field; it is the only
            // `argument_list` a class header can hold.
            clauses: &["argument_list"],
            base_types: BaseTypes::EveryChild,
        },
    ),
    (
        "php",
        Heritage {
            declarations: &["class_declaration", "interface_declaration"],
            clauses: &["base_clause"],
            base_types: BaseTypes::EveryChild,
        },
    ),
];

/// Subtrees inside a heritage clause that name type *arguments*, not base
/// types. `extends State<LedgerView>` declares `State`; `LedgerView` is what
/// it was instantiated with.
const TYPE_ARGUMENT_KINDS: &[&str] = &["type_arguments", "type_argument_list", "type_parameters"];

/// Fields whose value names a type argument rather than a base type. Python
/// has no type-argument node kind — it spells `Generic[T]` as a `subscript`
/// whose `subscript` field is the argument — so the exclusion has to be read
/// off the field name there.
const TYPE_ARGUMENT_FIELDS: &[&str] = &["subscript"];

/// True when a type declaration overlapping `span` in `source` names
/// `supertype` among its base types.
///
/// Both the declaration *containing* the span and any declaration the span
/// contains count: a ranked occurrence is usually the framework-mandated
/// member (Flutter's `createState`), not the class header that makes it
/// mandated.
///
/// This replaces `occurrence_text.contains("extends <supertype>")` (gh
/// #401), which was wrong in both directions at once — it fired on a
/// comment, doc comment or string literal that merely mentioned the
/// supertype, and it missed a declaration whose clause was wrapped across
/// lines. Both directions are pinned by this module's tests. The deleted
/// arm was also a straight `CLAUDE.md` violation: no pattern matching on
/// source text, use the AST.
///
/// # Errors
///
/// Returns an error when `span` is outside `source`, when `language` has no
/// registered parser, when it has no heritage grammar here, or when the
/// parse fails.
pub fn declares_forbidden_supertype(
    language: &str,
    source: &str,
    span: &Span,
    supertype: &str,
) -> Result<bool> {
    let heritage = HERITAGE
        .iter()
        .find(|(id, _)| *id == language)
        .map(|(_, heritage)| heritage)
        .ok_or_else(|| {
            anyhow!(
                "`must_not_rank_first` names {supertype} for language `{language}`, which \
                 carries no heritage grammar here — curate one rather than letting the \
                 precision gate pass without judging anything"
            )
        })?;
    let grammar = grammar_for(language)?;
    let tree = parse_source(language_id(language)?, &grammar, source.as_bytes())?;
    let bytes = source.as_bytes();
    let start = usize::try_from(span.start)?;
    let end = usize::try_from(span.end)?;
    Ok(declarations_overlapping(&tree, heritage, start..end)
        .iter()
        .any(|node| names_supertype(*node, heritage, bytes, supertype)))
}

/// Every declaration node whose byte range overlaps `range`.
fn declarations_overlapping<'tree>(
    tree: &'tree tree_sitter::Tree,
    heritage: &Heritage,
    range: std::ops::Range<usize>,
) -> Vec<tree_sitter::Node<'tree>> {
    let mut cursor = tree.walk();
    let mut pending = vec![tree.root_node()];
    let mut found = Vec::new();
    while let Some(node) = pending.pop() {
        if node.start_byte() >= range.end || node.end_byte() <= range.start {
            continue;
        }
        if heritage.declarations.contains(&node.kind()) {
            found.push(node);
        }
        pending.extend(node.named_children(&mut cursor));
    }
    found
}

/// True when `declaration`'s heritage clause names `supertype`.
fn names_supertype(
    declaration: tree_sitter::Node<'_>,
    heritage: &Heritage,
    source: &[u8],
    supertype: &str,
) -> bool {
    let mut cursor = declaration.walk();
    let clauses: Vec<tree_sitter::Node<'_>> = declaration
        .named_children(&mut cursor)
        .filter(|child| heritage.clauses.contains(&child.kind()))
        .collect();
    clauses
        .into_iter()
        .flat_map(|clause| base_type_roots(clause, heritage))
        .any(|root| names_type(root, source, supertype))
}

/// The subtrees of `clause` that name base types, per the language's
/// [`BaseTypes`] convention.
fn base_type_roots<'tree>(
    clause: tree_sitter::Node<'tree>,
    heritage: &Heritage,
) -> Vec<tree_sitter::Node<'tree>> {
    match heritage.base_types {
        BaseTypes::EveryChild => vec![clause],
        BaseTypes::FirstChildOnly => base_type_children(clause).into_iter().take(1).collect(),
    }
}

/// True when a type-name leaf of `root` reads exactly `supertype`.
///
/// Only leaves count, so `extends Framework.Widget` names `Widget`, and
/// type-argument subtrees are dropped on the way down.
fn names_type(root: tree_sitter::Node<'_>, source: &[u8], supertype: &str) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        let children = base_type_children(node);
        if children.is_empty() {
            if node.utf8_text(source).is_ok_and(|text| text == supertype) {
                return true;
            }
            continue;
        }
        pending.extend(children);
    }
    false
}

/// Named children of `node` that can carry a base-type name, in document
/// order. Type-argument nodes and type-argument fields are dropped.
fn base_type_children(node: tree_sitter::Node<'_>) -> Vec<tree_sitter::Node<'_>> {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return Vec::new();
    }
    let mut children = Vec::new();
    loop {
        let child = cursor.node();
        let is_type_argument = TYPE_ARGUMENT_KINDS.contains(&child.kind())
            || cursor
                .field_name()
                .is_some_and(|field| TYPE_ARGUMENT_FIELDS.contains(&field));
        if child.is_named() && !is_type_argument {
            children.push(child);
        }
        if !cursor.goto_next_sibling() {
            return children;
        }
    }
}

/// The tree-sitter grammar registered for `language`.
fn grammar_for(language: &str) -> Result<tree_sitter::Language> {
    default_parsers()
        .iter()
        .find(|parser| parser.id() == language)
        .map(|parser| parser.grammar())
        .ok_or_else(|| anyhow!("no registered parser for language `{language}`"))
}

/// The engine's `'static` id for `language`, which `parse_source` needs for
/// its error reporting.
fn language_id(language: &str) -> Result<&'static str> {
    default_parsers()
        .iter()
        .find(|parser| parser.id() == language)
        .map(|parser| parser.id())
        .ok_or_else(|| anyhow!("no registered parser for language `{language}`"))
}

#[cfg(test)]
mod tests;
