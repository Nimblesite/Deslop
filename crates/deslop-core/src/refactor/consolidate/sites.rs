//! Occurrence-to-definition-site resolution for the consolidation
//! engine ([AUTOFIX-CONSOLIDATE-GATE], v1.1 definition runs): each
//! occurrence span resolves to one whole top-level `fn` definition or
//! a contiguous run of them, and the per-occurrence lists transpose
//! into per-symbol groups the gates consume.

use std::{collections::HashMap, path::PathBuf};

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    lang::{shared::parse_source, LanguageParser},
    refactor::{
        consolidate::DefinitionSite,
        preconditions::{named_children, node_text},
        RefactorError,
    },
    report::{ReportCluster, ReportOccurrence},
};

/// Resolves every visible occurrence to whole top-level definitions —
/// one site for a single-definition span, several for a contiguous run
/// ([AUTOFIX-CONSOLIDATE-GATE] v1.1 definition runs). Hidden
/// occurrences are excluded, matching the LSP offer screen
/// ([AUTOFIX-CONSOLIDATE-SURFACE] parity).
pub(super) fn occurrence_sites<S: ::std::hash::BuildHasher>(
    cluster: &ReportCluster,
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<Vec<Vec<DefinitionSite>>, String>, RefactorError> {
    let visible: Vec<&ReportOccurrence> = cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .collect();
    if let Err(reason) = cross_file_screen(cluster) {
        return Ok(Err(reason));
    }
    let mut per_occurrence = Vec::new();
    for occurrence in visible {
        let Some(source) = sources.get(&occurrence.path) else {
            return Ok(Err(format!("no source for {}", occurrence.path.display())));
        };
        let tree = parse_source(parser.id(), &parser.grammar(), source)?;
        let span = ByteRange {
            start: occurrence.start_byte,
            end: occurrence.end_byte,
        };
        match sites_in_span(tree.root_node(), span, source, &occurrence.path) {
            Some(sites) => per_occurrence.push(sites),
            None => {
                return Ok(Err(
                    "an occurrence is not a run of whole top-level function definitions".to_owned(),
                ));
            }
        }
    }
    Ok(Ok(per_occurrence))
}

/// The visible occurrences must span at least two files, untruncated.
fn cross_file_screen(cluster: &ReportCluster) -> Result<(), String> {
    if crate::report::distinct_visible_path_count(cluster) < 2 || cluster.occurrences_truncated {
        return Err("consolidation needs untruncated occurrences in at least two files".to_owned());
    }
    Ok(())
}

/// Sites covered by one occurrence span: a single whole definition, or
/// a contiguous run of them ([AUTOFIX-CONSOLIDATE-GATE] v1.1).
fn sites_in_span(
    root: Node<'_>,
    span: ByteRange,
    source: &[u8],
    path: &std::path::Path,
) -> Option<Vec<DefinitionSite>> {
    if let Some(site) = definition_site(root, span, source, path) {
        return Some(vec![site]);
    }
    definition_run_sites(root, span, source, path)
}

/// A span covering ≥2 whole top-level definitions and nothing else —
/// the pipeline's sibling windows emit exactly this shape for adjacent
/// duplicated functions ([AUTOFIX-CONSOLIDATE-GATE] v1.1).
fn definition_run_sites(
    root: Node<'_>,
    span: ByteRange,
    source: &[u8],
    path: &std::path::Path,
) -> Option<Vec<DefinitionSite>> {
    let mut sites = Vec::new();
    for child in named_children(root) {
        if child.end_byte() <= span.start || child.start_byte() >= span.end {
            continue;
        }
        sites.push(covered_definition_site(child, span, source, path)?);
    }
    (sites.len() >= 2).then_some(sites)
}

/// Resolves one occurrence to a single top-level `fn` definition site.
fn definition_site(
    root: Node<'_>,
    span: ByteRange,
    source: &[u8],
    path: &std::path::Path,
) -> Option<DefinitionSite> {
    let node = root.named_descendant_for_byte_range(span.start, span.end)?;
    if node.parent().map(|parent| parent.kind()) != Some("source_file") {
        return None;
    }
    covered_definition_site(node, span, source, path)
}

/// One `function_item` whose body the span fully covers. The
/// pipeline's sibling windows can start mid-signature (after the
/// visibility modifier), so any window covering the whole body widens
/// to the full `function_item` — the span the consolidation deletes.
fn covered_definition_site(
    node: Node<'_>,
    span: ByteRange,
    source: &[u8],
    path: &std::path::Path,
) -> Option<DefinitionSite> {
    if node.kind() != "function_item" {
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
            start: decorated_start(node, source),
            end: node.end_byte(),
        },
        item_span: ByteRange {
            start: node.start_byte(),
            end: node.end_byte(),
        },
        name,
        visible,
    })
}

/// Outer attributes and doc comments are *sibling* nodes in
/// tree-sitter-rust; they belong to the definition, move with it, and
/// count toward the byte-equivalence proof. Inner doc
/// comments (`//!`, `/*!`) belong to the file and never extend a site.
fn decorated_start(node: Node<'_>, source: &[u8]) -> usize {
    let mut start = node.start_byte();
    let mut current = node;
    while let Some(previous) = current.prev_named_sibling() {
        if !is_decoration(previous, source) {
            break;
        }
        start = previous.start_byte();
        current = previous;
    }
    start
}

/// True for attribute items and outer (non-`//!`) comments.
fn is_decoration(node: Node<'_>, source: &[u8]) -> bool {
    match node.kind() {
        "attribute_item" => true,
        "line_comment" | "block_comment" => node_text(node, source)
            .is_some_and(|text| !text.starts_with("//!") && !text.starts_with("/*!")),
        _ => false,
    }
}

/// Transposes per-occurrence site lists into per-symbol groups; every
/// occurrence must resolve the same ordered symbol list
/// ([AUTOFIX-CONSOLIDATE-GATE] v1.1).
pub(super) fn symbol_groups(
    per_occurrence: &[Vec<DefinitionSite>],
) -> Result<Vec<Vec<DefinitionSite>>, String> {
    let Some(first) = per_occurrence.first() else {
        return Err("no definition sites".to_owned());
    };
    let matching = per_occurrence.iter().all(|sites| {
        sites.len() == first.len()
            && sites
                .iter()
                .zip(first)
                .all(|(site, lead)| site.name == lead.name)
    });
    if !matching {
        return Err("occurrences disagree on the definition run".to_owned());
    }
    Ok((0..first.len())
        .map(|index| {
            per_occurrence
                .iter()
                .filter_map(|sites| sites.get(index))
                .cloned()
                .collect()
        })
        .collect())
}
