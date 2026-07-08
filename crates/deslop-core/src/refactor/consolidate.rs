//! Cross-file identical-definition consolidation
//! ([AUTOFIX-CONSOLIDATE]).
//!
//! When a cluster's occurrences are whole top-level definitions that
//! are byte-equivalent across two or more files, keep the first
//! (canonical) copy, delete the duplicates, and point every duplicate
//! file at the canonical symbol with an import. The v1 resolver is
//! deliberately narrow and refusal-biased ([AUTOFIX-CONSOLIDATE-GATE]):
//! same-directory sibling modules, an already-visible canonical
//! definition, and no same-name collisions. A duplicate file that
//! would become empty refuses — deleting the file needs the module
//! declaration rewritten, which waits for the full resolver
//! ([AUTOFIX-CONSOLIDATE-EDIT] `DeleteFile` follow-up).
//!
//! v1.1 (issues #277/#279): an occurrence may cover a contiguous run
//! of whole definitions (each consolidated per symbol), and the
//! [`binding_drift`] gate refuses definitions whose free references
//! would re-bind after the move.

mod binding_drift;
mod sites;

use std::{collections::HashMap, path::PathBuf};

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    lang::{shared::parse_source, LanguageParser},
    refactor::{
        preconditions::{named_children, node_text, raw_slices_equivalent},
        RefactorError,
    },
    report::ReportCluster,
};

/// One planned edit against a specific file — consolidation spans
/// files, unlike the single-document extract/merge plans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFileEdit {
    /// File the edit applies to (workspace-relative, as reported).
    pub path: PathBuf,
    /// Inclusive start of the replaced span.
    pub start_byte: usize,
    /// Exclusive end of the replaced span.
    pub end_byte: usize,
    /// Replacement text (empty for deletions).
    pub new_text: String,
}

/// A mechanical consolidation: the duplicate definitions deleted and
/// every duplicate file re-pointed at the canonical symbols.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsolidatePlan {
    /// Symbols being consolidated, in definition order — one for the
    /// classic single-definition cluster, several for a definition run
    /// ([AUTOFIX-CONSOLIDATE-GATE] v1.1).
    pub symbols: Vec<String>,
    /// Path of the canonical definitions (kept untouched).
    pub canonical_path: PathBuf,
    /// Edits grouped per file, descending start order within a file.
    pub edits: Vec<PlannedFileEdit>,
}

/// The consolidation verdict ([AUTOFIX-CONSOLIDATE-GATE]).
#[derive(Debug)]
pub enum ConsolidationOutcome {
    /// All gates passed; the plan rewrites every dependent atomically.
    Mechanical(ConsolidatePlan),
    /// A gate failed or was undecidable; the reason routes the cluster
    /// to AI / human review.
    Refused(String),
}

/// One duplicate definition site resolved against its file.
#[derive(Clone)]
struct DefinitionSite {
    /// Workspace-relative file path as reported.
    path: PathBuf,
    /// Span of the whole definition including its outer attributes and
    /// doc comments — deleted and byte-equivalence-proven as one unit.
    span: ByteRange,
    /// Span of the bare item node, for reference analysis.
    item_span: ByteRange,
    /// Definition name text.
    name: String,
    /// Whether the definition carries a visibility marker.
    visible: bool,
}

