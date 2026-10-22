//! [CORPUS-SCORE] A curated clone requires a published clone classification.

use super::*;

const KIND_FIELD: &str = "kind";
const EXACT_FIELD: &str = "must_find";
const RENAMED_FIELD: &str = "must_find_type2";
const FILES_FIELD: &str = "files";
const NODES_FIELD: &str = "min_nodes";
const EXPECTED_CHECKS: [&str; 2] = ["recall", "type2_recall"];
const INFORMATIONAL_LABEL: &str = "structural_only";
const UNKNOWN_LABEL: &str = "unknown-kind";
const REJECTED_LABELS: [&str; 2] = [INFORMATIONAL_LABEL, UNKNOWN_LABEL];
/// How a breach names a family that carries no classification at all.
const UNCLASSIFIED: &str = "unclassified";
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

/// The family with its classification field removed, as a renamed or dropped label would leave it.
fn unclassified_report() -> Result<Value> {
    let mut cluster = spanning(FAMILY_ID, FAMILY_NODES, FIRST_RANK, &PAIR);
    let _label = cluster
        .as_object_mut()
        .and_then(|fields| fields.remove(KIND_FIELD))
        .ok_or_else(|| anyhow::anyhow!("the fixture cluster carries a {KIND_FIELD} field"))?;
    Ok(report(&[cluster]))
}

fn judge_precision(report: &Value) -> Vec<Failure> {
    let manifest = json!({PRECISION_FIELD: [{FILES_FIELD: PAIR}]});
    let mut failures = Vec::new();
    crate::corpus_precision::check_curated_precision(&manifest, report, &mut failures);
    failures
}

/// [CORPUS-PRECISION-CURATED] Only a family positively classified as
/// informational is excused. A missing or unrecognised classification fails
/// the check, so a renamed label can never silence it.
#[test]
fn only_an_informational_family_is_excused_from_curated_precision() -> Result<()> {
    for label in [VERIFIED_LABEL, UNKNOWN_LABEL] {
        let failures = judge_precision(&family_report(label)?);
        assert_eq!(
            only_check(&failures),
            Some(PRECISION_CHECK),
            "{label}: {failures:?}"
        );
        assert!(
            detail_mentions(&failures, FAMILY_ID) && detail_mentions(&failures, label),
            "{label}: the breach names the family and its classification: {failures:?}"
        );
    }
    let unclassified = judge_precision(&unclassified_report()?);
    assert_eq!(
        only_check(&unclassified),
        Some(PRECISION_CHECK),
        "{unclassified:?}"
    );
    assert!(
        detail_mentions(&unclassified, FAMILY_ID) && detail_mentions(&unclassified, UNCLASSIFIED),
        "the breach says the family is unclassified: {unclassified:?}"
    );
    let informational = judge_precision(&family_report(INFORMATIONAL_LABEL)?);
    assert!(informational.is_empty(), "{informational:?}");
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
