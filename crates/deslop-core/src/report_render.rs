//! Private rendering helpers used exclusively by [`crate::report::render_report`].
//!
//! These functions convert internal pipeline types into the agent-first
//! output contract described in [PRINCIPLES-AUDIENCE-AGENT]. All helpers
//! are `pub(crate)` — they have no stable public API contract and must
//! not be called outside `report.rs`.

use std::{
    collections::HashMap,
    hash::BuildHasher,
    path::{Path, PathBuf},
};

use tree_sitter::{Node, Parser};

use crate::{
    ast::ByteRange,
    buckets::{bucket_labels, classify_signals, ClusterKind},
    cluster::Cluster,
    config::ExclusionConfig,
    fingerprint::Fingerprint,
    report::{ReportCluster, ReportOccurrence, ReportSignals},
    report_location::format_occurrence,
    state::{FileId, FileRegistry},
};

/// Converts one internal [`Cluster`] to a [`ReportCluster`].
pub(crate) fn cluster_to_report<S: BuildHasher>(
    cluster: &Cluster,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
    sources: &HashMap<FileId, Vec<u8>>,
) -> ReportCluster {
    let canonical_node_count = cluster
        .members
        .first()
        .map(|member| member.node_count)
        .unwrap_or_default();
    let occurrences: Vec<ReportOccurrence> = cluster
        .members
        .iter()
        .map(|member| {
            occurrence(
                member.file_id,
                member.byte_range,
                registry,
                file_languages,
                scan_root,
                exclusion,
            )
        })
        .collect();
    let signals: ReportSignals = cluster.signals.into();
    let summary = summarise(
        cluster.members.len(),
        canonical_node_count,
        &cluster.members,
        &occurrences,
        sources,
        signals,
    );
    let kind = report_bucket_kind(signals, &cluster.members, sources, file_languages);
    let interpretation = interpret(kind);
    let bucket = kind.wire_label().to_owned();
    let occurrences_total = occurrences.len();
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
        signals,
        bucket,
        occurrences,
        occurrences_total,
        occurrences_truncated: false,
        summary,
        interpretation,
    }
}

/// Builds a [`ReportOccurrence`] for a single fingerprint member.
pub(crate) fn occurrence<S: BuildHasher>(
    file_id: FileId,
    byte_range: ByteRange,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
) -> ReportOccurrence {
    let absolute = registry.path(file_id).map(Path::to_path_buf);
    let language = file_languages.get(&file_id).copied().unwrap_or("");
    let hidden = absolute
        .as_deref()
        .is_some_and(|abs| exclusion.is_report_hidden(abs, language));
    let path = absolute.map_or_else(PathBuf::new, |abs| {
        abs.strip_prefix(scan_root)
            .map_or_else(|_| abs.clone(), Path::to_path_buf)
    });
    ReportOccurrence {
        path,
        start_byte: byte_range.start,
        end_byte: byte_range.end,
        hidden,
    }
}

/// Produces a short, agent-readable one-line summary for the cluster.
/// Includes the per-signal breakdown so a downstream agent can tell
/// whether the cluster fired on structure, tokens, or both
/// ([PRINCIPLES-AUDIENCE-AGENT]).
pub(crate) fn summarise(
    size: usize,
    canonical_node_count: usize,
    members: &[Fingerprint],
    occurrences: &[ReportOccurrence],
    sources: &HashMap<FileId, Vec<u8>>,
    signals: ReportSignals,
) -> String {
    let locations: Vec<String> = occurrences
        .iter()
        .zip(members)
        .take(3)
        .map(|(occ, member)| source_location(occ, member.file_id, sources))
        .collect();
    let suffix = if occurrences.len() > locations.len() {
        format!(
            " (+{} more)",
            occurrences.len().saturating_sub(locations.len())
        )
    } else {
        String::new()
    };
    format!(
        "{size} copies of a {canonical_node_count}-node subtree at {locs}{suffix} \
         [structural={structural:.2}, token_jaccard={token:.2}, embedding_cos={embed:.2}]",
        locs = locations.join(", "),
        structural = signals.structural,
        token = signals.token_jaccard,
        embed = signals.embedding_cos,
    )
}

