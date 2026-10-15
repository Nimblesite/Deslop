//! [RANK-STRUCTURAL-ONLY] Informational families carry no duplicate mass or rank.

use super::*;

const KIND_FIELD: &str = "kind";
const INFORMATIONAL_KIND: &str = "structural_only";
const INFO_ID: &str = "shape-family";
const INFO_NODES: u64 = 40;
const INFO_FILES: [&str; 2] = ["shape-a.rs", "shape-b.rs"];
const NO_RANK: u64 = 0;
const NO_MASS: u64 = 0;
const CLONE_ID: &str = "copy-family";
const CLONE_FILES: [&str; 2] = ["copy-a.rs", "copy-b.rs"];
const FIRST_CLONE_RANK: u64 = 1;
const CLAIMED_RANK: u64 = 2;

fn information() -> Result<Value> {
    let mut cluster = spanning(INFO_ID, INFO_NODES, NO_RANK, &INFO_FILES);
    set_field(&mut cluster, KIND_FIELD, json!(INFORMATIONAL_KIND))?;
    set_field(&mut cluster, MASS, json!(NO_MASS))?;
    Ok(cluster)
}

#[test]
fn informational_family_carries_zero_duplicate_mass_and_no_clone_rank() -> Result<()> {
    let clone = spanning(CLONE_ID, INFO_NODES, FIRST_CLONE_RANK, &CLONE_FILES);
    let failures = judge(&[clone, information()?]);
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[test]
fn informational_family_cannot_claim_positive_duplicate_mass() -> Result<()> {
    let mut cluster = information()?;
    set_field(&mut cluster, MASS, json!(INFO_NODES))?;
    let failures = judge(&[cluster]);
    assert_eq!(only_check(&failures), Some(MASS_CHECK), "{failures:?}");
    Ok(())
}

#[test]
fn informational_family_cannot_claim_a_clone_rank() -> Result<()> {
    let mut cluster = information()?;
    set_field(&mut cluster, RANK, json!(CLAIMED_RANK))?;
    let failures = judge(&[cluster]);
    assert_eq!(only_check(&failures), Some(RANK_CHECK), "{failures:?}");
    Ok(())
}
