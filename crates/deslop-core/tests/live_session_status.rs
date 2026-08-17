//! E2E pin for the status surface under the workspace escape hatch
//! ([CONFIG-INCREMENTAL-OPTOUT]): `session_config().incremental` is the
//! **effective** store mode — the request gated by the live config —
//! never the raw request. The audited mismatch reported `true` for a
//! session running every pass uncached under
//! `[analysis] incremental = false`, leaving `cache_stats` and the
//! config surface contradicting each other.

use std::{fs, path::Path, sync::Arc};

use anyhow::{anyhow, ensure, Result};
use deslop_core::{live::AnalysisSession, NoopProvider};

/// Subtree floor for the tiny fixture; the value is irrelevant to the
/// status surface but must admit the file so a real pass runs.
const MIN_NODES: u32 = 8;

/// One analysable file so the session's first pass is a real pass.
const SOURCE: &str = "pub fn doubled(value: i32) -> i32 {\n    value + value\n}\n";

/// Builds a live session over a fresh workspace, requesting persisted
/// processing, with an optional `.deslop.toml` body.
fn session_over(workspace: &Path, config_body: Option<&str>) -> Result<AnalysisSession> {
    fs::create_dir_all(workspace)?;
    fs::write(workspace.join("lib.rs"), SOURCE)?;
    if let Some(body) = config_body {
        fs::write(workspace.join(".deslop.toml"), body)?;
    }
    AnalysisSession::new(
        workspace.to_path_buf(),
        MIN_NODES,
        true,
        None,
        Arc::new(NoopProvider),
    )
    .map_err(|error| anyhow!("session: {error}"))
}

// [CONFIG-INCREMENTAL-OPTOUT] Under the workspace opt-out, a session
// *requested* with persisted processing runs uncached — and its status
// surface must say so. The store must not exist either: the surface,
// the passes, and the disk all agree.
#[test]
fn config_opt_out_is_reported_as_the_effective_incremental_mode() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let session = session_over(tmp.path(), Some("[analysis]\nincremental = false\n"))?;

    let config = session.session_config();
    ensure!(
        !config.incremental,
        "a session requested incremental under `[analysis] incremental = false` \
         runs every pass uncached; the status surface must report the effective \
         mode, got {config:?}"
    );
    let report = session.report();
    ensure!(
        (report.cache_stats.hits, report.cache_stats.misses) == (0, 0),
        "an opted-out pass never consults the store: {:?}",
        report.cache_stats
    );
    ensure!(
        !tmp.path().join(".deslop/cache/fingerprints").exists(),
        "an opted-out session must never create the fingerprint store"
    );
    Ok(())
}

// Without the opt-out the same request reports `true`, so the field is
// the gated conjunction — not a constant and not the raw request.
#[test]
fn a_default_config_reports_the_requested_incremental_mode() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let session = session_over(tmp.path(), None)?;

    let config = session.session_config();
    ensure!(
        config.incremental,
        "with no opt-out the requested store mode is the effective mode, \
         got {config:?}"
    );
    let report = session.report();
    ensure!(
        report.cache_stats.misses > 0,
        "the store-filling first pass must record its misses: {:?}",
        report.cache_stats
    );
    Ok(())
}

// The live toggle cannot out-rank the config: flipping the request on
// under the opt-out still reports — and runs — uncached
// ([CONFIG-INCREMENTAL-OPTOUT] always beats the invocation).
#[test]
fn the_live_toggle_never_overrides_the_config_opt_out() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut session = session_over(tmp.path(), Some("[analysis]\nincremental = false\n"))?;

    let toggled_off = session.toggle_incremental();
    ensure!(
        !toggled_off.incremental,
        "request toggled off under the opt-out: effective stays false: {toggled_off:?}"
    );
    let toggled_on = session.toggle_incremental();
    ensure!(
        !toggled_on.incremental,
        "request toggled back on under the opt-out: the config still gates the \
         effective mode, so the surface must keep reporting false: {toggled_on:?}"
    );
    ensure!(
        !tmp.path().join(".deslop/cache/fingerprints").exists(),
        "no toggle state may create the store while the config opts out"
    );
    Ok(())
}
