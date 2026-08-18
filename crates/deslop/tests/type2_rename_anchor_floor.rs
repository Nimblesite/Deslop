//! A maximal Type-2 rename with almost no literal anchors is still a
//! Type-2 clone ([FUSION-CONTENT-GATE], [FUSED-THRESHOLD],
//! [RANK-STRUCTURAL-ONLY], [TECH-PMATCH-BAKER]).
//!
//! `fused_golden_bands.rs` pins the Type-2 band — "the load-bearing one.
//! A rename-only copy is the textbook definition of a Type-2 clone and
//! every clone detector must report it." This suite pins the band at the
//! anchor-starved end, where the shipped engine once manufactured a
//! false negative: `content::pair_rename_consistency` used to return
//! `0.0` outright below a four-literal-anchor cliff, the gate collapsed
//! to the agreement term, and this exact fixture rendered
//! `structural 1.00 / token_jaccard 1.00 / fused 0.0588`, bucket
//! `structural_only` — inside the `< 0.6` band in which `CLAUDE.md`
//! instructs an agent to **write the copy anyway**. Rename evidence is
//! now Baker's parameterized match quantified — corroborated
//! substitutions and preserved literals as smooth anchor mass — so
//! scarce anchors weaken the proof instead of erasing it.
//!
//! The fixture: one loop, one accumulator, one multiplication, identical
//! logic on both sides, every identifier renamed, and exactly one
//! literal (`0`). The assertions below are the same contract
//! `fused_golden_bands.rs` holds every language's rename scenarios to.
//! Nothing here is specific to the literal count; that is the point.

use serde_json::Value;

mod common;
use crate::common::{signals::*, *};

/// Node floor matching the golden-band suites, so the renamed function
/// subtree qualifies as a candidate.
const MIN_NODES: u32 = 12;

/// The fixture's two sides, renamed maximally from one another.
const SIDES: [&str; 2] = ["invoice.ts", "charge.ts"];

/// The one cluster spanning both sides of the rename.
fn rename_cluster(report: &Value) -> Result<&Value> {
    clusters(report)
        .iter()
        .find(|cluster| {
            SIDES.iter().all(|side| {
                occurrences(cluster)
                    .iter()
                    .any(|occurrence| occurrence_path(occurrence).ends_with(side))
            })
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no rendered cluster spans both sides of the rename; \
                 a Type-2 clone that reaches no visible cluster is a false negative"
            )
        })
}

/// An occurrence's reported path.
fn occurrence_path(occurrence: &Value) -> &str {
    occurrence
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

#[test]
fn a_maximal_rename_with_few_literals_is_still_a_type2_clone() -> Result<()> {
    let root = fixture("type2-rename-few-literals");
    let report = run_report(&root, MIN_NODES)?;
    let cluster = rename_cluster(&report)?;
    let dump = signal_dump(cluster);

    assert_eq!(
        cluster_size(cluster),
        2,
        "the rename has exactly two occurrences — {dump}"
    );
    assert_eq!(
        distinct_texts(&root, cluster)?.len(),
        2,
        "the two occurrences must differ in raw bytes, or this is a Type-1 copy — {dump}"
    );
    assert!(
        occurrences(cluster)
            .iter()
            .all(|occurrence| occurrence.get("hidden") != Some(&Value::Bool(true))),
        "a proven Type-2 clone may not have a hidden occurrence — {dump}"
    );

    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "identifier normalisation makes a rename structurally identical — {dump}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "the normalised k-gram stream is rename-invariant by construction — {dump}"
    );

    assert!(
        !HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(cluster)),
        "a Type-2 rename of real logic is duplication, not shape-only evidence — \
         demoting it is a false negative — {dump}"
    );
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "same shape, same logic, renamed identifiers is the textbook \
         `nearly_identical` clone — {dump}"
    );

    let fused = signal(cluster, "fused");
    assert!(
        fused >= REUSE_FUSED,
        "a renamed copy of real logic must stay at or above the reuse-bias line \
         ({REUSE_FUSED}) — below it the agent recipe tells the agent to write the \
         copy anyway — {dump}"
    );
    assert!(
        fused < 1.0,
        "only a byte-identical copy may saturate the confidence — {dump}"
    );
    Ok(())
}
