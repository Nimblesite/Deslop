//! E2E regression for GH #224.
//!
//! Deslop clustered runs of **distinct** Rust struct field declarations —
//! different field names, different types — as duplicated code, because AST
//! normalization collapses identifiers, types, and literals. The normalized
//! Merkle hash of one serde field run
//! (`#[serde(skip_serializing_if = "Option::is_none")] pub x: Option<String>`,
//! repeated) is identical to that of an unrelated run with different field
//! names, so the two cluster as `structural_only` duplication. These are not
//! duplicated logic — they are different parts of a data model and no refactor
//! removes them, yet they dominated the duplication metric on serde-heavy
//! repos.
//!
//! Acceptance: distinct struct-field declaration runs must NOT be reported as
//! duplication (same boilerplate tier as the existing import / `using` /
//! Dart-class-field filters), while a *byte-identical* copy-pasted struct must
//! still surface — the raw-bytes-differ escape hatch that keeps genuine
//! copy-paste visible.

use crate::common::*;

/// `min-nodes` low enough to admit the per-struct and field-run subtrees the
/// fixture produces, matching the granularity issue #224 reports.
const MIN_NODES: u32 = 20;

#[test]
fn distinct_struct_field_runs_are_not_reported_as_duplication() -> Result<()> {
    let scan_root = fixture("rust-issue-224-struct-fields");
    let report = run_report(&scan_root, MIN_NODES)?;

    // The two serde-model files hold only distinct field-declaration runs:
    // after the fix no visible cluster may touch them.
    let leaked: Vec<String> = clusters(&report)
        .iter()
        .filter(|cluster| {
            occurrence_files(cluster)
                .iter()
                .any(|file| file == "host.rs" || file == "manifest.rs")
        })
        .map(|cluster| {
            format!(
                "cluster {id} bucket={bucket} files={files:?}",
                id = cluster_id(cluster),
                bucket = cluster_bucket(cluster),
                files = occurrence_files(cluster),
            )
        })
        .collect();
    assert!(
        leaked.is_empty(),
        "issue #224: distinct Rust struct-field declaration runs are different \
         parts of a data model, not extractable duplication, and must not be \
         reported. Offending clusters: {leaked:#?}\nfull report: {report:#}"
    );

    // The filter must stay targeted: a byte-identical copy-pasted struct is
    // genuine duplication and must still surface (the raw-bytes-differ escape
    // hatch). If this regresses, the filter is hiding real struct clones.
    let verbatim_surfaces = clusters(&report).iter().any(|cluster| {
        let files = occurrence_files(cluster);
        files.iter().any(|file| file == "verbatim_a.rs")
            && files.iter().any(|file| file == "verbatim_b.rs")
    });
    assert!(
        verbatim_surfaces,
        "a byte-identical copy-pasted struct must still surface as genuine \
         duplication (raw-bytes-differ escape hatch): {report:#}"
    );
    Ok(())
}
