//! [CLONE-NOISE-VERBATIM-SUBGROUP] E2E coverage for a convicted noise component that contains a byte-identical call family. The family survives whether its copies occupy one file or two; only the literal-varying stranger is dropped.

use std::{fs, path::PathBuf};

use serde_json::Value;

use crate::common::{signals::*, verbatim_subgroup::*, *};

/// The intra-file half: one file holding the run twice, plus the
/// literal-varying stranger that makes the component a noise family.
const GEOMETRY_INTRA_CASE: &str = "idiom-price";

/// The one file holding the doubled run, and the stranger.
const GEOMETRY_INTRA_FILES: u64 = 2;

/// The two copies and the same stranger, in three files.
const GEOMETRY_CROSS_FILES: u64 = 3;

/// The surviving family replaces the convicted component rather than adding a hidden report row.
const GEOMETRY_INTRA_HIDDEN: u64 = 0;

/// The one qualifying byte-identical family rendered from the component.
const GEOMETRY_INTRA_VISIBLE: usize = 1;

/// The literal-varying stranger contributes no duplicated lines.
const STRANGER_DUPLICATED_LOC: u64 = 0;

/// Both intra-file copy occurrences contribute their five copied lines.
const GEOMETRY_INTRA_DUPLICATED_LOC: u64 = 10;

/// What every failure message here names itself as.
const GEOMETRY_LABEL: &str = "[CLONE-NOISE-VERBATIM-SUBGROUP] verbatim-family geometry";

/// The signals a byte-identical copy saturates whatever its literal
/// density.
///
/// `literal_fraction` is deliberately absent. This run is literal-dense
/// — it measures 0.62 — so `TYPE1_IDENTICAL_SIGNALS`, which pins that
/// figure at 0.0, describes the literal-poor `settle_ledger` controls and
/// not this copy. Naming the saturating six keeps the strong half strong
/// without claiming the wrong thing about the seventh.
/// Every Python source byte in one `verbatim-subgroup` case,
/// concatenated in ascending file-name order — the whole corpus as one
/// string, so two corpora can be compared for equality however many
/// files each spreads itself over.
fn corpus_source(case: &str) -> Result<String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(fixture("verbatim-subgroup").join(case))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "py"))
        .collect();
    paths.sort();
    let mut source = String::new();
    for path in paths {
        source.push_str(&fs::read_to_string(path)?);
    }
    Ok(source)
}

/// The intra-file half: the convicted filter replaces its component with the qualifying copy family.
fn assert_intra_file_copy_survives(report: &Value) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(GEOMETRY_INTRA_FILES),
        "{GEOMETRY_LABEL}: the copy was analysed and decided *against*, not \
         skipped — a file the scan never opened proves nothing about the \
         arbitration: {report:#}"
    );
    assert_eq!(
        clusters(report).len(),
        GEOMETRY_INTRA_VISIBLE,
        "{GEOMETRY_LABEL}: a convicted component renders its qualifying byte-identical family: {published:#?}",
        published = published(report),
    );
    let copy = clusters(report)
        .iter()
        .find(|cluster| cluster_file_set(cluster) == [CALL_ORIGIN.to_owned()].into())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{GEOMETRY_LABEL}: the surviving intra-file family is missing: {report:#}"
            )
        })?;
    assert_eq!(
        occurrences(copy).len(),
        2,
        "{GEOMETRY_LABEL}: the surviving family contains both byte-identical occurrences"
    );
    assert!(
        has_verbatim_pair(
            &fixture("verbatim-subgroup").join(GEOMETRY_INTRA_CASE),
            copy
        )?,
        "{GEOMETRY_LABEL}: the retained family must be byte-proven: {copy:#}"
    );
    assert_structural_only_contract(copy, GEOMETRY_LABEL);
    assert_no_pair_surface_on_cluster(copy, GEOMETRY_LABEL);
    assert_eq!(
        clusters_hidden(report),
        GEOMETRY_INTRA_HIDDEN,
        "{GEOMETRY_LABEL}: replacement does not add a hidden row: {report:#}"
    );
    assert_eq!(
        duplicated_loc_for(report, CALL_ORIGIN),
        GEOMETRY_INTRA_DUPLICATED_LOC,
        "{GEOMETRY_LABEL}: both visible intra-file copies contribute their five copied lines: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
    assert_eq!(
        duplicated_loc_for(report, CALL_STRANGER),
        STRANGER_DUPLICATED_LOC,
        "{GEOMETRY_LABEL}: the literal-varying stranger remains uncharged"
    );
    Ok(())
}

/// The cross-file half: the same bytes, split across two files, are proof
/// of copying — and the report says so, first and in full.
fn assert_cross_file_copy_is_published(report: &Value) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(GEOMETRY_CROSS_FILES),
        "{GEOMETRY_LABEL}: the same source, now spread over three files: {report:#}"
    );
    assert_copy_survives_alone(report, GEOMETRY_LABEL, &CALL_COPY, CALL_STRANGER)?;
    let copy = expect_cluster_spanning(report, &CALL_COPY)?;
    assert_copy_is_saturated(report, copy)?;
    assert_cross_file_metrics_charge_only_the_copy(report);
    Ok(())
}

/// A byte-proven copy heads the report ([RANK-SCORE]) and is
/// byte-proven from the source.
fn assert_copy_is_saturated(report: &Value, copy: &Value) -> Result<()> {
    assert!(
        has_verbatim_pair(&fixture("verbatim-subgroup").join(CALL_CASE), copy)?,
        "{GEOMETRY_LABEL}: the two copies are byte-identical and must be \
         byte-proven — {dump}",
        dump = signal_dump(copy),
    );
    assert_eq!(
        rank_of(report, copy)?,
        0,
        "{GEOMETRY_LABEL}: a byte-for-byte copy is the strongest finding in the \
         run and heads the report — a reader must not have to scroll past weaker \
         evidence to reach it: {published:#?}",
        published = published(report),
    );
    Ok(())
}

/// The metric half of the published side: exactly the copied lines, in
/// exactly the copied files, and nothing for the stranger.
fn assert_cross_file_metrics_charge_only_the_copy(report: &Value) {
    for file in CALL_COPY {
        assert_eq!(
            duplicated_loc_for(report, file),
            CALL_LOC_PER_FILE,
            "{GEOMETRY_LABEL}: {file} holds {CALL_LOC_PER_FILE} copied lines and \
             must be charged for every one of them: {lines:#?}",
            lines = visible_cluster_lines(report),
        );
    }
    assert_eq!(
        duplicated_loc_for(report, CALL_STRANGER),
        STRANGER_DUPLICATED_LOC,
        "{GEOMETRY_LABEL}: {CALL_STRANGER} varies its literals — it is the family \
         the filter exists to suppress, and it earns nothing on either side of \
         the A/B: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
}

// [CLONE-NOISE-VERBATIM-SUBGROUP] The source is identical across the two layouts; both qualifying families survive.
#[test]
fn a_verbatim_family_survives_a_convicted_filter_in_any_file_geometry() -> Result<()> {
    assert_eq!(
        corpus_source(GEOMETRY_INTRA_CASE)?,
        corpus_source(CALL_CASE)?,
        "the A/B is only an argument while both halves hold the same source — \
         once the bytes differ, the opposite outcomes below stop being about file \
         geometry and this test stops proving anything"
    );
    assert_intra_file_copy_survives(&render(GEOMETRY_INTRA_CASE, MIN_NODES)?)?;
    assert_cross_file_copy_is_published(&render(CALL_CASE, MIN_NODES)?)
}
