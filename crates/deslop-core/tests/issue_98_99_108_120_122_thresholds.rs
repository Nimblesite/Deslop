//! [CLONE-NOISE] Render-stage regression coverage for low-structure false
//! positives tracked in GH #98, #99, #108, #120, and #122.
//!
//! Schema-shaped and embedding-role candidates are rejected before closure by
//! [FUSED-CONTENT-GATE] and [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]. This file
//! proves the remaining convicted-noise cases at their render-stage boundary.

use crate::common;

use anyhow::Result;
use common::ReportFixture;
use deslop_core::cluster::Cluster;

const EXPECTED_CONVICTED_CLUSTERS: usize = 2;
const EXPECTED_HIDDEN_CLUSTERS: usize = 2;
const EXPECTED_VISIBLE_CLUSTERS: usize = 0;
const EXPECTED_OCCURRENCES_PER_CLUSTER: usize = 2;
const MIXED_FIXTURE_CLUSTER_ID: &str = "json-fixture-mixed";
const ASSERTION_CLUSTER_ID: &str = "assertion-only";

#[test]
fn convicted_low_structure_noise_stays_out_of_ranked_report() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path();
    let mut fixture = ReportFixture::new(scan_root, "python");

    let clusters = threshold_false_positive_clusters(&mut fixture);
    let report = fixture.render(&clusters);
    let visible_clusters = report
        .clusters
        .iter()
        .map(|cluster| (&cluster.id, cluster.mass, cluster.occurrence_count))
        .collect::<Vec<_>>();

    assert_eq!(
        clusters.len(),
        EXPECTED_CONVICTED_CLUSTERS,
        "fixture must exercise each recognised render-stage noise shape"
    );
    assert!(
        clusters.iter().all(|cluster| cluster.mass > 0),
        "each convicted fixture must carry positive mass before suppression"
    );
    assert!(
        clusters
            .iter()
            .all(|cluster| cluster.members.len() == EXPECTED_OCCURRENCES_PER_CLUSTER),
        "each fixture must exercise both members of its convicted component"
    );
    assert!(
        clusters
            .iter()
            .any(|cluster| cluster.id == MIXED_FIXTURE_CLUSTER_ID),
        "GH #98 mixed fixture case must reach render-stage suppression"
    );
    assert!(
        clusters
            .iter()
            .any(|cluster| cluster.id == ASSERTION_CLUSTER_ID),
        "GH #99 assertion-only case must reach render-stage suppression"
    );
    assert_eq!(
        report.clusters_hidden, EXPECTED_HIDDEN_CLUSTERS,
        "recognised noise must be hidden after closure; visible clusters: {visible_clusters:#?}"
    );
    assert_eq!(
        report.clusters.len(),
        EXPECTED_VISIBLE_CLUSTERS,
        "convicted components must not retain a visible rendering"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != MIXED_FIXTURE_CLUSTER_ID),
        "GH #98 mixed test fixture cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != ASSERTION_CLUSTER_ID),
        "GH #99 assertion-only cluster leaked into the ranked report"
    );
    assert!(
        report.clusters.is_empty(),
        "ranked report must not surface a filter-convicted noise cluster: {visible_clusters:#?}"
    );
    Ok(())
}

fn threshold_false_positive_clusters(fixture: &mut ReportFixture) -> Vec<Cluster> {
    vec![mixed_fixture_cluster(fixture), assertion_cluster(fixture)]
}

fn mixed_fixture_cluster(fixture: &mut ReportFixture) -> Cluster {
    fixture.cluster(
        "json-fixture-mixed",
        vec![
            ("test_http.py", "payload = {\"name\": \"Bundle Sandbox Agent\", \"system_prompt\": \"You are a website builder.\", \"model_config\": {\"provider\": \"test\"}}\n"),
            ("test_docker.py", "container = {\"id\": \"abc\", \"image\": \"runner\", \"ports\": {\"http\": 8080}, \"status\": \"running\"}\n"),
        ],
        72,
    )
}

fn assertion_cluster(fixture: &mut ReportFixture) -> Cluster {
    fixture.cluster(
        "assertion-only",
        vec![
            ("test_openapi.py", "def test_openapi_doc(doc):\n    assert doc[\"info\"][\"title\"] == \"Agent Backend\"\n    assert doc[\"info\"][\"version\"] == \"0.1.0\"\n"),
            ("test_config.py", "def test_fly_config(cfg):\n    assert cfg.api_token == \"t\"\n    assert cfg.app_name == \"a\"\n"),
        ],
        44,
    )
}
