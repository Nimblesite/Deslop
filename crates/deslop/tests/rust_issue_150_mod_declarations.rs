//! E2E regression for GH #150: `pub(crate) mod e0001;` /
//! `pub use foo::bar;` top-level declarations cluster across registries
//! because Rust requires literal module statements. These are scaffolding,
//! not actionable duplication, and must never surface in the report.
//! Spec: [CLONE-NOISE-RUST-DECL].

use anyhow::Result;

use crate::common::*;

/// The node floor `mod`/`use` runs are judged at — low enough that a
/// declaration run would cluster if it were not filtered.
const MOD_DECLARATION_MIN_NODES: u32 = 3;

#[test]
fn rust_mod_and_use_declarations_do_not_cluster_as_duplicates() -> Result<()> {
    let report = run_report(
        &fixture("rust-issue-150-mod-declarations"),
        MOD_DECLARATION_MIN_NODES,
    )?;
    let count = cluster_count(&report);
    assert_eq!(
        count, 0,
        "pub(crate) mod / pub use declarations are language scaffolding \
         and must not surface as duplicate clusters: {report:#}"
    );
    Ok(())
}
