//! Curated Type-2 recall asserts membership and extent, never cluster evidence.

use super::*;

/// Curated pair used by every case.
const PAIR: [&str; 2] = ["src/a.ts", "src/b.ts"];
/// Minimum normalised extent curated for the pair.
const MIN_NODES: u64 = 300;
/// Rank ceiling curated for the pair.
const CURATED_CEILING: u64 = 4;
/// A report position past [`CURATED_CEILING`].
const RANK_PAST_CEILING: u64 = 5;
/// A report position inside [`CURATED_CEILING`].
const RANK_WITHIN_CEILING: u64 = 3;

/// Manifest containing one hand-verified renamed pair.
fn manifest(min_nodes: Option<u64>, max_rank: Option<u64>) -> Value {
    json!({
        "must_find_type2": [{
            "files": PAIR,
            "min_nodes": min_nodes,
            "max_rank": max_rank,
            "why": "hand-verified renamed module pair"
        }]
    })
}

/// A pair verdict the content gate vouched for as a Type-2 rename.
fn vouched() -> Value {
    json!({
        "files": PAIR,
        "evidence": {
            "structural": 1.0,
            "content_required": true,
            "content_ok": true,
            "admitted": true,
            "classification": "nearly_identical"
        }
    })
}

/// The same pair, admitted without the content guard its route requires.
fn unvouched() -> Value {
    json!({
        "files": PAIR,
        "evidence": {
            "structural": 1.0,
            "content_required": true,
            "content_ok": false,
            "admitted": true,
            "classification": "structural_only"
        }
    })
}

/// Runs the curated Type-2 assertion with no curated rank ceiling.
fn judge(clusters: &[Value], min_nodes: Option<u64>) -> Vec<Failure> {
    judge_ranked(clusters, min_nodes, None)
}

/// Runs the curated Type-2 assertion against a curated rank ceiling.
fn judge_ranked(clusters: &[Value], min_nodes: Option<u64>, max_rank: Option<u64>) -> Vec<Failure> {
    judge_vouched(clusters, min_nodes, max_rank, &[vouched()])
}

/// Runs the curated Type-2 assertion against supplied pair verdicts.
fn judge_vouched(
    clusters: &[Value],
    min_nodes: Option<u64>,
    max_rank: Option<u64>,
    verdicts: &[Value],
) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_type2_curated_recall(
        &manifest(min_nodes, max_rank),
        &report(clusters),
        verdicts,
        &mut failures,
    );
    failures
}

/// Unrelated clusters filling `count` report positions from `first_rank`,
/// so a curated pair can be placed at a chosen position in report order.
fn padding(first_rank: u64, count: u64) -> Vec<Value> {
    (0..count)
        .map(|offset| {
            let rank = first_rank.saturating_add(offset);
            let left = format!("src/pad{rank}a.ts");
            let right = format!("src/pad{rank}b.ts");
            spanning(
                &format!("pad{rank}"),
                MIN_NODES,
                rank,
                &[left.as_str(), right.as_str()],
            )
        })
        .collect()
}

/// Report order placing the curated pair at `position` with `nodes` extent.
fn pair_at(position: u64, nodes: u64) -> Vec<Value> {
    let mut clusters = padding(1, position.saturating_sub(1));
    clusters.push(spanning("pair", nodes, position, &PAIR));
    clusters
}

#[test]
fn reported_pair_at_curated_extent_passes() {
    let cluster = spanning("pair", MIN_NODES, 1, &PAIR);
    assert!(judge(&[cluster], Some(MIN_NODES)).is_empty());
}

#[test]
fn missing_curated_pair_fails() {
    let cluster = spanning("elsewhere", MIN_NODES, 1, &["src/c.ts", "src/d.ts"]);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "a missing curated pair is a false negative",
        "src/a.ts",
        "the failure names the missing pair",
    );
}

#[test]
fn fragment_below_curated_extent_fails() {
    let cluster = spanning("fragment", MIN_NODES.saturating_sub(1), 1, &PAIR);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "a fragment is not the curated module clone",
        "expected at least 300",
        "the failure names the extent deficit",
    );
}

#[test]
fn missing_extent_curation_fails() {
    assert_only_failure(
        &judge(&[spanning("pair", MIN_NODES, 1, &PAIR)], None),
        "type2_recall",
        "a manifest without extent asserts too little",
        "min_nodes",
        "the failure names the missing field",
    );
}

#[test]
fn hidden_curated_occurrence_fails() {
    let cluster = hide_occurrence(spanning("pair", MIN_NODES, 1, &PAIR), PAIR[1]);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "both curated sides must be visible",
        "src/a.ts",
        "the failure names the pair",
    );
}

