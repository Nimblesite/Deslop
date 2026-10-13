//! [LIVE-WATCHER-DIRECTORIES] Directory events recover files created before watch attachment.

use std::collections::BTreeSet;

use deslop_core::Report;

use super::*;

pub(super) const FIXTURE: &str = "csharp-small";
pub(super) const FIRST_SOURCE: &str = "Alpha.cs";
pub(super) const SECOND_SOURCE: &str = "Beta.cs";
const DIRECTORY: &str = "incoming";
const NESTED_SOURCE: &str = "incoming/nested/Beta.cs";
const EMPTY_PATH: &str = "incoming/nested/empty.rs";
const EMPTY_SOURCE: &str = "";
const SINGLE_FILE: usize = 1;
pub(super) const CLONE_FILES: usize = 2;
const TOTAL_FILES: usize = 3;
const SINGLE_CLUSTER: usize = 1;
const NO_CLUSTERS: usize = 0;
const FIRST_RANK: usize = 1;
const CLONE_KIND: &str = "nearly_identical";
const IGNORE_NAME: &str = ".ignore";
const CONFIG_NAME: &str = ".deslop.toml";
const IGNORE_INCOMING: &str = "incoming/**/*.cs\n";
const ALLOW_BETA: &str = "!Beta.cs\n";
const EXCLUDE_BETA: &str = "[defaults]\nexclude = [\"**/Beta.cs\"]\n";
const IGNORE_BETA: &str = "/Beta.cs\n";
const EXCLUDED_DIRECTORY: &str = "target/cache";
const IGNORE_DIRECTORY: &str = "incoming/\n";
const IGNORE_CLONE: &str = "Beta.cs\n";

/// Creates a tracked empty file beside a clone rejected by its directory's ignore file.
fn ignored_incoming_session(root: &Path) -> Result<AnalysisSession> {
    fs::remove_file(root.join(SECOND_SOURCE))?;
    let mut session = live_session(root)?;
    write_incoming(root)?;
    fs::write(
        root.join(NESTED_SOURCE).with_file_name(IGNORE_NAME),
        IGNORE_CLONE,
    )?;
    let _delta = session.apply_changes(&[root.join(DIRECTORY)])?;
    assert_eq!(session.report().files_analysed, CLONE_FILES);
    assert_eq!(session.report().clusters.len(), NO_CLUSTERS);
    assert_membership(&session.report(), &[FIRST_SOURCE, EMPTY_PATH]);
    Ok(session)
}

#[test]
fn deleted_directory_releases_its_ignore_rules() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    let mut session = ignored_incoming_session(root.path())?;
    fs::remove_dir_all(root.path().join(DIRECTORY))?;
    let _removed = session.apply_changes(&[root.path().join(DIRECTORY)])?;
    assert_eq!(session.report().files_analysed, SINGLE_FILE);
    assert_eq!(session.report().clusters.len(), NO_CLUSTERS);
    write_incoming(root.path())?;
    let _added = session.apply_changes(&[
        root.path().join(NESTED_SOURCE),
        root.path().join(EMPTY_PATH),
    ])?;
    assert_incoming_report(&session.report())?;
    assert_cold_parity(&session, root.path())
}

/// Creates a populated subtree without delivering any leaf events.
fn write_incoming(root: &Path) -> Result<()> {
    let source = root.join(NESTED_SOURCE);
    fs::create_dir_all(source.parent().context("nested source parent")?)?;
    fs::write(source, fs::read(fixture(FIXTURE).join(SECOND_SOURCE))?)?;
    fs::write(root.join(EMPTY_PATH), EMPTY_SOURCE)?;
    Ok(())
}

/// Pins the clone, ordering, occurrence paths, and all live files including empty source.
fn assert_incoming_report(report: &Report) -> Result<()> {
    assert_clone(report, &[FIRST_SOURCE, NESTED_SOURCE], TOTAL_FILES)?;
    assert_membership(report, &[FIRST_SOURCE, NESTED_SOURCE, EMPTY_PATH]);
    Ok(())
}

/// Pins the complete ranked clone rather than relying only on corpus size.
pub(super) fn assert_clone(report: &Report, expected: &[&str], files: usize) -> Result<()> {
    assert_eq!(report.files_analysed, files);
    assert_eq!(report.clusters.len(), SINGLE_CLUSTER);
    let cluster = report
        .clusters
        .first()
        .context("expected clone must be present")?;
    assert_eq!(serde_json::to_value(cluster.kind)?, CLONE_KIND);
    assert_eq!(cluster.rank, FIRST_RANK);
    assert_eq!(cluster.occurrence_count, CLONE_FILES);
    assert_eq!(cluster.occurrences_total, CLONE_FILES);
    let paths: BTreeSet<_> = cluster
        .occurrences
        .iter()
        .map(|item| item.path.clone())
        .collect();
    assert_eq!(paths, expected.iter().map(PathBuf::from).collect());
    Ok(())
}

/// Includes authored empty files in the asserted corpus denominator.
pub(super) fn assert_membership(report: &Report, expected: &[&str]) {
    let paths: BTreeSet<_> = report
        .metrics
        .per_file
        .iter()
        .map(|item| item.path.clone())
        .collect();
    assert_eq!(paths, expected.iter().map(PathBuf::from).collect());
}