/// Routes the signal triple into the report bucket and is the *single
/// source of truth* for the [CLONE-BUCKETS-IDENTICAL] downgrade.
///
/// Issue #66: structural normalisation collapses identifiers and literals,
/// so two snippets that share AST shape but differ in routes, handlers, or
/// rate-limit policy literals still reach `structural=1.00, jaccard=1.00`.
/// Calling them "Identical code / every copy is the same" is a lie — the
/// raw source bytes disagree. We downgrade any such cluster to
/// [`ClusterKind::NearlyIdentical`] regardless of language. Reports that
/// cannot supply source bytes also cannot prove byte-equivalence, so they
/// take the same downgrade instead of using a compatibility fallback.
pub(crate) fn report_bucket_kind(
    signals: ReportSignals,
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
) -> ClusterKind {
    let kind = classify_signals(signals);
    let equivalent = source_slices_are_equivalent_for_language(members, sources, file_languages);
    match (kind, equivalent, signals.structural >= 0.99) {
        (ClusterKind::Identical, false, _) => ClusterKind::NearlyIdentical,
        (ClusterKind::NearlyIdentical, true, true) => ClusterKind::Identical,
        _ => kind,
    }
}

/// Maps the report bucket onto a one-line interpretation for AI agents.
/// `kind` is already the authoritative bucket from [`report_bucket_kind`],
/// so an `Identical` kind here is guaranteed to reflect byte-equivalent
/// source slices ([CLONE-BUCKETS-IDENTICAL] single-source-of-truth).
pub(crate) fn interpret(kind: ClusterKind) -> String {
    bucket_labels(kind).agent_summary()
}

/// Returns true when every cluster member maps to source bytes that are
/// equal after collapsing ASCII whitespace runs. Whitespace-insensitive so
/// reformatted-but-identical copies still classify as `Identical`, but
/// any difference in identifiers, literals, or punctuation prevents the
/// `Identical` label per [CLONE-BUCKETS-IDENTICAL] (issue #66). When a
/// member's source bytes are unavailable, the function returns `false`
/// because the renderer cannot prove byte-equivalence.
pub(crate) fn source_slices_are_equivalent(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> bool {
    if members.len() < 2 {
        return true;
    }
    let mut canonical_slices = Vec::with_capacity(members.len());
    for member in members {
        let Some(slice) = source_slice(member, sources) else {
            return false;
        };
        canonical_slices.push(canonicalise_whitespace(slice));
    }
    canonical_slices
        .windows(2)
        .all(|window| matches!(window, [left, right] if left == right))
}

/// Returns true when source slices are equivalent for the member language.
fn source_slices_are_equivalent_for_language(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
) -> bool {
    source_slices_are_equivalent(members, sources)
        || csharp_method_declarations_are_equivalent(members, sources, file_languages)
}

/// Compares contained C# methods when the clone range includes wrappers.
fn csharp_method_declarations_are_equivalent(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
) -> bool {
    if !members
        .iter()
        .all(|member| language_is(file_languages, member, "csharp"))
    {
        return false;
    }
    let mut method_sets = Vec::with_capacity(members.len());
    for member in members {
        let Some(source) = sources.get(&member.file_id) else {
            return false;
        };
        method_sets.push(csharp_methods_in_range(source, member.byte_range));
    }
    equivalent_method_sets(&method_sets)
}

/// Returns true when a member belongs to `language`.
fn language_is(
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
    member: &Fingerprint,
    language: &str,
) -> bool {
    file_languages.get(&member.file_id).copied() == Some(language)
}

/// Extracts canonical C# method declarations contained by `range`.
fn csharp_methods_in_range(source: &[u8], range: ByteRange) -> Vec<Vec<u8>> {
    let mut parser = Parser::new();
    let language = tree_sitter_c_sharp::LANGUAGE.into();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    parser.parse(source, None).map_or_else(Vec::new, |tree| {
        let mut out = Vec::new();
        collect_csharp_methods(tree.root_node(), range, source, &mut out);
        out
    })
}

/// Collects method declarations fully contained in `range`.
fn collect_csharp_methods(node: Node<'_>, range: ByteRange, source: &[u8], out: &mut Vec<Vec<u8>>) {
    if outside_range(node, range) {
        return;
    }
    if contained_method(node, range) {
        if let Some(method) = source.get(node.start_byte()..node.end_byte()) {
            out.push(canonicalise_whitespace(method));
        }
        return;
    }
    collect_csharp_method_children(node, range, source, out);
}

/// Recurses over a node's children.
fn collect_csharp_method_children(
    node: Node<'_>,
    range: ByteRange,
    source: &[u8],
    out: &mut Vec<Vec<u8>>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_csharp_methods(child, range, source, out);
    }
}

