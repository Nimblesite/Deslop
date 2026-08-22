//! E2E regression for GH #115 [CLONE-NOISE-PY-PYDANTIC-PARTIAL].
//!
//! Pydantic's create/update pattern declares `XCreate(BaseModel)` with
//! required fields and `XUpdate(BaseModel)` mirroring the same fields
//! with every annotation wrapped in `T | None = None`. Pydantic has no
//! native `PartialModel`, so this mirror is unavoidable and shows up
//! as a cluster after identifier normalisation. The cluster filter
//! must drop those `*Create` / `*Update` mirrors.

use crate::common::*;

#[test]
fn pydantic_create_update_mirrors_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-115-pydantic-partial");
    let report = run_report(&scan_root, 4)?;
    let offenders = summaries_where(&report, &scan_root, |text| {
        text.contains("BaseModel") || text.contains("| None = None")
    })?;
    assert!(
        offenders.is_empty(),
        "Pydantic `*Create` / `*Update` partial-mirror pairs must not \
         surface as duplicate logic — the mirror is mandated by the \
         framework: {offenders:#?}"
    );
    Ok(())
}