#[test]
fn directory_event_recovers_every_preexisting_descendant() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    fs::remove_file(root.path().join(SECOND_SOURCE))?;
    let mut session = live_session(root.path())?;
    assert_eq!(session.report().files_analysed, SINGLE_FILE);
    assert_eq!(session.report().clusters.len(), NO_CLUSTERS);
    write_incoming(root.path())?;
    let _delta = session.apply_changes(&[root.path().join(DIRECTORY)])?;
    assert_incoming_report(&session.report())
}

/// [LIVE-WATCHER-RESCAN] One root event repairs both missed deletions and additions.
#[test]
fn root_event_reconciles_missing_and_new_source_paths() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    let mut session = live_session(root.path())?;
    assert_eq!(session.report().files_analysed, CLONE_FILES);
    assert_eq!(session.report().clusters.len(), SINGLE_CLUSTER);
    fs::remove_file(root.path().join(SECOND_SOURCE))?;
    write_incoming(root.path())?;
    let _delta = session.apply_changes(&[root.path().to_path_buf()])?;
    assert_incoming_report(&session.report())
}

#[test]
fn directory_event_honours_new_nested_ignore_overrides() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    fs::remove_file(root.path().join(SECOND_SOURCE))?;
    fs::write(root.path().join(IGNORE_NAME), IGNORE_INCOMING)?;
    let mut session = live_session(root.path())?;
    assert_eq!(session.report().files_analysed, SINGLE_FILE);
    assert_eq!(session.report().clusters.len(), NO_CLUSTERS);
    write_incoming(root.path())?;
    fs::write(
        root.path().join(NESTED_SOURCE).with_file_name(IGNORE_NAME),
        ALLOW_BETA,
    )?;
    let _delta = session.apply_changes(&[root.path().join(DIRECTORY)])?;
    assert_incoming_report(&session.report())?;
    assert_cold_parity(&session, root.path())
}

/// Ensures scoped admission and metrics agree with independent canonical cold discovery.
fn assert_cold_parity(session: &AnalysisSession, root: &Path) -> Result<()> {
    let cold = live_session(root)?;
    let report = session.report();
    let expected = cold.report();
    assert_eq!(
        serde_json::to_value(&report.clusters)?,
        serde_json::to_value(&expected.clusters)?
    );
    assert_eq!(
        serde_json::to_value(&report.metrics)?,
        serde_json::to_value(&expected.metrics)?
    );
    Ok(())
}

/// The old clone must disappear when missed config or ignore edits reject Beta.
fn assert_beta_excluded(session: &mut AnalysisSession, root: &Path) -> Result<()> {
    let _delta = session.apply_changes(&[root.to_path_buf()])?;
    let report = session.report();
    assert_eq!(report.files_analysed, SINGLE_FILE);
    assert_eq!(report.clusters.len(), NO_CLUSTERS);
    assert_membership(&report, &[FIRST_SOURCE]);
    Ok(())
}

#[test]
fn root_rescan_reloads_missed_config_and_ignore_edits() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    let mut session = live_session(root.path())?;
    assert_clone(
        &session.report(),
        &[FIRST_SOURCE, SECOND_SOURCE],
        CLONE_FILES,
    )?;
    exclude_beta(&mut session, root.path(), CONFIG_NAME, EXCLUDE_BETA)?;
    fs::remove_file(root.path().join(CONFIG_NAME))?;
    exclude_beta(&mut session, root.path(), IGNORE_NAME, IGNORE_BETA)?;
    fs::write(root.path().join(IGNORE_NAME), EMPTY_SOURCE)?;
    let _delta = session.apply_changes(&[root.path().to_path_buf()])?;
    assert_clone(
        &session.report(),
        &[FIRST_SOURCE, SECOND_SOURCE],
        CLONE_FILES,
    )
}

/// Replays a root rescan after a policy notification was missed.
fn exclude_beta(
    session: &mut AnalysisSession,
    root: &Path,
    name: &str,
    policy: &str,
) -> Result<()> {
    fs::write(root.join(name), policy)?;
    assert_beta_excluded(session, root)
}

#[test]
fn directory_event_honours_ancestor_directory_ignore_rules() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    fs::remove_file(root.path().join(SECOND_SOURCE))?;
    fs::write(root.path().join(IGNORE_NAME), IGNORE_DIRECTORY)?;
    let mut session = live_session(root.path())?;
    let before = session.report();
    write_incoming(root.path())?;
    let _delta = session.apply_changes(&[root.path().join(DIRECTORY)])?;
    assert!(Arc::ptr_eq(&before, &session.report()));
    assert_eq!(session.report().files_analysed, SINGLE_FILE);
    assert_eq!(session.report().clusters.len(), NO_CLUSTERS);
    assert_membership(&session.report(), &[FIRST_SOURCE]);
    assert_cold_parity(&session, root.path())
}

#[test]
fn excluded_directory_event_preserves_the_existing_report() -> Result<()> {
    let root = copy_fixture(FIXTURE)?;
    let mut session = live_session(root.path())?;
    let before = session.report();
    assert_clone(&before, &[FIRST_SOURCE, SECOND_SOURCE], CLONE_FILES)?;
    let excluded = root.path().join(EXCLUDED_DIRECTORY);
    fs::create_dir_all(&excluded)?;
    fs::write(
        excluded.join(SECOND_SOURCE),
        fs::read(root.path().join(SECOND_SOURCE))?,
    )?;
    let _delta = session.apply_changes(&[excluded])?;
    assert!(Arc::ptr_eq(&before, &session.report()));
    assert_membership(&session.report(), &[FIRST_SOURCE, SECOND_SOURCE]);
    Ok(())
}
