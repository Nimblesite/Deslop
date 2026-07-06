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
/// every duplicate file re-pointed at the canonical symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsolidatePlan {
    /// Symbol being consolidated.
    pub symbol: String,
    /// Path of the canonical definition (kept untouched).
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
struct DefinitionSite {
    /// Workspace-relative file path as reported.
    path: PathBuf,
    /// Span of the whole definition in that file.
    span: ByteRange,
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
    match definition_sites(cluster, sources, parser)? {
        Err(reason) => Ok(ConsolidationOutcome::Refused(reason)),
        Ok(sites) => Ok(build_plan(&sites, sources, parser)?.map_or_else(
            ConsolidationOutcome::Refused,
            ConsolidationOutcome::Mechanical,
        )),
    }
}

/// Resolves every occurrence to a whole, top-level, byte-equivalent
/// definition ([AUTOFIX-CONSOLIDATE] shape gate).
fn definition_sites<S: ::std::hash::BuildHasher>(
    cluster: &ReportCluster,
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<Vec<DefinitionSite>, String>, RefactorError> {
    let distinct_paths: std::collections::HashSet<&PathBuf> = cluster
        .occurrences
        .iter()
        .map(|entry| &entry.path)
        .collect();
    if cluster.occurrences.len() < 2 || distinct_paths.len() < 2 || cluster.occurrences_truncated {
        return Ok(Err(
            "consolidation needs untruncated occurrences in at least two files".to_owned(),
        ));
    }
    let mut sites = Vec::new();
    for occurrence in &cluster.occurrences {
        let Some(source) = sources.get(&occurrence.path) else {
            return Ok(Err(format!("no source for {}", occurrence.path.display())));
        };
        let tree = parse_source(parser.id(), &parser.grammar(), source)?;
        let span = ByteRange {
            start: occurrence.start_byte,
            end: occurrence.end_byte,
        };
        match definition_site(tree.root_node(), span, source, &occurrence.path) {
            Some(site) => sites.push(site),
            None => {
                return Ok(Err(
                    "an occurrence is not a whole top-level function definition".to_owned(),
                ));
            }
        }
    }
    Ok(Ok(sites))
}

/// Resolves one occurrence to a top-level `fn` definition site. The
/// pipeline's sibling windows can start mid-signature (after the
/// visibility modifier), so any window covering the whole body widens
/// to the full `function_item` — the span the consolidation deletes.
fn definition_site(
    root: Node<'_>,
    span: ByteRange,
    source: &[u8],
    path: &std::path::Path,
) -> Option<DefinitionSite> {
    let node = root.named_descendant_for_byte_range(span.start, span.end)?;
    if node.kind() != "function_item"
        || node.parent().map(|parent| parent.kind()) != Some("source_file")
    {
        return None;
    }
    let body_covered = node
        .child_by_field_name("body")
        .is_some_and(|body| span.start <= body.start_byte() && body.end_byte() <= span.end);
    if !body_covered {
        return None;
    }
    let name = node
        .child_by_field_name("name")
        .and_then(|child| node_text(child, source))?;
    let visible = named_children(node)
        .into_iter()
        .any(|child| child.kind() == "visibility_modifier");
    Some(DefinitionSite {
        path: path.to_path_buf(),
        span: ByteRange {
            start: node.start_byte(),
            end: node.end_byte(),
        },
        name,
        visible,
    })
}

/// Applies the remaining gates and assembles the edits. The inner
/// `Err` is the refusal reason.
fn build_plan<S: ::std::hash::BuildHasher>(
    sites: &[DefinitionSite],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<ConsolidatePlan, String>, RefactorError> {
    let Some((canonical, duplicates)) = sites.split_first() else {
        return Ok(Err("no definition sites".to_owned()));
    };
    if let Err(reason) = consolidation_gate(canonical, duplicates, sources) {
        return Ok(Err(reason));
    }
    let Some(module) = module_stem(&canonical.path) else {
        return Ok(Err(
            "canonical file name is not a valid module name".to_owned()
        ));
    };
    let mut edits = Vec::new();
    for duplicate in duplicates {
        let Some(source) = sources.get(&duplicate.path) else {
            return Ok(Err(format!("no source for {}", duplicate.path.display())));
        };
        match duplicate_edits(duplicate, source, parser, &module)? {
            Ok(mut file_edits) => edits.append(&mut file_edits),
            Err(reason) => return Ok(Err(reason)),
        }
    }
    Ok(Ok(ConsolidatePlan {
        symbol: canonical.name.clone(),
        canonical_path: canonical.path.clone(),
        edits,
    }))
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

/// Edits for one duplicate file: delete the definition and, when the
/// file still references the symbol, import the canonical one. The
/// Schäfer invariant holds by construction: the only definition of the
/// name in the file is removed and the import re-binds every reference
/// to the canonical item ([AUTOFIX-CONSOLIDATE]).
fn duplicate_edits(
    duplicate: &DefinitionSite,
    source: &[u8],
    parser: &dyn LanguageParser,
    module: &str,
) -> Result<Result<Vec<PlannedFileEdit>, String>, RefactorError> {
    let tree = parse_source(parser.id(), &parser.grammar(), source)?;
    if count_definitions(tree.root_node(), source, &duplicate.name) > 1 {
        return Ok(Err(format!(
            "{} defines `{}` more than once — resolution is ambiguous",
            duplicate.path.display(),
            duplicate.name
        )));
    }
    if file_becomes_empty(source, duplicate.span) {
        return Ok(Err(format!(
            "{} would become empty — file deletion needs the module declaration rewritten (v1 gate)",
            duplicate.path.display()
        )));
    }
    let mut edits = vec![PlannedFileEdit {
        path: duplicate.path.clone(),
        start_byte: duplicate.span.start,
        end_byte: deletion_end(source, duplicate.span.end),
        new_text: String::new(),
    }];
    if references_remain(tree.root_node(), source, parser, duplicate) {
        edits.push(PlannedFileEdit {
            path: duplicate.path.clone(),
            start_byte: 0,
            end_byte: 0,
            new_text: format!("use crate::{module}::{};\n\n", duplicate.name),
        });
    }
    edits.sort_unstable_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    Ok(Ok(edits))
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

/// True when deleting `span` leaves only whitespace behind.
fn file_becomes_empty(source: &[u8], span: ByteRange) -> bool {
    let head = source.get(..span.start).unwrap_or_default();
    let tail = source.get(span.end..).unwrap_or_default();
    head.iter().all(u8::is_ascii_whitespace) && tail.iter().all(u8::is_ascii_whitespace)
}

/// Extends a deletion to swallow the trailing blank line.
fn deletion_end(source: &[u8], end: usize) -> usize {
    let mut cursor = end;
    while source.get(cursor).is_some_and(|byte| *byte == b'\n') {
        cursor = cursor.saturating_add(1);
    }
    cursor
}

/// True when the duplicate file still references the symbol outside
/// the deleted definition.
fn references_remain(
    root: Node<'_>,
    source: &[u8],
    parser: &dyn LanguageParser,
    duplicate: &DefinitionSite,
) -> bool {
    let references = parser.identifier_reference_kinds();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let inside_definition =
            node.start_byte() >= duplicate.span.start && node.end_byte() <= duplicate.span.end;
        if inside_definition {
            continue;
        }
        if references.reference_kinds.contains(&node.kind())
            && node_text(node, source).as_deref() == Some(duplicate.name.as_str())
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