/// Returns true when `node` has no overlap with `range`.
fn outside_range(node: Node<'_>, range: ByteRange) -> bool {
    node.end_byte() <= range.start || node.start_byte() >= range.end
}

/// Returns true when `node` is a method fully contained by `range`.
fn contained_method(node: Node<'_>, range: ByteRange) -> bool {
    node.kind() == "method_declaration"
        && range.start <= node.start_byte()
        && node.end_byte() <= range.end
}

/// Returns true when all non-empty method sets are byte-equivalent.
fn equivalent_method_sets(method_sets: &[Vec<Vec<u8>>]) -> bool {
    method_sets.iter().all(|methods| !methods.is_empty())
        && method_sets
            .windows(2)
            .all(|window| matches!(window, [left, right] if left == right))
}

/// Collapses all runs of ASCII whitespace in `bytes` to a single space and
/// trims leading / trailing whitespace. Used to compare source slices
/// without being fooled by formatting differences.
fn canonicalise_whitespace(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut last_was_space = true;
    for &byte in bytes {
        if byte.is_ascii_whitespace() {
            if !last_was_space {
                out.push(b' ');
                last_was_space = true;
            }
        } else {
            out.push(byte);
            last_was_space = false;
        }
    }
    if out.last() == Some(&b' ') {
        let _popped = out.pop();
    }
    out
}

/// Borrows the source bytes covered by one fingerprint range.
pub(crate) fn source_slice<'a>(
    member: &Fingerprint,
    sources: &'a HashMap<FileId, Vec<u8>>,
) -> Option<&'a [u8]> {
    sources
        .get(&member.file_id)?
        .get(member.byte_range.start..member.byte_range.end)
}

/// Formats one occurrence through the shared human-location renderer.
pub(crate) fn source_location(
    occurrence: &ReportOccurrence,
    file_id: FileId,
    sources: &HashMap<FileId, Vec<u8>>,
) -> String {
    let source = sources.get(&file_id).map(Vec::as_slice);
    format_occurrence(&occurrence.path, occurrence.start_byte, source)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use crate::{
        ast::ByteRange,
        fingerprint::Fingerprint,
        report::ReportSignals,
        state::{FileId, FileRegistry},
    };

    use super::*;

    #[test]
    fn missing_source_bytes_do_not_use_legacy_language_fallback_issue_85() {
        let mut registry = FileRegistry::new();
        let left = registry.register(PathBuf::from("Left.cs"));
        let right = registry.register(PathBuf::from("Right.cs"));
        let members = [fingerprint(left), fingerprint(right)];
        let sources = HashMap::<FileId, Vec<u8>>::new();
        let file_languages = HashMap::from([(left, "csharp"), (right, "csharp")]);

        assert!(!source_slices_are_equivalent(&members, &sources));
        assert_eq!(
            report_bucket_kind(identical_signals(), &members, &sources, &file_languages),
            ClusterKind::NearlyIdentical
        );
    }

    fn fingerprint(file_id: FileId) -> Fingerprint {
        Fingerprint {
            hash: [0; 32],
            file_id,
            byte_range: ByteRange { start: 0, end: 12 },
            node_count: 8,
        }
    }

    fn identical_signals() -> ReportSignals {
        ReportSignals {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.0,
            fused: 1.0,
        }
    }
}
