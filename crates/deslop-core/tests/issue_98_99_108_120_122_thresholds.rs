//! Regression coverage for the low-structure fusion false positives
//! tracked in GH #98, #99, #108, #120, and #122.

use crate::common;

use anyhow::Result;
use common::ReportFixture;
use deslop_core::{cluster::Cluster, pair::PairScore};

#[test]
fn low_structure_token_and_embedding_noise_stays_out_of_ranked_report() -> Result<()> {
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
        4,
        "fixture must exercise all threshold false-positive shapes"
    );
    assert!(clusters.iter().all(|cluster| cluster.mass > 0));
    assert_eq!(
        report.clusters_hidden, 4,
        "all threshold false positives should be hidden from the ranked report; visible clusters: {visible_clusters:#?}"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != "schema-token-only"),
        "GH #108 token-only JSON schema cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != "json-fixture-mixed"),
        "GH #98 mixed test fixture cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != "assertion-only"),
        "GH #99 assertion-only cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _)| *id != "embedding-mega"),
        "GH #120/#122 embedding mega-cluster leaked into the ranked report"
    );
    assert!(
        report.clusters.is_empty(),
        "ranked report must not surface low-structure token/embedding noise: {visible_clusters:#?}"
    );
    Ok(())
}

fn threshold_false_positive_clusters(fixture: &mut ReportFixture) -> Vec<Cluster> {
    vec![
        json_schema_cluster(fixture),
        mixed_fixture_cluster(fixture),
        assertion_cluster(fixture),
        embedding_mega_cluster(fixture),
    ]
}

fn json_schema_cluster(fixture: &mut ReportFixture) -> Cluster {
    fixture.cluster(
        "schema-token-only",
        vec![
            ("schemas.py", "def schema_report_get():\n    return {\"type\": \"object\", \"properties\": {\"path\": {\"type\": \"string\"}}, \"required\": [\"path\"]}\n"),
            ("schemas.py", "def schema_top_offenders():\n    return {\"type\": \"object\", \"properties\": {\"limit\": {\"type\": \"integer\"}}, \"required\": [\"limit\"]}\n"),
        ],
        56,
        PairScore {
            structural: 0.0,
            token_jaccard: 0.96,
            embedding_cos: 0.0,
        },
    )
}

fn mixed_fixture_cluster(fixture: &mut ReportFixture) -> Cluster {
    fixture.cluster(
        "json-fixture-mixed",
        vec![
            ("test_http.py", "payload = {\"name\": \"Bundle Sandbox Agent\", \"system_prompt\": \"You are a website builder.\", \"model_config\": {\"provider\": \"test\"}}\n"),
            ("test_docker.py", "container = {\"id\": \"abc\", \"image\": \"runner\", \"ports\": {\"http\": 8080}, \"status\": \"running\"}\n"),
        ],
        72,
        PairScore {
            structural: 0.11,
            token_jaccard: 0.96,
            embedding_cos: 0.53,
        },
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
        PairScore {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.0,
        },
    )
}

fn embedding_mega_cluster(fixture: &mut ReportFixture) -> Cluster {
    fixture.cluster(
        "embedding-mega",
        vec![
            ("test_usage_api.py", "import pytest\n\nDAY1 = object()\n\n@pytest.fixture\nasync def second_tenant(db_session):\n    return object()\n"),
            ("test_builtins.py", "\"\"\"Tests for built-in tools.\"\"\"\n\nfrom unittest.mock import AsyncMock, patch\n\nimport pytest\n"),
            ("test_sandbox_embodied_http.py", "def _bundle_config_payload(bundle_url):\n    return {\"name\": \"Bundle Sandbox Agent\", \"system_prompt\": \"You are a website builder.\"}\n"),
            ("test_live_edit.py", "class TestSandboxEmbodiedLiveEditHttp:\n    async def test_api_file_patch_is_observable_in_next_chat_turn(self, client):\n        assert client\n"),
            ("test_host.py", "class _LiveEditHost(MockAgentWorkspaceHost):\n    async def admin_request(self, *, instance):\n        return instance\n"),
            ("test_auth.py", "async def test_auth_token(client, tenant):\n    token = await client.post('/auth')\n    assert token\n"),
            ("test_sandbox_coverage.py", "async def test_workspace_status(client, workspace):\n    response = await client.get('/status')\n    assert response.status_code == 200\n"),
            ("test_providers.py", "def test_provider_model_selection(provider):\n    assert provider.default_model == 'test-model'\n"),
            ("conftest.py", "@pytest.fixture\nasync def tenant(db_session):\n    return object()\n"),
            ("test_agent_factory.py", "def test_agent_factory_builds_runner(factory):\n    assert factory.build('runner') is not None\n"),
            ("test_sandbox_dispatcher_e2e.py", "async def test_dispatcher_routes_message(dispatcher):\n    result = await dispatcher.send('hello')\n    assert result\n"),
        ],
        1040,
        PairScore {
            structural: 0.02,
            token_jaccard: 0.79,
            embedding_cos: 0.86,
        },
    )
}