/// Computes the consolidation for a cross-file identical-definition
/// cluster. `sources` maps each occurrence path (as reported) to its
/// file bytes.
///
/// # Errors
///
/// Returns [`RefactorError::Core`] when an occurrence file fails to
/// parse; every gate failure is a [`ConsolidationOutcome::Refused`].
pub fn compute_consolidation_plan<S: ::std::hash::BuildHasher>(
    cluster: &ReportCluster,
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<ConsolidationOutcome, RefactorError> {
    if parser.id() != "rust" {
        return Ok(ConsolidationOutcome::Refused(format!(
            "{} consolidation is not mechanical yet (v1 covers Rust sibling modules)",
            parser.id()
        )));
    }
    match sites::occurrence_sites(cluster, sources, parser)? {
        Err(reason) => Ok(ConsolidationOutcome::Refused(reason)),
        Ok(per_occurrence) => Ok(build_plan(&per_occurrence, sources, parser)?.map_or_else(
            ConsolidationOutcome::Refused,
            ConsolidationOutcome::Mechanical,
        )),
    }
}

/// Applies the remaining gates and assembles the edits. The inner
/// `Err` is the refusal reason.
fn build_plan<S: ::std::hash::BuildHasher>(
    per_occurrence: &[Vec<DefinitionSite>],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<ConsolidatePlan, String>, RefactorError> {
    let groups = match sites::symbol_groups(per_occurrence) {
        Ok(groups) => groups,
        Err(reason) => return Ok(Err(reason)),
    };
    for group in &groups {
        let Some((canonical, duplicates)) = group.split_first() else {
            return Ok(Err("no definition sites".to_owned()));
        };
        if let Err(reason) = consolidation_gate(canonical, duplicates, sources) {
            return Ok(Err(reason));
        }
    }
    if let Err(reason) = binding_drift::gate(&groups, sources, parser)? {
        return Ok(Err(reason));
    }
    assemble_edits(&groups, sources, parser)
}

/// Builds the per-duplicate-file edits and the plan record once every
/// gate has passed.
fn assemble_edits<S: ::std::hash::BuildHasher>(
    groups: &[Vec<DefinitionSite>],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<ConsolidatePlan, String>, RefactorError> {
    let Some(canonical) = groups.first().and_then(|group| group.first()) else {
        return Ok(Err("no definition sites".to_owned()));
    };
    let Some(module) = module_stem(&canonical.path) else {
        return Ok(Err(
            "canonical file name is not a valid module name".to_owned()
        ));
    };
    let occurrence_count = groups.first().map_or(0, Vec::len);
    let mut edits = Vec::new();
    for index in 1..occurrence_count {
        match duplicate_edits_at(index, groups, sources, parser, &module)? {
            Ok(mut file_edits) => edits.append(&mut file_edits),
            Err(reason) => return Ok(Err(reason)),
        }
    }
    Ok(Ok(plan_record(groups, canonical, edits)))
}

/// The plan record: one symbol per group, canonical file untouched.
fn plan_record(
    groups: &[Vec<DefinitionSite>],
    canonical: &DefinitionSite,
    edits: Vec<PlannedFileEdit>,
) -> ConsolidatePlan {
    ConsolidatePlan {
        symbols: groups
            .iter()
            .filter_map(|group| group.first())
            .map(|site| site.name.clone())
            .collect(),
        canonical_path: canonical.path.clone(),
        edits,
    }
}

/// Edits for the duplicate file at occurrence `index` across every
/// symbol group.
fn duplicate_edits_at<S: ::std::hash::BuildHasher>(
    index: usize,
    groups: &[Vec<DefinitionSite>],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
    module: &str,
) -> Result<Result<Vec<PlannedFileEdit>, String>, RefactorError> {
    let sites: Vec<&DefinitionSite> = groups
        .iter()
        .filter_map(|group| group.get(index))
        .collect();
    let Some(path) = sites.first().map(|site| site.path.clone()) else {
        return Ok(Ok(Vec::new()));
    };
    let Some(source) = sources.get(&path) else {
        return Ok(Err(format!("no source for {}", path.display())));
    };
    duplicate_file_edits(&sites, source, parser, module)
}

/// [AUTOFIX-CONSOLIDATE-GATE]: byte-equivalent definitions, a visible
/// canonical, same-directory sibling modules, distinct files.
fn consolidation_gate<S: ::std::hash::BuildHasher>(
    canonical: &DefinitionSite,
    duplicates: &[DefinitionSite],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
) -> Result<(), String> {
    if !canonical.visible {
        return Err(format!(
            "canonical `{}` is private — the duplicates' modules could not see it",
            canonical.name
        ));
    }
    let slices: Option<Vec<&[u8]>> = std::iter::once(canonical)
        .chain(duplicates)
        .map(|site| {
            sources
                .get(&site.path)
                .and_then(|source| source.get(site.span.start..site.span.end))
        })
        .collect();
    if !slices.is_some_and(|slices| raw_slices_equivalent(&slices)) {
        return Err("definitions are not byte-equivalent".to_owned());
    }
    for duplicate in duplicates {
        if duplicate.name != canonical.name {
            return Err("definitions disagree on the symbol name".to_owned());
        }
        if duplicate.path == canonical.path {
            return Err("duplicate and canonical share a file — use the extract action".to_owned());
        }
        if duplicate.path.parent() != canonical.path.parent() {
            return Err(
                "duplicates live outside the canonical module's directory (v1 gate)".to_owned(),
            );
        }
    }
    Ok(())
}

/// Edits for one duplicate file: delete every consolidated definition
/// and, when the file still references any consolidated symbol, import
/// the canonical ones. Together with the [`binding_drift`] gate this
/// upholds the Schäfer invariant: the file's own definitions are
/// removed and the import re-binds every remaining reference to the
/// canonical items ([AUTOFIX-CONSOLIDATE]).
fn duplicate_file_edits(
    sites: &[&DefinitionSite],
    source: &[u8],
    parser: &dyn LanguageParser,
    module: &str,
) -> Result<Result<Vec<PlannedFileEdit>, String>, RefactorError> {
    let tree = parse_source(parser.id(), &parser.grammar(), source)?;
    let spans: Vec<ByteRange> = sites.iter().map(|site| site.span).collect();
    if let Err(reason) = deletability_gate(tree.root_node(), source, sites, &spans) {
        return Ok(Err(reason));
    }
    let mut edits = deletion_edits(sites, source, &spans);
    let referenced = referenced_symbols(tree.root_node(), source, parser, sites, &spans);
    if let (Some(site), false) = (sites.first(), referenced.is_empty()) {
        let offset = import_offset(tree.root_node(), source);
        edits.push(import_edit(&site.path, module, &referenced, offset));
    }
    // Descending (start, end): a deletion starting where the import
    // inserts must apply first so both hold against original offsets.
    edits.sort_unstable_by_key(|edit| {
        (
            std::cmp::Reverse(edit.start_byte),
            std::cmp::Reverse(edit.end_byte),
        )
    });
    Ok(Ok(edits))
}

/// Insertion offset for the `use` item: after leading inner attributes
/// and inner doc comments, which must stay first in the file
/// (#279 review).
fn import_offset(root: Node<'_>, source: &[u8]) -> usize {
    let mut offset = 0;
    for child in named_children(root) {
        let inner = child.kind() == "inner_attribute"
            || (matches!(child.kind(), "line_comment" | "block_comment")
                && node_text(child, source)
                    .is_some_and(|text| text.starts_with("//!") || text.starts_with("/*!")));
        if !inner {
            break;
        }
        offset = deletion_end(source, child.end_byte());
    }
    offset
}

/// Refuses ambiguous or would-empty deletions
/// ([AUTOFIX-CONSOLIDATE-EDIT] v1 gates).
fn deletability_gate(
    root: Node<'_>,
    source: &[u8],
    sites: &[&DefinitionSite],
    spans: &[ByteRange],
) -> Result<(), String> {
    for site in sites {
        if count_definitions(root, source, &site.name) > 1 {
            return Err(format!(
                "{} defines `{}` more than once — resolution is ambiguous",
                site.path.display(),
                site.name
            ));
        }
    }
    if file_becomes_empty(source, spans) {
        let display = sites
            .first()
            .map(|site| site.path.display().to_string())
            .unwrap_or_default();
        return Err(format!(
            "{display} would become empty — file deletion needs the module declaration rewritten (v1 gate)"
        ));
    }
    Ok(())
}

/// One deletion per consolidated definition, trailing blank line
/// included.
fn deletion_edits(
    sites: &[&DefinitionSite],
    source: &[u8],
    spans: &[ByteRange],
) -> Vec<PlannedFileEdit> {
    sites
        .iter()
        .zip(spans)
        .map(|(site, span)| PlannedFileEdit {
            path: site.path.clone(),
            start_byte: span.start,
            end_byte: deletion_end(source, span.end),
            new_text: String::new(),
        })
        .collect()
}

/// Consolidated symbols still referenced outside the deleted spans,
/// alphabetical for a deterministic import.
fn referenced_symbols<'a>(
    root: Node<'_>,
    source: &[u8],
    parser: &dyn LanguageParser,
    sites: &[&'a DefinitionSite],
    spans: &[ByteRange],
) -> Vec<&'a str> {
    let mut names: Vec<&str> = sites
        .iter()
        .filter(|site| references_remain(root, source, parser, &site.name, spans))
        .map(|site| site.name.as_str())
        .collect();
    names.sort_unstable();
    names
}

/// The `use` insertion re-binding every remaining reference to the
/// canonical items.
fn import_edit(
    path: &std::path::Path,
    module: &str,
    names: &[&str],
    offset: usize,
) -> PlannedFileEdit {
    let list = match names {
        [only] => (*only).to_owned(),
        many => format!("{{{}}}", many.join(", ")),
    };
    PlannedFileEdit {
        path: path.to_path_buf(),
        start_byte: offset,
        end_byte: offset,
        new_text: format!("use crate::{module}::{list};\n\n"),
    }
}

/// Counts top-level definitions of `name` in the file.
fn count_definitions(root: Node<'_>, source: &[u8], name: &str) -> usize {
    named_children(root)
        .into_iter()
        .filter(|node| {
            node.kind() == "function_item"
                && node
                    .child_by_field_name("name")
                    .and_then(|child| node_text(child, source))
                    .as_deref()
                    == Some(name)
        })
        .count()
}

/// True when deleting every span leaves only whitespace behind.
fn file_becomes_empty(source: &[u8], spans: &[ByteRange]) -> bool {
    source.iter().enumerate().all(|(index, byte)| {
        byte.is_ascii_whitespace()
            || spans
                .iter()
                .any(|span| span.start <= index && index < span.end)
    })
}

/// Extends a deletion to swallow the trailing blank line.
fn deletion_end(source: &[u8], end: usize) -> usize {
    let mut cursor = end;
    while source.get(cursor).is_some_and(|byte| *byte == b'\n') {
        cursor = cursor.saturating_add(1);
    }
    cursor
}

/// True when the duplicate file still references `name` outside the
/// deleted definition spans.
fn references_remain(
    root: Node<'_>,
    source: &[u8],
    parser: &dyn LanguageParser,
    name: &str,
    excluded: &[ByteRange],
) -> bool {
    let references = parser.identifier_reference_kinds();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let inside_deleted = excluded
            .iter()
            .any(|span| node.start_byte() >= span.start && node.end_byte() <= span.end);
        if inside_deleted {
            continue;
        }
        if references.reference_kinds.contains(&node.kind())
            && node_text(node, source).as_deref() == Some(name)
        {
            return true;
        }
        stack.extend(named_children(node));
    }
    false
}

/// The canonical file's module name (its file stem).
fn module_stem(path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let mut chars = stem.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && chars.all(|rest| rest.is_ascii_alphanumeric() || rest == '_');
    valid.then(|| stem.to_owned())
}
