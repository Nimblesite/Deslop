//! The committed-golden comparison every golden suite shares
//! ([PIPELINE-DETERMINISM]).
//!
//! A golden test is only as good as its regeneration story: blessing has
//! to be possible, obvious, and never the silent default. One definition
//! of that story here means `report_golden.rs` and
//! `incremental_multilang_golden.rs` cannot drift into two different
//! answers about when a golden may be rewritten.

use std::{fs, path::Path};

use anyhow::Context as _;

use super::Result;

/// The environment variable that switches a golden suite from verifying
/// to regenerating.
const BLESS_VAR: &str = "DESLOP_BLESS";

/// Compares `rendered` against the committed golden at `path`.
///
/// With `DESLOP_BLESS` set the golden is rewritten and the test then
/// fails on purpose: a bless run must never be mistaken for a passing
/// one, and the diff has to be reviewed before it can go green.
/// Otherwise the committed bytes must match exactly, and `drift_hint`
/// explains — in the failure itself — why re-blessing is not the default
/// remedy for this particular golden.
///
/// `bless_command` is the exact command that regenerates this golden, so
/// every failure mode names it rather than leaving the reader to guess
/// the test-binary name.
pub(crate) fn assert_matches_golden(
    rendered: &str,
    path: &Path,
    bless_command: &str,
    drift_hint: &str,
) -> Result<()> {
    if std::env::var_os(BLESS_VAR).is_some() {
        fs::write(path, rendered.as_bytes())?;
        anyhow::bail!(
            "golden re-blessed at {}; review the diff, then re-run without {BLESS_VAR} to verify",
            path.display()
        );
    }
    let expected = fs::read_to_string(path).with_context(|| {
        format!(
            "missing golden {} — generate it with {bless_command}",
            path.display()
        )
    })?;
    assert_eq!(
        rendered,
        expected,
        "rendered output drifted from {} [PIPELINE-DETERMINISM]. {drift_hint} Regenerating is \
         NOT the default remedy — prove the drift is intended, re-bless with {bless_command}, \
         and review the diff.",
        path.display()
    );
    Ok(())
}

/// Reads and parses a committed golden, naming the regeneration command
/// when it is missing or unreadable.
pub(crate) fn load_golden(path: &Path, bless_command: &str) -> Result<serde_json::Value> {
    super::load_json(path).with_context(|| {
        format!(
            "unreadable golden {} — generate it with {bless_command}",
            path.display()
        )
    })
}
