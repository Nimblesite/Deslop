//! Mass-only cluster ordering ([RANK-MASS-SUM]).

use crate::report::ReportCluster;

/// Sorts clusters by mass descending and stable id ascending, then stamps rank.
pub(crate) fn rank_by_mass(clusters: &mut [ReportCluster]) {
    clusters.sort_by(|left, right| {
        right
            .mass
            .cmp(&left.mass)
            .then_with(|| left.id.cmp(&right.id))
    });
    stamp_ranks(clusters);
}

/// Stamps repository-global rank and its engine-authored visual band.
pub(crate) fn stamp_ranks(clusters: &mut [ReportCluster]) {
    let total = clusters.len();
    for (index, cluster) in clusters.iter_mut().enumerate() {
        let rank = index.saturating_add(1);
        cluster.rank = rank;
        rank_band(rank, total).clone_into(&mut cluster.rank_band);
    }
}

/// Returns the band for a one-based repository-global rank.
fn rank_band(rank: usize, total: usize) -> &'static str {
    if rank <= total.div_ceil(100) {
        "worst"
    } else if rank <= total.div_ceil(10) {
        "top10"
    } else if rank <= total.div_ceil(2) {
        "mid"
    } else {
        "faint"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIGH_MASS: u64 = 20;
    const LOW_MASS: u64 = 10;
    const FIRST_ID: &str = "a";
    const SECOND_ID: &str = "b";

    #[test]
    fn mass_and_id_are_the_only_ordering_inputs() {
        let mut clusters = [cluster(SECOND_ID, LOW_MASS), cluster(FIRST_ID, HIGH_MASS)];
        rank_by_mass(&mut clusters);
        assert_eq!(clusters[0].id, FIRST_ID);
        assert_eq!(clusters[0].rank, 1);
        assert_eq!(clusters[1].id, SECOND_ID);
        assert_eq!(clusters[1].rank, 2);
    }

    #[test]
    fn id_breaks_equal_mass_ties() {
        let mut clusters = [cluster(SECOND_ID, HIGH_MASS), cluster(FIRST_ID, HIGH_MASS)];
        rank_by_mass(&mut clusters);
        assert_eq!(clusters[0].id, FIRST_ID);
        assert_eq!(clusters[1].id, SECOND_ID);
    }

    fn cluster(id: &str, mass: u64) -> ReportCluster {
        ReportCluster {
            id: id.to_owned(),
            rank: 0,
            rank_band: String::new(),
            mass,
            canonical_node_count: 1,
            occurrences: Vec::new(),
            occurrences_total: 0,
            occurrence_count: 0,
            occurrences_truncated: false,
            intersects_diff: None,
            is_newly_introduced: None,
        }
    }
}
