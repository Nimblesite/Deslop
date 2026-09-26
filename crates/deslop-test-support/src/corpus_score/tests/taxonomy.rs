//! [CORPUS-SCORE] Informational findings cannot answer judged clone pairs.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    first_entry, occurrence, register, score_repo, the_pair, CLEARLY_IN, CLEARLY_OUT, FIRST_RANGE,
    PATH, REPO, SECOND_RANGE,
};

const INFORMATION_ID: &str = "information";
const CLONE_ID: &str = "clone";
const STRUCTURAL_ONLY: &str = "structural_only";
const UNKNOWN_KIND: &str = "future_unknown";
const EXTRA_START: u64 = 50;
const EXTRA_END: u64 = 60;

fn unproven_clone_reports() -> [Value; 5] {
    [
        json!({"clusters": [{
            "id": INFORMATION_ID,
            "bucket": STRUCTURAL_ONLY,
            "occurrences": the_pair(),
        }]}),
        json!({"clusters": [{
            "id": INFORMATION_ID,
            "kind": STRUCTURAL_ONLY,
            "occurrences": the_pair(),
        }]}),
        json!({"clusters": [{
            "id": INFORMATION_ID,
            "occurrences": the_pair(),
        }]}),
        json!({"clusters": [{
            "id": INFORMATION_ID,
            "kind": super::IDENTICAL_KIND,
            "bucket": STRUCTURAL_ONLY,
            "occurrences": the_pair(),
        }]}),
        json!({"clusters": [{
            "id": INFORMATION_ID,
            "kind": UNKNOWN_KIND,
            "occurrences": the_pair(),
        }]}),
    ]
}

fn assert_pair_status(report: &Value, found: bool, cluster_id: Option<&str>) -> Result<()> {
    let ranges = [FIRST_RANGE, SECOND_RANGE];
    let recall = score_repo(REPO, &register(CLEARLY_IN, &ranges), report)?;
    let precision = score_repo(REPO, &register(CLEARLY_OUT, &ranges), report)?;
    assert_eq!(recall.clearly_in_found, usize::from(found));
    assert_eq!(recall.false_negatives, usize::from(!found));
    assert_eq!(precision.clearly_out_absent, usize::from(!found));
    assert_eq!(precision.false_positives, usize::from(found));
    assert_eq!(first_entry(&recall)?.matched, found);
    assert_eq!(first_entry(&precision)?.matched, found);
    assert_eq!(first_entry(&recall)?.cluster.as_deref(), cluster_id);
    assert_eq!(first_entry(&precision)?.cluster.as_deref(), cluster_id);
    Ok(())
}

fn informational_family() -> Value {
    let mut occurrences = the_pair();
    occurrences.push(occurrence(PATH, EXTRA_START, EXTRA_END, false));
    json!({"clusters": [{
        "id": INFORMATION_ID,
        "kind": STRUCTURAL_ONLY,
        "occurrences": occurrences,
    }]})
}

fn add_clone(report: &mut Value) -> Result<()> {
    let clone = json!({"id": CLONE_ID, "kind": super::IDENTICAL_KIND, "occurrences": the_pair()});
    report
        .get_mut("clusters")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow!("fixture has no cluster list"))?
        .push(clone);
    Ok(())
}

/// Old `bucket` and current `kind` schemas must both prove a clone kind.
#[test]
fn informational_findings_cannot_satisfy_either_verdict() -> Result<()> {
    for report in unproven_clone_reports() {
        let score = score_repo(
            REPO,
            &register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]),
            &report,
        )?;
        assert_eq!(score.clusters_total, 1);
        assert_pair_status(&report, false, None)?;
    }
    Ok(())
}

/// A broad informational family cannot certify one of its apparent pairs;
/// the separately published clone group can.
#[test]
fn only_a_clone_finding_certifies_a_pair_inside_a_mixed_family() -> Result<()> {
    let mut mixed = informational_family();
    assert_pair_status(&mixed, false, None)?;
    add_clone(&mut mixed)?;
    assert_pair_status(&mixed, true, Some(CLONE_ID))?;
    Ok(())
}
