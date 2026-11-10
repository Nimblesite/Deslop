//! [LIVE-CONFIG-STARTUP] A `.deslop.toml` edited while the session is
//! still scanning must rescope the report once the watcher attaches.
//!
//! The live session reads its config at the start of the cold scan, and
//! the watcher only attaches after that scan returns. On an Osprey
//! checkout the scan took two minutes; an agent rewrote `.deslop.toml`
//! to exclude 86 k lines of generated parser inside that window, raised
//! no event, and the panel kept the old 10.3 % until the file happened
//! to be edited again — while the CLI reported 5.61 % for the same tree.

use std::cmp::Ordering;

use deslop_core::Report;

use super::*;

const CONFIG_NAME: &str = ".deslop.toml";
const LOADED_CONFIG: &str = "[defaults]\n";
const EDITED_CONFIG: &str = "[defaults]\nreport_hide = [\"benchmarks/fixtures/**\"]\n";
const EXCLUDING_CONFIG: &str = "[defaults]\nexclude = [\"benchmarks/**\"]\n";
const STARTUP_EDIT: &str = ".deslop.toml edit during the cold scan";
const STARTUP_DELETE: &str = ".deslop.toml deletion during the cold scan";
const EXTENSIONS: [&str; 1] = ["cs"];
const CLONE_FILES: usize = 2;
const NO_FILES: usize = 0;
const NOTHING_HIDDEN: usize = 0;

/// What the session served before the watcher attached, and after.
struct Rescoped {
    before: Arc<Report>,
    after: Arc<Report>,
}

/// Loads `loaded` as the session's config, lets `change` rewrite or delete
/// it after the session read it but before the watcher exists — so no
/// event will ever describe it — then attaches the live loop and returns
/// the report the attach-time replay produced.
async fn rescope_across_attach(
    loaded: &str,
    files_loaded: usize,
    change: impl FnOnce(&Path) -> std::io::Result<()>,
    trigger: &str,
) -> Result<Rescoped> {
    let _slot = live_slot().await;
    let (_tmp, scan_root) = canonical_temp_root()?;
    seed_hidden_fixture_pair(&scan_root)?;
    let config_path = scan_root.join(CONFIG_NAME);
    fs::write(&config_path, loaded).context("seed .deslop.toml")?;
    let baseline = ConfigBaseline::capture(&scan_root, None);
    assert!(
        baseline.drifted().is_empty(),
        "an untouched config has not drifted"
    );
    let session = live_session(&scan_root)?;
    let (before, generation) = (session.report(), session.generation());
    assert_eq!(
        before.files_analysed, files_loaded,
        "the loaded config's scope"
    );
    assert_eq!(
        before.clusters.is_empty(),
        files_loaded == NO_FILES,
        "the pair clusters when analysed"
    );
    change(&config_path).with_context(|| trigger.to_owned())?;
    assert_eq!(
        baseline.drifted(),
        vec![config_path],
        "the {trigger} is drift"
    );
    let (session_lock, _watcher, _scheduler, mut report_rx) =
        start_live_loop(&scan_root, session, &EXTENSIONS, &baseline).await?;
    let after = reacted_report(&mut report_rx, &session_lock, generation, trigger).await?;
    Ok(Rescoped { before, after })
}

/// Asserts the headline duplicated lines moved in the `expected` direction.
fn assert_duplication(before: &Report, after: &Report, expected: Ordering, what: &str) {
    let (from, to) = (before.metrics.duplicated_loc, after.metrics.duplicated_loc);
    assert_eq!(
        to.cmp(&from),
        expected,
        "{what}: {from} -> {to} duplicated lines"
    );
}

/// An edit between the session reading its config and the watcher
/// attaching is delivered on attach, so the visible report follows it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_edited_during_the_cold_scan_rescopes_the_report_on_attach() -> Result<()> {
    let edit = |path: &Path| fs::write(path, EDITED_CONFIG);
    let Rescoped { before, after } =
        rescope_across_attach(LOADED_CONFIG, CLONE_FILES, edit, STARTUP_EDIT).await?;
    assert_eq!(
        before.clusters_hidden, NOTHING_HIDDEN,
        "nothing is hidden under the loaded config"
    );
    assert!(
        after.clusters.is_empty(),
        "the edited config hides the pair: {:?}",
        after.clusters
    );
    assert_eq!(
        after.clusters_hidden,
        before.clusters.len(),
        "every cluster moved to hidden"
    );
    assert_eq!(
        after.files_analysed, CLONE_FILES,
        "report_hide keeps both files analysed"
    );
    assert_duplication(
        &before,
        &after,
        Ordering::Less,
        "hiding the pair lowers the headline duplication",
    );
    Ok(())
}

/// Deleting the config during the cold scan is drift too: the exclusion
/// it carried is dropped and the excluded files rejoin the corpus.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_deleted_during_the_cold_scan_readmits_its_exclusions_on_attach() -> Result<()> {
    let delete = |path: &Path| fs::remove_file(path);
    let Rescoped { before, after } =
        rescope_across_attach(EXCLUDING_CONFIG, NO_FILES, delete, STARTUP_DELETE).await?;
    assert_eq!(
        after.files_analysed, CLONE_FILES,
        "both copies rejoin the corpus"
    );
    assert!(
        !after.clusters.is_empty(),
        "the readmitted pair clusters again"
    );
    assert_duplication(
        &before,
        &after,
        Ordering::Greater,
        "the readmitted pair raises the headline duplication",
    );
    Ok(())
}
