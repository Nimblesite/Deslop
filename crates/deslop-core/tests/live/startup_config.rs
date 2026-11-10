//! [LIVE-CONFIG-STARTUP] A `.deslop.toml` edited while the session is
//! still scanning must rescope the report once the watcher attaches.
//!
//! The live session reads its config at the start of the cold scan, and
//! the watcher only attaches after that scan returns. On an Osprey
//! checkout the scan took two minutes; an agent rewrote `.deslop.toml`
//! to exclude 86 k lines of generated parser inside that window, raised
//! no event, and the panel kept the old 10.3 % until the file happened
//! to be edited again — while the CLI reported 5.61 % for the same tree.

use super::*;

const CONFIG_NAME: &str = ".deslop.toml";
const LOADED_CONFIG: &str = "[defaults]\n";
const EDITED_CONFIG: &str = "[defaults]\nreport_hide = [\"benchmarks/fixtures/**\"]\n";
const DELETED_CONFIG_EXCLUDES: &str = "[defaults]\nexclude = [\"benchmarks/**\"]\n";
const STARTUP_EDIT: &str = ".deslop.toml edit during the cold scan";
const STARTUP_DELETE: &str = ".deslop.toml deletion during the cold scan";
const EXTENSIONS: [&str; 1] = ["cs"];
const CLONE_FILES: usize = 2;
const NO_FILES: usize = 0;
const NOTHING_HIDDEN: usize = 0;

/// An edit between the session reading its config and the watcher
/// attaching is delivered on attach, so the visible report follows it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_edited_during_the_cold_scan_rescopes_the_report_on_attach() -> Result<()> {
    let _slot = live_slot().await;
    let (_tmp, scan_root) = canonical_temp_root()?;
    seed_hidden_fixture_pair(&scan_root)?;
    let config_path = scan_root.join(CONFIG_NAME);
    fs::write(&config_path, LOADED_CONFIG).context("seed .deslop.toml")?;

    let baseline = ConfigBaseline::capture(&scan_root, None);
    assert!(
        baseline.drifted().is_empty(),
        "an untouched config has not drifted"
    );
    let session = live_session(&scan_root)?;
    let before = session.report();
    let before_generation = session.generation();
    assert_eq!(
        before.files_analysed, CLONE_FILES,
        "both fixture copies are analysed"
    );
    assert!(
        !before.clusters.is_empty(),
        "the fixture pair clusters under the loaded config"
    );
    assert_eq!(
        before.clusters_hidden, NOTHING_HIDDEN,
        "nothing is hidden under the loaded config"
    );

    // The edit lands after the session read the config and before the
    // watcher exists: no filesystem event will ever describe it.
    fs::write(&config_path, EDITED_CONFIG).context("edit .deslop.toml")?;
    assert_eq!(
        baseline.drifted(),
        vec![config_path.clone()],
        "the edit is drift"
    );

    let (session_lock, _watcher, _scheduler, mut report_rx) =
        start_live_loop(&scan_root, session, &EXTENSIONS, &baseline).await?;
    let notification = next_report_change(&mut report_rx, STARTUP_EDIT).await?;
    assert_generation_advanced(&notification, before_generation, STARTUP_EDIT);
    let after = report_under_lock(&session_lock).await;
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
    assert!(
        after.metrics.duplicated_loc < before.metrics.duplicated_loc,
        "the headline duplication must fall once the pair is hidden: {} -> {}",
        before.metrics.duplicated_loc,
        after.metrics.duplicated_loc,
    );
    Ok(())
}

/// Deleting the config during the cold scan is drift too: the exclusion
/// it carried is dropped and the excluded files rejoin the corpus.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_deleted_during_the_cold_scan_readmits_its_exclusions_on_attach() -> Result<()> {
    let _slot = live_slot().await;
    let (_tmp, scan_root) = canonical_temp_root()?;
    seed_hidden_fixture_pair(&scan_root)?;
    let config_path = scan_root.join(CONFIG_NAME);
    fs::write(&config_path, DELETED_CONFIG_EXCLUDES).context("seed .deslop.toml")?;

    let baseline = ConfigBaseline::capture(&scan_root, None);
    let session = live_session(&scan_root)?;
    let before = session.report();
    let before_generation = session.generation();
    assert_eq!(
        before.files_analysed, NO_FILES,
        "the loaded config excludes every copy"
    );
    assert!(
        before.clusters.is_empty(),
        "nothing is analysed, so nothing clusters"
    );

    fs::remove_file(&config_path).context("delete .deslop.toml")?;
    assert_eq!(
        baseline.drifted(),
        vec![config_path.clone()],
        "the deletion is drift"
    );

    let (session_lock, _watcher, _scheduler, mut report_rx) =
        start_live_loop(&scan_root, session, &EXTENSIONS, &baseline).await?;
    let notification = next_report_change(&mut report_rx, STARTUP_DELETE).await?;
    assert_generation_advanced(&notification, before_generation, STARTUP_DELETE);
    let after = report_under_lock(&session_lock).await;
    assert_eq!(
        after.files_analysed, CLONE_FILES,
        "both copies rejoin the corpus"
    );
    assert!(
        !after.clusters.is_empty(),
        "the readmitted pair clusters again"
    );
    assert!(
        after.metrics.duplicated_loc > 0,
        "the headline duplication reappears"
    );
    Ok(())
}
