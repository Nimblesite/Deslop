//! [LIVE-CACHE-SEED-KEY] A warm start may only serve a seed analysed under
//! the configuration the session is about to run.
//!
//! The editor never passes `--config`: the session finds `<root>/.deslop.toml`
//! itself. The key used to record only an explicit path, so it read
//! `config=none` whatever the workspace file said, and a seed analysed before
//! the user excluded a directory was served again after Reload Window (#609).
//! Each case here starts the session the way the LSP does, with no explicit
//! path, and changes only the workspace config between two starts.

#![cfg(feature = "live")]

use std::{fs, path::Path, sync::Arc};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    embedding::{test_support::StubProvider, EmbeddingMode},
    live::AnalysisSession,
    paths::cache_dir,
    report::Report,
    EmbeddingProvider, DEFAULT_CONFIG_FILENAME,
};

use crate::common::{copy_fixture, DEFAULT_LIVE_MIN_NODES};

/// A fixture whose clone spans [`EXCLUDED_FILE`].
const FIXTURE: &str = "csharp-small";
/// The clustered file the config excludes.
const EXCLUDED_FILE: &str = "Alpha.cs";
/// The key file written beside the seed report.
const SEED_KEY_FILE: &str = "live-report.key";
/// The saved report agents and the next start read.
const SAVED_REPORT_FILE: &str = "live-report.json";
/// A fixture file outside the clone, edited to prove an ordinary edit leaves the saved report alone.
const UNRELATED_EDIT_FILE: &str = "Gamma.cs";
const UNRELATED_SOURCE: &str = "namespace Unique { public class Isolated { public int Answer() { return 42; } } }\n";
/// The key line for a run with no configuration at all.
const NO_CONFIG_LINE: &str = "config=none";
/// A config excluding [`EXCLUDED_FILE`], and the same config one byte longer.
const EXCLUDING_CONFIG: &str = "[defaults]\nexclude = [\"Alpha.cs\"]\n";
const EDITED_CONFIG: &str = "[defaults]\nexclude = [\"Alpha.cs\"]\n\n";

/// Runs a full pass the way the LSP does — no explicit config — and persists its seed.
fn warm_start(root: &Path, provider: &Arc<dyn EmbeddingProvider>) -> Result<Arc<Report>> {
    let session = AnalysisSession::new(
        root.to_path_buf(),
        DEFAULT_LIVE_MIN_NODES,
        false,
        None,
        Arc::clone(provider),
    )
    .context("full pass")?;
    session.persist_seed_cache();
    Ok(session.report())
}

/// The report a restarted session serves from the seed, or `None` when it refuses it.
fn seeded(root: &Path, provider: &Arc<dyn EmbeddingProvider>) -> Option<Arc<Report>> {
    AnalysisSession::try_seeded_from_cache(
        root.to_path_buf(),
        DEFAULT_LIVE_MIN_NODES,
        false,
        None,
        Arc::clone(provider),
        EmbeddingMode::Off,
    )
    .map(|session| session.report())
}

/// Occurrences in `report` that fall in [`EXCLUDED_FILE`].
fn excluded_occurrences(report: &Report) -> usize {
    report
        .clusters
        .iter()
        .flat_map(|cluster| &cluster.occurrences)
        .filter(|occurrence| Path::new(&occurrence.path).file_name() == Some(EXCLUDED_FILE.as_ref()))
        .count()
}

/// The `config=` line of the key recorded beside the seed.
fn recorded_config_line(root: &Path) -> Result<String> {
    let key = fs::read_to_string(cache_dir(root).join(SEED_KEY_FILE)).context("seed key")?;
    key.lines()
        .find(|line| line.starts_with("config="))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("the key has no config line: {key}"))
}