#[test]
fn unrelated_sprawl_below_extent_does_not_satisfy_recall() {
    let files = [PAIR[0], PAIR[1], "src/net.ts", "src/process.ts"];
    let cluster = spanning("boilerplate", 31, 1, &files);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "path overlap alone cannot satisfy curated recall",
        "31 canonical nodes",
        "the failure names the unrelated small extent",
    );
}

#[test]
fn extent_above_floor_passes() {
    let cluster = spanning("pair", MIN_NODES.saturating_add(80), 1, &PAIR);
    assert!(judge(&[cluster], Some(MIN_NODES)).is_empty());
}

#[test]
fn empty_curated_list_asserts_nothing() {
    let mut failures = Vec::new();
    check_type2_curated_recall(
        &json!({"must_find_type2": []}),
        &report(&[]),
        &[],
        &mut failures,
    );
    assert!(failures.is_empty());
}

/// [CORPUS-RECALL] Ranking is the product. A curated rename reported at
/// full extent but buried past its curated ceiling is a finding the user
/// never scrolls to — gh #439 witness 2 sat at rank 1628 of 2155 while
/// `type2_recall` stayed green, because the check read no rank at all.
#[test]
fn a_curated_pair_ranked_past_its_ceiling_is_not_recall() {
    assert_only_failure(
        &judge_ranked(
            &pair_at(RANK_PAST_CEILING, MIN_NODES),
            Some(MIN_NODES),
            Some(CURATED_CEILING),
        ),
        "type2_recall",
        "a curated rename buried past its curated ceiling is not recall",
        "ranks 5",
        "the failure names the rank the curated pair reached",
    );
}

/// The ceiling is inclusive, and a pair inside it is clean recall.
#[test]
fn a_curated_pair_inside_its_ceiling_passes() {
    assert!(judge_ranked(
        &pair_at(RANK_WITHIN_CEILING, MIN_NODES),
        Some(MIN_NODES),
        Some(CURATED_CEILING),
    )
    .is_empty());
}

/// The rank asserted must be the rank of the cluster that actually reaches
/// the curated extent. A sub-extent fragment ranking first must not answer
/// the ceiling for the module buried behind it — that is gh #439 witness 2
/// exactly, where a 39-node fragment stood in for the whole-module view.
#[test]
fn a_fragment_ranked_first_does_not_answer_the_ceiling_for_the_buried_module() {
    let mut clusters = vec![spanning("fragment", MIN_NODES.saturating_sub(1), 1, &PAIR)];
    clusters.extend(padding(2, RANK_PAST_CEILING.saturating_sub(2)));
    clusters.push(spanning("module", MIN_NODES, RANK_PAST_CEILING, &PAIR));
    assert_only_failure(
        &judge_ranked(&clusters, Some(MIN_NODES), Some(CURATED_CEILING)),
        "type2_recall",
        "the ceiling must be judged on the cluster that reaches the curated extent",
        "ranks 5",
        "the failure names the buried module's rank, not the fragment's",
    );
}

/// An entry curating no ceiling still asserts membership and extent — the
/// ceiling is optional per entry, because only a human-ranked pair earns a
/// rank assertion.
#[test]
fn an_entry_curating_no_ceiling_still_passes_on_extent() {
    assert!(judge(&pair_at(RANK_PAST_CEILING, MIN_NODES), Some(MIN_NODES)).is_empty());
}

/// [CORPUS-RECALL] Recall is not "a cluster of the right size spans the
/// right files" — it is the engine *admitting* that pair as a rename. A
/// cluster can span the curated paths at full extent while the content
/// gate never vouched for the relation, and until the mass-only wire
/// removed evidence from clusters this check said so (gh #488).
#[test]
fn a_curated_pair_the_content_gate_never_vouched_is_not_recall() {
    assert_only_failure(
        &judge_vouched(
            &[spanning("pair", MIN_NODES, 1, &PAIR)],
            Some(MIN_NODES),
            None,
            &[unvouched()],
        ),
        "type2_recall",
        "an unvouched pair is not a reported rename, however large the cluster",
        "structural_only",
        "the failure names the classification the engine actually reached",
    );
}

/// A verdict the gate never obtained asserts nothing, and must fail rather
/// than pass — the stance [CORPUS-SCOPE] takes on a missing bound.
#[test]
fn a_curated_pair_with_no_verdict_fails_rather_than_passing() {
    assert_only_failure(
        &judge_vouched(
            &[spanning("pair", MIN_NODES, 1, &PAIR)],
            Some(MIN_NODES),
            None,
            &[],
        ),
        "type2_recall",
        "no verdict means the evidence clause judged nothing",
        "no admission evidence",
        "the failure says the verdict is missing",
    );
}

/// And a pair the gate did vouch for passes every clause.
#[test]
fn a_vouched_pair_at_extent_passes() {
    assert!(judge_vouched(
        &[spanning("pair", MIN_NODES, 1, &PAIR)],
        Some(MIN_NODES),
        None,
        &[vouched()],
    )
    .is_empty());
}
