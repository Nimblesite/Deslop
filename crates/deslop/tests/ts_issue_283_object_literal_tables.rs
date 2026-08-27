//! gh #283 — unrelated object-literal tables are not one clone
//! ([CLONE-NOISE-CONSTANT-TABLE]).
//!
//! Reported against `Nimblesite/typeDiagram` as the repository's #1
//! offender: seventeen occurrences at `structural = 0.36,
//! token_jaccard = 1.00, fused = 1.00`, pooling directional language
//! scalar maps, render themes, a syntax-highlight expectation and TDBIN
//! error metadata. They differ in meaning, in key and value semantics,
//! and in ownership; extracting one shared implementation would couple
//! independent converters to unrelated modules.
//!
//! `ecmascript::is_ecmascript_data_shape_cluster` already covered the
//! **type**-level spelling of this shape — a run of
//! `property_signature` members. A `const` bound to an object literal is
//! the same argument one level down, and it now goes through the same
//! language-agnostic rule as Rust `const` and Python `NAME =`.
//!
//! The fixture stages three unrelated tables and a byte-identical
//! `settleLedger` pair. Suppressing the tables must not touch the pair.

use anyhow::Result;

use crate::common::{
    negative_pin::{
        assert_control_is_the_only_published_cluster, assert_family_hidden_with_control,
        assert_only_the_control_files_carry_duplicated_lines,
    },
    *,
};

/// The three unrelated tables.
const FAMILY: [&str; 3] = ["theme.ts", "rust_scalars.ts", "tdbin_errors.ts"];

/// The false-negative control.
const CONTROL: [&str; 2] = ["control_clone_a.ts", "control_clone_b.ts"];

/// Duplicated lines the control clone accounts for: eleven lines, twice.
const CONTROL_LOC: u64 = 22;

/// The three tables and both control files.
const FILES_ANALYSED: u64 = 5;

/// The committed corpus this pin runs against.
const FIXTURE: &str = "ts-issue-283-object-literal-tables";

/// What every failure message here names itself as.
const LABEL: &str = "gh #283 object-literal table family";

/// Node floor at which a run of object-literal properties is a
/// candidate window — the geometry gh #283 reports.
const MIN_NODES: u32 = 8;

/// Components [CLONE-NOISE-CONSTANT-TABLE] suppresses here — measured.
/// The three tables do not pool into one component: the constant-table
/// rule elects two, and a *higher* count is the filter eating something
/// this fixture never staged as noise.
const EXPECTED_HIDDEN: u64 = 2;

// [CLONE-NOISE-CONSTANT-TABLE] gh #283.
#[test]
fn unrelated_object_literal_tables_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    for table in FAMILY {
        assert_family_hidden_with_control(&report, LABEL, &[table], &CONTROL, EXPECTED_HIDDEN)?;
    }
    assert_control_is_the_only_published_cluster(&report, LABEL, &CONTROL, CONTROL_LOC)?;
    assert_only_the_control_files_carry_duplicated_lines(&report, LABEL, &CONTROL);
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED),
        "{LABEL}: all three tables and both control files were analysed, so the \
         suppression was decided rather than the files skipped: {report:#}"
    );
    Ok(())
}
