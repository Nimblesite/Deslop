//! Regression coverage for GH #117: live updates must never surface a
//! clone cluster with fewer than two occurrences.

#![cfg(feature = "live")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use deslop_core::{embedding::StubProvider, live::AnalysisSession, report::ReportCluster};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop")
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
    let src = fixture(name);
    let dir = tempfile::tempdir().context("tempdir")?;
    copy_recursive(&src, dir.path())?;
    Ok(dir)
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst).with_context(|| format!("mkdir {}", dst.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("read_dir {}", src.display()))? {
            let entry = entry.context("dir entry")?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        let _bytes = fs::copy(src, dst).with_context(|| format!("copy {}", src.display()))?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn live_update_removes_cluster_when_one_occurrence_remains() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let mut session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;
    let initial = session.report();
    let original = initial
        .clusters
        .first()
        .context("fixture must start with a clone cluster")?;
    assert_eq!(
        original.occurrences.len(),
        2,
        "fixture must start with exactly two duplicate occurrences"
    );
    assert_eq!(original.occurrences_total, 2);
    assert!(all_reported_clusters_have_peers(&initial.clusters));
    let original_id = original.id.clone();

    let changed = tmp.path().join("Alpha.cs");
    fs::write(
        &changed,
        b"namespace Alpha { public class Unique { public int Compute(int input) { return input * 17; } } }\n",
    )
    .context("write unique Alpha")?;

    let delta = session
        .apply_changes(std::slice::from_ref(&changed))
        .context("apply")?;
    let next = session.report();
    assert!(
        delta.clusters_removed.contains(&original_id),
        "the two-copy cluster must be removed when one occurrence changes: {delta:?}"
    );
    assert!(
        delta
            .clusters_added
            .iter()
            .chain(delta.clusters_updated.iter())
            .all(|cluster| cluster.occurrences.len() >= 2),
        "live delta must not carry singleton clusters: {delta:?}"
    );
    assert!(
        all_reported_clusters_have_peers(&next.clusters),
        "latest live report must not retain singleton clusters: {next:#?}"
    );
    assert!(
        session.cluster_by_id(&original_id).is_err(),
        "removed cluster id must no longer resolve"
    );
    assert!(
        session.report_for_file(&changed).clusters.is_empty(),
        "changed file must not show a one-sided stale cluster"
    );
    Ok(())
}

fn all_reported_clusters_have_peers(clusters: &[ReportCluster]) -> bool {
    clusters.iter().all(|cluster| {
        cluster.size >= 2 && cluster.occurrences.len() >= 2 && cluster.occurrences_total >= 2
    })
}
