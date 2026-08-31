//! [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] — the price of the idiom
//! proof, charged where a reader can see it.
//!
//! `docs/specs/noise.md` records what the cross-file arbitration
//! knowingly gives up: *"a genuine intra-file byte-identical copy sitting
//! inside a component the filters suppressed stays hidden; that is the
//! price of the idiom proof, paid once, visibly, in the pins."*
//!
//! A pin that only shows the hidden side cannot charge that price. "The
//! family was suppressed and nothing published" is satisfied just as well
//! by a detector that never found the copy at all, so the assertion says
//! nothing about the arbitration — it says the report was empty.
//!
//! So this suite runs the **same source bytes twice** and changes one
//! thing: how many files they are spread across. It proves the byte
//! equality rather than assuming it, then asserts the two opposite
//! outcomes:
//!
//! - **cross-file** — byte-identity is proof of copying, because
//!   independently authored code does not coincide byte for byte. The
//!   copy publishes, saturated, ranked first, and charged to the
//!   duplication gate.
//! - **intra-file** — byte-identity is proof of the *idiom* the filter
//!   has just recognised. The copy is hidden and charged nothing.
//!
//! Those two outcomes over identical bytes *are* the arbitration. A
//! change that collapses them into one answer fails here whichever answer
//! it picks, and a detector that has gone blind fails the cross-file half
//! outright.

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

/// Components suppressed in the intra-file half: the one call family.
const GEOMETRY_INTRA_HIDDEN: u64 = 1;

/// Clusters the intra-file half may publish. The copy is real and the
/// tool saw it; the arbitration still declines to report it.
const NOTHING_PUBLISHED: usize = 0;

/// Duplicated lines a copy the report will not show may contribute.
const NOTHING_DUPLICATED: u64 = 0;

/// What every failure message here names itself as.
const GEOMETRY_LABEL: &str = "[CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] idiom-proof price";

/// The signals a byte-identical copy saturates whatever its literal
/// density.
///
/// `literal_fraction` is deliberately absent. This run is literal-dense
/// — it measures 0.62 — so `TYPE1_IDENTICAL_SIGNALS`, which pins that
/// figure at 0.0, describes the literal-poor `settle_ledger` controls and
/// not this copy. Naming the saturating six keeps the strong half strong
/// without claiming the wrong thing about the seventh.
const DETERMINED_SIGNALS: [(&str, f64); 6] = [
    ("structural", 1.0),
    ("token_jaccard", 1.0),
    ("shape", 1.0),
    ("embedding_cos", 0.0),
    ("pair_agreement", 1.0),
    ("pair_rename_consistency", 1.0),
];

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

/// The intra-file half: the copy is real, the tool saw it, and the report
/// declines to show it. That is the price, stated as a value.
fn assert_intra_file_copy_stays_hidden(report: &Value) {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(GEOMETRY_INTRA_FILES),
        "{GEOMETRY_LABEL}: the copy was analysed and decided *against*, not \
         skipped — a file the scan never opened proves nothing about the \
         arbitration: {report:#}"
    );
    assert_eq!(
        clusters(report).len(),
        NOTHING_PUBLISHED,
        "{GEOMETRY_LABEL}: the copy sits inside a component the literal-variation \
         filter suppressed, and an intra-file family never reaches the verbatim \
         hatch — so nothing may publish: {published:#?}",
        published = published(report),
    );
    assert_eq!(
        clusters_hidden(report),
        GEOMETRY_INTRA_HIDDEN,
        "{GEOMETRY_LABEL}: the component must be hidden *and counted*. Zero means \
         the family was never recognised — an empty report for the wrong reason — \
         and more means the filter reached past the one component this corpus \
         stages: {report:#}"
    );
    assert_intra_file_metrics_are_empty(report);
}

/// The metric half of the price: a copy the report will not show may not
/// reach the duplication gate either, in any file or in the total.
fn assert_intra_file_metrics_are_empty(report: &Value) {
    for file in [CALL_ORIGIN, CALL_STRANGER] {
        assert_eq!(
            duplicated_loc_for(report, file),
            NOTHING_DUPLICATED,
            "{GEOMETRY_LABEL}: {file} earns nothing while the copy inside it is \
             hidden — a line no reader can see in the report may not be charged \
             in the CI gate: {lines:#?}",
            lines = visible_cluster_lines(report),
        );
    }
    assert_eq!(
        visible_duplicated_loc(report),
        NOTHING_DUPLICATED,
        "{GEOMETRY_LABEL}: nothing is published, so nothing is duplicated: \
         {metrics:#}",
        metrics = field(report, "metrics"),
    );
}

/// The cross-file half: the same bytes, split across two files, are proof
/// of copying — and the report says so, first and in full.
fn assert_cross_file_copy_is_published(report: &Value) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(GEOMETRY_CROSS_FILES),
        "{GEOMETRY_LABEL}: the same source, now spread over three files: {report:#}"
    );
    assert_copy_survives_alone(
        report,
        GEOMETRY_LABEL,
        &CALL_COPY,
        CALL_STRANGER,
        rename_consistency_for(CALL_PAIR_ANCHORS),
    )?;
    let copy = expect_cluster_spanning(report, &CALL_COPY)?;
    assert_copy_is_saturated(report, copy)?;
    assert_cross_file_metrics_charge_only_the_copy(report);
    Ok(())
}

/// Every axis a byte-proven copy is measured on reads full, and the copy
/// heads the report ([RANK-SCORE]).
fn assert_copy_is_saturated(report: &Value, copy: &Value) -> Result<()> {
    for (name, expected) in DETERMINED_SIGNALS {
        assert!(
            approx(signal(copy, name), expected),
            "{GEOMETRY_LABEL}: nothing differs between the two copies, so `{name}` \
             must read {expected} with embeddings off — {dump}",
            dump = signal_dump(copy),
        );
    }
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
        NOTHING_DUPLICATED,
        "{GEOMETRY_LABEL}: {CALL_STRANGER} varies its literals — it is the family \
         the filter exists to suppress, and it earns nothing on either side of \
         the A/B: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
}

// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] The arbitration itself,
// stated as the only difference between two runs over one source.
#[test]
fn an_intra_file_verbatim_copy_pays_the_idiom_proof_price() -> Result<()> {
    assert_eq!(
        corpus_source(GEOMETRY_INTRA_CASE)?,
        corpus_source(CALL_CASE)?,
        "the A/B is only an argument while both halves hold the same source — \
         once the bytes differ, the opposite outcomes below stop being about file \
         geometry and this test stops proving anything"
    );
    assert_intra_file_copy_stays_hidden(&render(GEOMETRY_INTRA_CASE, MIN_NODES)?);
    assert_cross_file_copy_is_published(&render(CALL_CASE, MIN_NODES)?)
}
