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

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    buckets::{
        bucket_labels, classify_signals, content_gated_signals, has_saturating_shape_evidence,
        is_structural_only_signals, lacks_content_support, ClusterKind, CONTENT_PROMOTE_FLOOR,
        LITERAL_TABLE_MIN_FRACTION,
    },
    cluster::Cluster,
    cluster_filters::ParseCache,
    config::ExclusionConfig,
    content::ContentEvidence,
    fingerprint::Fingerprint,
    pair::{PairScore, LSH_ONLY_MIN_JACCARD, LSH_ONLY_MIN_NODE_COUNT},
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
    parse_cache: &ParseCache,
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
                sources.get(&member.file_id).map(Vec::as_slice),
            )
        })
        .collect();
    let raw_signals: ReportSignals = cluster.signals.into();
    let kind = report_bucket_kind(
        raw_signals,
        cluster.content,
        &cluster.members,
        sources,
        file_languages,
        parse_cache,
    );
    let signals = content_gated_signals(
        proven_identical_signals(raw_signals, kind),
        cluster.content,
        kind,
    );
    let summary = summarise(
        cluster.members.len(),
        canonical_node_count,
        &cluster.members,
        &occurrences,
        sources,
        signals,
    );
    let interpretation = interpret(kind);
    // The bucket is the wire label of the routed kind — including
    // `structural_only` ([RANK-STRUCTURAL-ONLY]), which issue #134
    // introduced as a label-only override here. It is now a
    // first-class [`ClusterKind`] routed in `report_bucket_kind`, so
    // the label, the interpretation, and the ranking demotion can no
    // longer diverge (issue #197 inconsistency #1).
    let bucket = kind.wire_label().to_owned();
    let occurrences_total = occurrences.len();
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
        signals,
        bucket,
        // Default category; `render_report` re-parses the members and
        // stamps the authoritative `CloneCategory` ([RANK-CATEGORY]) using
        // the shared parse cache, so this stays `logic` here.
        category: crate::clone_category::CloneCategory::Logic
            .wire_label()
            .to_owned(),
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
    source: Option<&[u8]>,
) -> ReportOccurrence {
    let absolute = registry.path(file_id).map(Path::to_path_buf);
    let language = file_languages.get(&file_id).copied().unwrap_or("");
    let hidden = absolute
        .as_deref()
        .is_some_and(|abs| exclusion.is_report_hidden(abs, language))
        || source.is_some_and(crate::config::has_generated_header);
    let (start_line, end_line) = source.map_or((0, 0), |bytes| {
        byte_range_to_line_range(bytes, byte_range.start, byte_range.end)
    });
    let path = absolute.map_or_else(PathBuf::new, |abs| relative_to_scan_root(&abs, scan_root));
    ReportOccurrence {
        path,
        start_byte: byte_range.start,
        end_byte: byte_range.end,
        start_line,
        end_line,
        hidden,
    }
}

/// Renders `absolute` the way every report surface must: relative to
/// `scan_root` when it lies inside, unchanged when it does not.
///
/// Occurrences, boilerplate rows and per-file metrics all resolve paths
/// through here, so a single report can never mix relative and absolute
/// forms — the drift that put the user's home directory into every
/// `metrics.per_file` row ([Deslop#286]).
pub(crate) fn relative_to_scan_root(absolute: &Path, scan_root: &Path) -> PathBuf {
    absolute
        .strip_prefix(scan_root)
        .map_or_else(|_| absolute.to_path_buf(), Path::to_path_buf)
}

/// Resolves the report path for `file_id`, or an empty path when the
/// registry cannot resolve it.
pub(crate) fn display_path(file_id: FileId, registry: &FileRegistry, scan_root: &Path) -> PathBuf {
    registry
        .path(file_id)
        .map_or_else(PathBuf::new, |absolute| {
            relative_to_scan_root(absolute, scan_root)
        })
}

/// Converts a byte range into an inclusive 1-indexed line range.
fn byte_range_to_line_range(source: &[u8], start: usize, end: usize) -> (i64, i64) {
    let safe_start = start.min(source.len());
    let end_offset = end.saturating_sub(1).min(source.len());
    (
        line_for_offset(source, safe_start),
        line_for_offset(source, end_offset),
    )
}

/// Returns the 1-indexed line containing `offset`.
fn line_for_offset(source: &[u8], offset: usize) -> i64 {
    let safe_offset = offset.min(source.len());
    let line = source
        .get(..safe_offset)
        .map_or(1, |prefix| count_newlines(prefix).saturating_add(1));
    i64::try_from(line).unwrap_or(i64::MAX)
}

