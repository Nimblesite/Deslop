//! E2E regression for GH #115 [CLONE-NOISE-PY-STRENUM-CLASS-SHAPE].
//!
//! Different `class X(StrEnum)` declarations carry the same AST shape
//! — docstring + member assignments — even though every enum encodes a
//! distinct closed discriminator. After identifier normalisation they
//! cluster as duplicates. The cluster filter must drop them.


use crate::common::*;

#[test]
fn strenum_class_shapes_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-115-strenum");
    let report = run_report(&scan_root, 4)?;
    let offenders = summaries_where(&report, &scan_root, |text| text.contains("StrEnum"))?;
    assert!(
        offenders.is_empty(),
        "`class X(StrEnum)` declarations must not surface as duplicate \
         logic — each enum is a closed discriminator: {offenders:#?}"
    );
    let visible = visible_cluster_lines(&report);
    assert!(
        visible.is_empty(),
        "every line of this fixture is a `StrEnum` declaration, so a \
         visible cluster over any of it — member blocks and single member \
         lines included — reports the closed discriminators the filter \
         must suppress, merely windowed below the `class` keyword the \
         marker check looks for: {visible:#?}"
    );
    Ok(())
}