#[test]
fn a_seed_written_before_the_workspace_config_appeared_is_refused() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let root = tmp.path();
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
    let unconfigured = warm_start(root, &provider)?;
    ensure!(excluded_occurrences(&unconfigured) > 0, "control: {EXCLUDED_FILE} clusters with no config");
    ensure!(recorded_config_line(root)? == NO_CONFIG_LINE, "control: no config file, no config identity");
    ensure!(seeded(root, &provider).is_some(), "control: an unchanged start re-uses its seed");

    fs::write(root.join(DEFAULT_CONFIG_FILENAME), EXCLUDING_CONFIG)?;
    ensure!(
        seeded(root, &provider).is_none(),
        "the seed was analysed before .deslop.toml excluded {EXCLUDED_FILE}; serving it shows excluded duplicates"
    );

    let configured = warm_start(root, &provider)?;
    ensure!(excluded_occurrences(&configured) == 0, "the configured pass honours the exclusion");
    let canonical = fs::canonicalize(root.join(DEFAULT_CONFIG_FILENAME))?;
    let line = recorded_config_line(root)?;
    let digest = blake3::hash(EXCLUDING_CONFIG.as_bytes()).to_hex().to_string();
    ensure!(
        line == format!("config={}#{digest}", canonical.display()),
        "the key names the discovered config and digests its bytes: {line}"
    );
    let served = seeded(root, &provider).context("a seed written under the same config is served")?;
    ensure!(excluded_occurrences(&served) == 0, "the served seed holds no excluded occurrence");
    Ok(())
}

#[test]
fn a_seed_written_under_a_since_edited_workspace_config_is_refused() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let root = tmp.path();
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
    fs::write(root.join(DEFAULT_CONFIG_FILENAME), EXCLUDING_CONFIG)?;
    let _report = warm_start(root, &provider)?;
    ensure!(seeded(root, &provider).is_some(), "control: an untouched config re-uses its seed");

    fs::write(root.join(DEFAULT_CONFIG_FILENAME), EDITED_CONFIG)?;
    ensure!(seeded(root, &provider).is_none(), "one edited byte of .deslop.toml refuses the seed");
    Ok(())
}

#[test]
fn a_seed_written_under_a_since_deleted_workspace_config_is_refused() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let root = tmp.path();
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
    fs::write(root.join(DEFAULT_CONFIG_FILENAME), EXCLUDING_CONFIG)?;
    let configured = warm_start(root, &provider)?;
    ensure!(excluded_occurrences(&configured) == 0, "control: the config excludes {EXCLUDED_FILE}");

    fs::remove_file(root.join(DEFAULT_CONFIG_FILENAME))?;
    ensure!(
        seeded(root, &provider).is_none(),
        "the seed hides {EXCLUDED_FILE}, which the session without a config reports"
    );
    Ok(())
}

/// The saved report beside the seed key, as an agent reads it.
fn saved_report(root: &Path) -> Result<Report> {
    let text = fs::read_to_string(cache_dir(root).join(SAVED_REPORT_FILE)).context("saved report")?;
    serde_json::from_str(&text).context("the saved report parses as a report")
}

/// [LIVE-SEED-CACHE] The for-AI page sends agents to the saved report, so a
/// `.deslop.toml` reload rewrites it, and its key, the moment the reload pass
/// lands (#610). An ordinary edit still leaves the file alone.
#[test]
fn a_workspace_config_reload_rewrites_the_saved_report_agents_read() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let root = tmp.path();
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
    let mut session = AnalysisSession::new(
        root.to_path_buf(),
        DEFAULT_LIVE_MIN_NODES,
        false,
        None,
        Arc::clone(&provider),
    )
    .context("full pass")?;
    session.persist_seed_cache();
    ensure!(excluded_occurrences(&saved_report(root)?) > 0, "control: the saved report lists {EXCLUDED_FILE}");

    let config = root.join(DEFAULT_CONFIG_FILENAME);
    fs::write(&config, EXCLUDING_CONFIG)?;
    let _delta = session.apply_changes(std::slice::from_ref(&config))?;
    ensure!(excluded_occurrences(&session.report()) == 0, "the live report honours the new exclusion");
    let saved = saved_report(root)?;
    ensure!(
        excluded_occurrences(&saved) == 0,
        "an agent reading the saved report after the reload still sees {} occurrences in {EXCLUDED_FILE}",
        excluded_occurrences(&saved)
    );
    ensure!(
        saved.clusters.len() == session.report().clusters.len(),
        "the saved report is the reload pass's report"
    );
    let served = seeded(root, &provider).context("the rewritten key matches the new config")?;
    ensure!(excluded_occurrences(&served) == 0, "Reload Window serves the reloaded report");

    let before = fs::read(cache_dir(root).join(SAVED_REPORT_FILE))?;
    fs::write(root.join(UNRELATED_EDIT_FILE), UNRELATED_SOURCE)?;
    let _delta = session.apply_changes(&[root.join(UNRELATED_EDIT_FILE)])?;
    ensure!(
        fs::read(cache_dir(root).join(SAVED_REPORT_FILE))? == before,
        "an ordinary edit does not rewrite the saved report ([LIVE-SEED-CACHE])"
    );
    Ok(())
}
