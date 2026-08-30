//! Cluster-id resolution shared by the LSP and MCP transports
//! (, [VSIX-CLUSTER-ID-CONSISTENCY]).
//!
//! The VSIX surfaces a 7-hex slug (`clusterSlug()` in
//! `clients/vscode/src/types/report.ts`) across every cluster panel —
//! tree, hover, bubble, webview. An agent that quotes the slug from one
//! of those panels must be able to feed it straight to `cluster-by-id`
//! without first expanding to the full 16-hex canonical id. This module
//! owns that lookup so the LSP IPC, the MCP wire, and `find-similar`'s
//! id-match fast path all agree on what a "match" means.

use crate::report::ReportCluster;

use super::errors::LiveError;

/// Minimum cluster-id prefix length accepted by
/// [`resolve_cluster_by_id_prefix`]. Mirrors the 7-hex slug surfaced in
/// every UI panel via `clusterSlug()` ([VSIX-CLUSTER-ID-CONSISTENCY])
/// so an agent that copied the slug from a hover bubble can hand it
/// straight to `cluster-by-id` without expanding to the full 16-hex
/// canonical id.
pub const MIN_CLUSTER_ID_PREFIX_LEN: usize = 7;

/// Resolves a cluster from a possibly-truncated id.
///
/// Accepts either the full 16-hex canonical id or any prefix at least
/// [`MIN_CLUSTER_ID_PREFIX_LEN`] characters long (matches the 7-hex
/// `clusterSlug()` semantics in the VSIX). Returns
/// [`LiveError::UnknownCluster`] when:
/// - no cluster id starts with the prefix,
/// - the prefix is shorter than the slug floor (prevents stray single
///   characters silently matching the longest prefix),
/// - or the prefix matches more than one cluster (caller must
///   disambiguate). The error reproduces the caller's input so the
///   transport can surface it verbatim.
///
/// # Errors
///
/// Returns [`LiveError::UnknownCluster`] in the cases above.
pub fn resolve_cluster_by_id_prefix<'a>(
    clusters: &'a [ReportCluster],
    id: &str,
) -> Result<&'a ReportCluster, LiveError> {
    // Exact-match fast path keeps O(N) behaviour on the canonical id
    // and short-circuits before the prefix uniqueness scan so a full
    // 16-hex query never trips the ambiguity check.
    if let Some(cluster) = clusters.iter().find(|cluster| cluster.id == id) {
        return Ok(cluster);
    }
    if id.len() < MIN_CLUSTER_ID_PREFIX_LEN {
        return Err(LiveError::UnknownCluster { id: id.to_owned() });
    }
    let mut matches = clusters.iter().filter(|cluster| cluster.id.starts_with(id));
    let first = matches
        .next()
        .ok_or_else(|| LiveError::UnknownCluster { id: id.to_owned() })?;
    if matches.next().is_some() {
        tracing::warn!(
            prefix = %id,
            "cluster_by_id_prefix_ambiguous_returning_unknown",
        );
        return Err(LiveError::UnknownCluster { id: id.to_owned() });
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::{resolve_cluster_by_id_prefix, MIN_CLUSTER_ID_PREFIX_LEN};
    use crate::{live::errors::LiveError, report::ReportCluster, wire_generated::ReportSignals};

    /// Builds a `ReportCluster` with just the canonical id populated.
    /// Every other field is zeroed so the prefix-resolution helper is
    /// exercised against the minimum shape it needs. `ReportCluster`
    /// is wire-generated (no `Default` derive) so we spell out each
    /// field explicitly.
    fn cluster_with_id(id: &str) -> ReportCluster {
        let mut cluster = crate::report_fixtures::fixture_cluster(id, Vec::new());
        cluster.weight = 0.0;
        cluster.canonical_node_count = 0;
        cluster.signals = ReportSignals {
            structural: 0.0,
            token_jaccard: 0.0,
            shape: 0.0,
            embedding_cos: 0.0,
            agreement: 0.0,
            rename_consistency: 0.0,
            literal_fraction: 0.0,
        };
        crate::report_fixtures::restamp_fixture(&mut cluster);
        cluster
    }

    /// The 7-hex slug from the VSIX hover bubble must
    /// resolve to the same cluster as the 16-hex canonical id. Asserts
    /// against an explicit human-readable cluster id so a regression
    /// shows up as a string mismatch in the failure output.
    #[test]
    fn cluster_by_id_seven_hex_slug_resolves_to_same_cluster_as_full_id() -> Result<(), LiveError> {
        let canonical = "1802186abcdef012";
        let clusters = vec![
            cluster_with_id(canonical),
            cluster_with_id("ffffffffaaaaaaaa"),
            cluster_with_id("00112233aabbccdd"),
        ];
        let slug = &canonical[..MIN_CLUSTER_ID_PREFIX_LEN];
        assert_eq!(
            slug.len(),
            7,
            "the VSIX slug is exactly 7 hex characters; got {slug:?}",
        );
        let by_full = resolve_cluster_by_id_prefix(&clusters, canonical)?;
        let by_slug = resolve_cluster_by_id_prefix(&clusters, slug)?;
        assert_eq!(
            by_full.id,
            canonical,
            "full id must round-trip the canonical cluster: got {got}",
            got = by_full.id,
        );
        assert_eq!(
            by_slug.id,
            canonical,
            "slug {slug:?} must resolve to the same canonical cluster {canonical}: got {got}",
            got = by_slug.id,
        );
        Ok(())
    }

    /// A prefix shorter than [`MIN_CLUSTER_ID_PREFIX_LEN`] must be
    /// rejected so stray inputs cannot silently grab the longest match.
    #[test]
    fn cluster_by_id_prefix_shorter_than_slug_is_rejected() {
        let clusters = vec![cluster_with_id("1802186abcdef012")];
        let outcome = resolve_cluster_by_id_prefix(&clusters, "180218");
        assert!(
            matches!(outcome, Err(LiveError::UnknownCluster { ref id }) if id == "180218"),
            "expected UnknownCluster for short prefix; got {outcome:?}",
        );
    }

    /// An ambiguous prefix (matches more than one cluster) must fail
    /// closed rather than guessing a winner.
    #[test]
    fn cluster_by_id_ambiguous_prefix_returns_unknown() {
        let clusters = vec![
            cluster_with_id("1802186aaaa00001"),
            cluster_with_id("1802186bbbb00002"),
        ];
        let outcome = resolve_cluster_by_id_prefix(&clusters, "1802186");
        assert!(
            matches!(outcome, Err(LiveError::UnknownCluster { ref id }) if id == "1802186"),
            "expected UnknownCluster for ambiguous prefix; got {outcome:?}",
        );
    }
}
