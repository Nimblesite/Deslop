//! Regression coverage for the low-structure fusion false positives
//! tracked in GH #98, #99, #108, #120, and #122.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use deslop_core::{
    ast::ByteRange,
    cluster::Cluster,
    fingerprint::Fingerprint,
    pair::PairScore,
    render_report,
    report::CacheStats,
    report_metrics::AnalysedLines,
    state::{FileId, FileRegistry},
    EmbeddingProvenance, ExclusionConfig, ReportInputs,
};

#[test]
fn low_structure_token_and_embedding_noise_stays_out_of_ranked_report() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path();
    let mut fixture = ReportFixture::new(scan_root);

    let clusters = threshold_false_positive_clusters(&mut fixture);
    let report = fixture.render(&clusters);
    let visible_clusters = report
        .clusters
        .iter()
        .map(|cluster| (&cluster.id, &cluster.bucket, cluster.size, cluster.signals))
        .collect::<Vec<_>>();

    assert_eq!(
        clusters.len(),
        4,
        "fixture must exercise all threshold false-positive shapes"
    );
    assert!(
        clusters
            .iter()
            .all(|cluster| cluster.signals.fused() >= 0.85),
        "each fixture cluster models a pair that clears the current fused gate"
    );
    assert_eq!(
        report.clusters_hidden, 4,
        "all threshold false positives should be hidden from the ranked report; visible clusters: {visible_clusters:#?}"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _, _)| *id != "schema-token-only"),
        "GH #108 token-only JSON schema cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _, _)| *id != "json-fixture-mixed"),
        "GH #98 mixed test fixture cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _, _)| *id != "assertion-only"),
        "GH #99 assertion-only cluster leaked into the ranked report"
    );
    assert!(
        visible_clusters
            .iter()
            .all(|(id, _, _, _)| *id != "embedding-mega"),
        "GH #120/#122 embedding mega-cluster leaked into the ranked report"
    );
    assert!(
        report.clusters.is_empty(),
        "ranked report must not surface low-structure token/embedding noise: {visible_clusters:#?}"
    );
    assert!(
        report.clusters.iter().all(
            |cluster| cluster.bucket != "same_behavior" && cluster.bucket != "nearly_identical"
        ),
        "false positives must not be relabelled into actionable buckets"
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

struct ReportFixture {
    scan_root: PathBuf,
    registry: FileRegistry,
    file_languages: HashMap<FileId, &'static str>,
    sources: HashMap<FileId, Vec<u8>>,
    analysed_lines: AnalysedLines,
}

impl ReportFixture {
    fn new(scan_root: &Path) -> Self {
        Self {
            scan_root: scan_root.to_owned(),
            registry: FileRegistry::new(),
            file_languages: HashMap::new(),
            sources: HashMap::new(),
            analysed_lines: HashMap::new(),
        }
    }

    fn cluster(
        &mut self,
        id: &str,
        snippets: Vec<(&str, &str)>,
        node_count: usize,
        signals: PairScore,
    ) -> Cluster {
        let members = snippets
            .into_iter()
            .enumerate()
            .map(|(index, (path, source))| self.member(path, source, node_count, index))
            .collect::<Vec<_>>();
        Cluster {
            id: id.to_owned(),
            members,
            weight: 10_000.0,
            signals,
        }
    }

    fn member(
        &mut self,
        path: &str,
        source: &str,
        node_count: usize,
        hash_seed: usize,
    ) -> Fingerprint {
        let file_id = self.registry.register(self.scan_root.join(path));
        let bytes = source.as_bytes().to_vec();
        let _old = self.sources.insert(file_id, bytes.clone());
        let _old = self.file_languages.insert(file_id, "python");
        let _old = self.analysed_lines.insert(
            file_id,
            u64::try_from(source.lines().count()).unwrap_or(u64::MAX),
        );
        Fingerprint {
            hash: [u8::try_from(hash_seed).unwrap_or(u8::MAX); 32],
            file_id,
            byte_range: ByteRange {
                start: 0,
                end: bytes.len(),
            },
            node_count,
        }
    }

    fn render(&self, clusters: &[Cluster]) -> deslop_core::Report {
        let exclusion = ExclusionConfig::empty();
        render_report(ReportInputs {
            clusters,
            registry: &self.registry,
            file_languages: &self.file_languages,
            files_analysed: self.sources.len(),
            min_nodes: 15,
            scan_root: &self.scan_root,
            exclusion: &exclusion,
            embedding_provenance: Some(EmbeddingProvenance {
                provider_id: "stub".to_owned(),
                model_id: "threshold-fixture".to_owned(),
                model_version: "test".to_owned(),
                dimensions: 3,
                attempted_subtrees: self.sources.len(),
                indexed_subtrees: self.sources.len(),
                failed_subtrees: 0,
            }),
            cache_stats: CacheStats::default(),
            sources: &self.sources,
            analysed_lines: &self.analysed_lines,
            boilerplate_ranges: &[],
        })
    }
}