/// Counts newline bytes in `source`.
fn count_newlines(source: &[u8]) -> usize {
    source
        .split(|byte| *byte == b'\n')
        .count()
        .saturating_sub(1)
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

/// Lowest `token_jaccard` [`classify_signals`] will tolerate while still
/// labelling a cluster `Identical` on the signal alone. Any cluster that
/// reaches [`ClusterKind::Identical`] with a value below this floor was
/// promoted by the byte-equivalence upgrade in [`report_bucket_kind`], not
/// by its token signal — the GH #232 fingerprint to correct.
const PROVEN_IDENTICAL_TOKEN_JACCARD_FLOOR: f64 = 0.99;

/// Corrects the reported `token_jaccard` for clusters the byte-equivalence
/// upgrade in [`report_bucket_kind`] routed to [`ClusterKind::Identical`]
/// (GH #232).
///
/// A synthetic sibling-window fingerprint matches no single AST node, so the
/// non-language signature path resolves it to a byte-offset-seeded fallback
/// signature; two byte-identical occurrences in different files then estimate
/// `token_jaccard ≈ 0`. Once a cluster is *proven* `Identical` (its members'
/// raw source slices are byte-equivalent), their normalised token multisets
/// are identical by construction, so the true Jaccard is 1.0. We only touch
/// the upgraded case: [`classify_signals`] never labels a cluster `Identical`
/// below [`PROVEN_IDENTICAL_TOKEN_JACCARD_FLOOR`], so the demoted
/// `structural_only` family (renamed, *not* byte-equivalent) keeps its
/// unscored signal and its ranking demotion.
fn proven_identical_signals(signals: ReportSignals, kind: ClusterKind) -> ReportSignals {
    if kind != ClusterKind::Identical
        || signals.token_jaccard >= PROVEN_IDENTICAL_TOKEN_JACCARD_FLOOR
    {
        return signals;
    }
    PairScore {
        structural: signals.structural,
        token_jaccard: 1.0,
        embedding_cos: signals.embedding_cos,
    }
    .into()
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
    content: ContentEvidence,
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
    parse_cache: &ParseCache,
) -> ClusterKind {
    let kind = if is_csharp_lsh_type3_near_miss(signals, members, file_languages) {
        ClusterKind::NearlyIdentical
    } else {
        classify_signals(signals)
    };
    let equivalent =
        source_slices_are_equivalent_for_language(members, sources, file_languages, parse_cache);
    // `StructuralOnly` joins `NearlyIdentical` in the byte-equivalence
    // upgrade: an exact-structural cluster whose raw source slices are
    // equivalent is a proven Type-1/2 clone regardless of the unscored
    // token signal ([CLONE-BUCKETS-IDENTICAL]), so it must never sink
    // into the demoted structural-only tier.
    let kind = match (kind, equivalent, signals.structural >= 0.99) {
        (ClusterKind::Identical, false, _) => ClusterKind::NearlyIdentical,
        (ClusterKind::NearlyIdentical | ClusterKind::StructuralOnly, true, true) => {
            ClusterKind::Identical
        }
        _ => kind,
    };
    // Structural-only routing ([RANK-STRUCTURAL-ONLY], one shared
    // predicate with the ranking demotion — issue #197 inconsistency
    // #1). Source-bytes equivalent clusters (Identical) keep their
    // bucket because byte-level proof is independent of the signal
    // triple. Two destinations:
    //
    // - Issue #134: a *cross-file multi-copy* structural-only match
    //   (3+ occurrences spread across 3+ files) is test scaffolding /
    //   generated boilerplate — demoted to `LooselySimilar`, which the
    //   renderer hides.
    // - Everything else with shape-only evidence becomes
    //   [`ClusterKind::StructuralOnly`]: surfaced and labelled
    //   honestly, demoted in ranking by the `[ranking]`
    //   `structural_only` policy. The *single-file* sibling-method
    //   family (issue #197) is additionally hidden by the renderer's
    //   `cluster_is_hidden` AST pass, which needs the CST this
    //   signal-only routing does not have.
    route_shape_identical(kind, signals, content, members)
}

/// [FUSION-CONTENT-GATE] routing tail: for shape-identical clusters the
/// measured content evidence decides in both directions. It subsumes
/// the legacy `token_jaccard < 0.05` structural-only test (a
/// fallback-signature artifact scores `0.0` content on unresolvable
/// members) and overrides it the other way: a cluster either population
/// vouches for — pooled raw bytes that mostly agree, or a
/// literal-anchored consistent rename ([`ContentEvidence::support`]) —
/// is a genuine clone even when the token layer lost its signature to
/// the fingerprint-scoped fallback (gh #339). Support below the floor
/// with no semantic backing routes to the demoted structural-only tier
/// — [`ClusterKind::LooselySimilar`] for the cross-file scaffolding
/// spread (#134), [`ClusterKind::StructuralOnly`] otherwise.
fn route_shape_identical(
    kind: ClusterKind,
    signals: ReportSignals,
    content: ContentEvidence,
    members: &[Fingerprint],
) -> ClusterKind {
    if !matches!(
        kind,
        ClusterKind::NearlyIdentical | ClusterKind::StructuralOnly
    ) || !has_saturating_shape_evidence(signals)
    {
        return kind;
    }
    let shape_only =
        is_structural_only_signals(signals) || lacks_content_support(signals, content);
    if !shape_only {
        return kind;
    }
    // Promotion rescues real clones from the demoted tier for small
    // spreads only: a fallback-signature artifact whose raw bytes agree
    // (gh #339), or a maximal Type-2 rename whose literals and
    // identifier mapping prove the copy. The cross-file scaffolding
    // spread (#134) is excluded: contract-mandated wiring (trait
    // adapter impls, framework overrides) pins the very leaves both
    // populations measure, so a 3+-file same-shape family scoring high
    // support is still scaffolding and keeps the legacy hidden
    // destination.
    if content.support() >= CONTENT_PROMOTE_FLOOR && !is_cross_file_scaffolding(members) {
        return ClusterKind::NearlyIdentical;
    }
    // Literal-dominated families ([CLONE-NOISE-LITERAL-TABLE], #336)
    // stay in the surfaced `structural_only` tier instead of the hidden
    // scaffolding one: the data-category policy ([RANK-CATEGORY]) owns
    // their visibility, and a policy knob cannot govern a cluster the
    // renderer already made disappear.
    if is_cross_file_scaffolding(members) && content.literal_fraction < LITERAL_TABLE_MIN_FRACTION {
        return ClusterKind::LooselySimilar;
    }
    ClusterKind::StructuralOnly
}

/// Returns true when a structural-only cluster spans enough distinct
/// files to mirror the cross-test-file scaffolding pattern from issue
/// #134. Caller has already established the structural-only signal
/// shape via [`is_structural_only_signals`]. The 3-member, 3-file
/// floors preserve small two-occurrence pairs; smaller spreads route
/// to [`ClusterKind::StructuralOnly`] instead.
fn is_cross_file_scaffolding(members: &[Fingerprint]) -> bool {
    if members.len() < 3 {
        return false;
    }
    let mut files: Vec<FileId> = members.iter().map(|member| member.file_id).collect();
    files.sort_unstable();
    files.dedup();
    files.len() >= 3
}

/// Returns true for substantive C# Type-3 candidates found only through
/// token LSH. These have no exact structural anchor (`structural=0.0`),
/// but they passed the LSH-only Jaccard and node-count floors, so they
/// are real near-miss duplication rather than the low-information token
/// noise hidden from the ranked report.
fn is_csharp_lsh_type3_near_miss<S: BuildHasher>(
    signals: ReportSignals,
    members: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    signals.structural <= f64::EPSILON
        && signals.embedding_cos <= f64::EPSILON
        && signals.token_jaccard >= LSH_ONLY_MIN_JACCARD
        && members.len() >= 2
        && members
            .iter()
            .all(|member| member.node_count >= LSH_ONLY_MIN_NODE_COUNT)
        && members
            .iter()
            .all(|member| file_languages.get(&member.file_id).copied() == Some("csharp"))
        && members
            .first()
            .is_some_and(|first| members.iter().any(|member| member.file_id != first.file_id))
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
    parse_cache: &ParseCache,
) -> bool {
    source_slices_are_equivalent(members, sources)
        || csharp_method_declarations_are_equivalent(members, sources, file_languages, parse_cache)
}

/// Compares contained C# methods when the clone range includes wrappers.
/// Member CSTs come from the shared per-render `parse_cache` so a file is
/// parsed at most once per report regardless of cluster or member count —
/// re-parsing per member pinned the LSP for minutes on large C# corpora
/// (GH #239, [CLONE-NOISE-REPARSE-CACHE]).
fn csharp_method_declarations_are_equivalent(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, impl BuildHasher>,
    parse_cache: &ParseCache,
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
        let Some(tree) = parse_cache.tree_for(member.file_id, "csharp", source) else {
            return false;
        };
        method_sets.push(csharp_methods_in_range(&tree, source, member.byte_range));
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

/// Extracts canonical C# method declarations contained by `range` from a
/// cached CST.
fn csharp_methods_in_range(
    tree: &tree_sitter::Tree,
    source: &[u8],
    range: ByteRange,
) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    collect_csharp_methods(tree.root_node(), range, source, &mut out);
    out
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
pub(crate) fn canonicalise_whitespace(bytes: &[u8]) -> Vec<u8> {
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
            report_bucket_kind(
                identical_signals(),
                ContentEvidence::unmeasured(),
                &members,
                &sources,
                &file_languages,
                &ParseCache::new(),
            ),
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
