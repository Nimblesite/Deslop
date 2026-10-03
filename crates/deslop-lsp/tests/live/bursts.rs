//! [LIVE-WATCHER-DELIVERY] Real filesystem bursts preserve the rendered report.

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::{
    at,
    reports::{assert_report_shell, cluster_paths, report_clusters, wait_for_report},
    session::FixtureSession,
};

const FIXTURE: &str = "csharp-small";
const SOURCE_FILES: u64 = 2;
const BURST_FILES: u64 = 257;
const FIRST_FILE: u64 = 0;
const ONE_FILE: u64 = 1;
const ONE_CLUSTER: usize = 1;
const NO_CLUSTERS: usize = 0;
const FIRST_CLUSTER: usize = 0;
const FIRST_RANK: u64 = 1;
const EMPTY_SOURCE: &str = "";
const BURST_DIRECTORY: &str = "generated";
const FIRST_SOURCE: &str = "Alpha.cs";
const SECOND_SOURCE: &str = "Beta.cs";
const FILES_FIELD: &str = "files_analysed";
const CLUSTERS_FIELD: &str = "clusters";
const METRICS_FIELD: &str = "metrics";
const KIND_FIELD: &str = "kind";
const RANK_FIELD: &str = "rank";
const COUNT_FIELD: &str = "occurrence_count";
const CLONE_KIND: &str = "nearly_identical";

fn settled(session: &mut FixtureSession, files: u64, clusters: usize) -> Result<Value> {
    wait_for_report(&mut session.stdin, &mut session.stdout, |report| {
        at(report, FILES_FIELD).as_u64() == Some(files)
            && at(report, CLUSTERS_FIELD)
                .as_array()
                .is_some_and(|entries| entries.len() == clusters)
    })
}

fn assert_clone(report: &Value, files: u64) -> Result<()> {
    assert_report_shell(report, files);
    let clusters = report_clusters(report)?;
    assert_eq!(clusters.len(), ONE_CLUSTER);
    let cluster = clusters
        .get(FIRST_CLUSTER)
        .ok_or_else(|| anyhow!("expected the fixture's clone cluster"))?;
    assert_eq!(at(cluster, KIND_FIELD), CLONE_KIND);
    assert_eq!(at(cluster, RANK_FIELD), FIRST_RANK);
    assert_eq!(at(cluster, COUNT_FIELD), SOURCE_FILES);
    let paths = BTreeSet::from([FIRST_SOURCE.to_owned(), SECOND_SOURCE.to_owned()]);
    assert_eq!(cluster_paths(cluster), paths);
    Ok(())
}

fn create_burst(root: &Path) -> Result<()> {
    let directory = root.join(BURST_DIRECTORY);
    fs::create_dir(&directory)?;
    for index in FIRST_FILE..BURST_FILES {
        fs::write(directory.join(format!("empty-{index}.rs")), EMPTY_SOURCE)?;
    }
    fs::remove_file(root.join(SECOND_SOURCE))?;
    Ok(())
}

fn burst_and_restore(session: &mut FixtureSession, initial: &Value) -> Result<()> {
    let source = fs::read(session.workspace.path().join(SECOND_SOURCE))?;
    create_burst(session.workspace.path())?;
    let removed = settled(session, SOURCE_FILES + BURST_FILES - ONE_FILE, NO_CLUSTERS)?;
    assert_eq!(
        at(&removed, FILES_FIELD),
        SOURCE_FILES + BURST_FILES - ONE_FILE
    );
    assert_eq!(report_clusters(&removed)?.len(), NO_CLUSTERS);
    fs::write(session.workspace.path().join(SECOND_SOURCE), source)?;
    let restored = settled(session, SOURCE_FILES + BURST_FILES, ONE_CLUSTER)?;
    assert_clone(&restored, SOURCE_FILES + BURST_FILES)?;
    assert_eq!(at(&restored, CLUSTERS_FIELD), at(initial, CLUSTERS_FIELD));
    Ok(())
}

#[test]
fn source_burst_and_directory_removal_refresh_the_complete_report() -> Result<()> {
    let mut session = FixtureSession::open(FIXTURE)?;
    let initial = settled(&mut session, SOURCE_FILES, ONE_CLUSTER)?;
    assert_clone(&initial, SOURCE_FILES)?;
    burst_and_restore(&mut session, &initial)?;
    fs::remove_dir_all(session.workspace.path().join(BURST_DIRECTORY))?;
    let final_report = settled(&mut session, SOURCE_FILES, ONE_CLUSTER)?;
    assert_clone(&final_report, SOURCE_FILES)?;
    assert_eq!(
        at(&final_report, CLUSTERS_FIELD),
        at(&initial, CLUSTERS_FIELD)
    );
    assert_eq!(
        at(&final_report, METRICS_FIELD),
        at(&initial, METRICS_FIELD)
    );
    Ok(())
}
