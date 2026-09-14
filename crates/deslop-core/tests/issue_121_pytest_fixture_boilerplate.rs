//! Regression coverage for GH #121: repeated pytest fixture row builders
//! are test setup boilerplate, not actionable duplication.
//!
//! Tests [CLONE-NOISE-PY-PYTEST-FIXTURE]

use crate::common;

use anyhow::Result;
use common::ReportFixture;

#[test]
fn pytest_fixture_row_builders_stay_out_of_ranked_report() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path();
    let mut fixture = ReportFixture::new(scan_root, "python");

    let fixture_cluster = fixture.cluster(
        "pytest-fixture-row-builders",
        vec![
            (
                "test_conversations.py",
                "@pytest.fixture\nasync def conversation(db_session, tenant):\n    row = Conversation(id=uuid.uuid4(), tenant_id=tenant.id, title=\"chat\")\n    db_session.add(row)\n    await db_session.commit()\n    await db_session.refresh(row)\n    return row\n",
            ),
            (
                "test_messages.py",
                "@fixture\nasync def message(db_session, tenant):\n    row = Message(id=uuid.uuid4(), tenant_id=tenant.id, body=\"hello\")\n    db_session.add(row)\n    await db_session.commit()\n    await db_session.refresh(row)\n    return row\n",
            ),
            (
                "test_runs.py",
                "@pytest_asyncio.fixture\nasync def run(db_session, tenant):\n    row = AgentRun(id=uuid.uuid4(), tenant_id=tenant.id, status=\"queued\")\n    db_session.add(row)\n    await db_session.commit()\n    await db_session.refresh(row)\n    return row\n",
            ),
        ],
        118,
    );
    let report = fixture.render(&[fixture_cluster]);
    let visible_clusters = report
        .clusters
        .iter()
        .map(|cluster| (&cluster.id, cluster.mass, cluster.occurrence_count))
        .collect::<Vec<_>>();

    assert_eq!(
        report.files_analysed, 3,
        "all fixture files must be analysed"
    );
    assert_eq!(
        report.clusters_hidden, 1,
        "pytest fixture row builders should be hidden as setup boilerplate; visible clusters: {visible_clusters:#?}"
    );
    assert!(
        report.clusters.is_empty(),
        "ranked report must not ask users to extract pytest fixture row setup: {visible_clusters:#?}"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != "pytest-fixture-row-builders"),
        "GH #121 fixture builder cluster leaked into the ranked report"
    );

    Ok(())
}
