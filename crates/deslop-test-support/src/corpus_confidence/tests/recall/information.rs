//! [CORPUS-SCORE] A curated clone requires a published clone classification.

use super::*;

const KIND_FIELD: &str = "kind";
const EXACT_FIELD: &str = "must_find";
const RENAMED_FIELD: &str = "must_find_type2";
const FILES_FIELD: &str = "files";
const NODES_FIELD: &str = "min_nodes";
const EXPECTED_CHECKS: [&str; 2] = ["recall", "type2_recall"];
const REJECTED_LABELS: [&str; 2] = ["structural_only", "unknown-kind"];
const VERIFIED_LABEL: &str = "identical";
const FAMILY_ID: &str = "curated-family";
const FAMILY_NODES: u64 = 300;
const EXTENT_FLOOR: u64 = 200;
const FIRST_RANK: u64 = 1;
const NO_QUANTITY: u64 = 0;
const PRECISION_FIELD: &str = "must_not_cluster";
const PRECISION_CHECK: &str = "precision";

fn family_report(label: &str) -> Result<Value> {
    let mut cluster = spanning(FAMILY_ID, FAMILY_NODES, FIRST_RANK, &PAIR);
    set_field(&mut cluster, KIND_FIELD, json!(label))?;
    if label != VERIFIED_LABEL {
        set_field(&mut cluster, MASS, json!(NO_QUANTITY))?;
        set_field(&mut cluster, RANK, json!(NO_QUANTITY))?;
    }
    Ok(report(&[cluster]))
}

fn judge_both(label: &str) -> Result<Vec<Failure>> {
    let entry = json!({FILES_FIELD: PAIR, NODES_FIELD: EXTENT_FLOOR});
    let manifest = json!({EXACT_FIELD: [entry.clone()], RENAMED_FIELD: [entry]});
    let report = family_report(label)?;
    let mut failures = Vec::new();
    check_curated_recall(&manifest, &report, &mut failures);
    check_type2_curated_recall(&manifest, &report, &mut failures);
    Ok(failures)
}

fn judge_precision(label: &str) -> Result<Vec<Failure>> {
    let manifest = json!({PRECISION_FIELD: [{FILES_FIELD: PAIR}]});
    let mut failures = Vec::new();
    crate::corpus_precision::check_curated_precision(
        &manifest,
        &family_report(label)?,
        &mut failures,
    );
    Ok(failures)
}

#[test]
fn informational_and_unknown_families_cannot_breach_curated_clone_precision() -> Result<()> {
    assert_eq!(
        only_check(&judge_precision(VERIFIED_LABEL)?),
        Some(PRECISION_CHECK)
    );
    for label in REJECTED_LABELS {
        let failures = judge_precision(label)?;
        assert!(failures.is_empty(), "{label}: {failures:?}");
    }
    Ok(())
}

#[test]
fn informational_and_unknown_families_cannot_answer_curated_clone_recall() -> Result<()> {
    assert!(judge_both(VERIFIED_LABEL)?.is_empty());
    for label in REJECTED_LABELS {
        let failures = judge_both(label)?;
        let checks: Vec<_> = failures
            .iter()
            .map(|failure| failure.check.as_str())
            .collect();
        assert_eq!(checks, EXPECTED_CHECKS, "{label}: {failures:?}");
    }
    Ok(())
}
