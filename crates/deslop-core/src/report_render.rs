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

use crate::{
    ast::ByteRange,
    buckets::{bucket_labels, classify_signals, ClusterKind},
    cluster::Cluster,
    config::ExclusionConfig,
    fingerprint::Fingerprint,
    report::{ReportCluster, ReportOccurrence, ReportSignals},
    report_literals::value_tokens_are_identical,
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
/// [`ClusterKind::NearlyIdentical`] regardless of language. The
/// language-aware C# value-token check is kept as a redundant guard for
/// reports that lose source bytes (deserialised reports, tests) but the
/// raw-source equality check is now the primary gate.
pub(crate) fn report_bucket_kind(
    signals: ReportSignals,
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
) -> ClusterKind {
    let kind = classify_signals(signals);
    if kind == ClusterKind::Identical
        && (!source_slices_are_equivalent(members, sources)
            || !value_tokens_are_identical(members, sources, file_languages))
    {
        ClusterKind::NearlyIdentical
    } else {
        kind
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
/// member's source bytes are unavailable (deserialised reports, tests)
/// the function returns `true` so the legacy language-aware fallback in
/// [`value_tokens_are_identical`] still gates the bucket.
pub(crate) fn source_slices_are_equivalent(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> bool {
    let canonical_slices: Vec<Vec<u8>> = members
        .iter()
        .filter_map(|member| source_slice(member, sources).map(canonicalise_whitespace))
        .collect();
    if canonical_slices.len() < 2 {
        return true;
    }
    canonical_slices
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
