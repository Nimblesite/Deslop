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
         and review the diff.\n{EPOCH_REMINDER}",
        path.display()
    );
    Ok(())
}

/// The one thing a proven-intended report drift must be paired with, and
/// the one a reviewer cannot see in the diff
/// ([PIPELINE-INCREMENTAL-INTEGRITY]).
///
/// Parse-store blobs are addressed by `(language, tool_version,
/// min_nodes, source_hash)`, and the workspace version is the
/// permanently-reused `0.0.0-dev` — so a change that alters what a
/// parse *means* without altering the blob layout leaves every stored
/// blob addressable and a warm run keeps serving the pre-change tree and
/// signatures. That is the only way a warm report can differ from the
/// cold report of the same tree
/// ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]), and no equivalence
/// test can catch it: both sides of the comparison would be stale
/// together.
const EPOCH_REMINDER: &str = "\
If the drift came from a change to parsing, normalisation, fingerprinting or \
signature construction — rather than from ranking, bucketing or rendering — \
bump `fpcache::blob::SEMANTIC_EPOCH` in the same change, or every warm run \
against an existing store keeps serving the pre-change analysis \
([PIPELINE-INCREMENTAL-INTEGRITY]).";

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
