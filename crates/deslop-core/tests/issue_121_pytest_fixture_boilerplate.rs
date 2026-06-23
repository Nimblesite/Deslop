//! Regression coverage for GH #121: repeated pytest fixture row builders
//! are test setup boilerplate, not actionable duplication.
//!
//! Tests [CLONE-NOISE-PY-PYTEST-FIXTURE]

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
fn pytest_fixture_row_builders_stay_out_of_ranked_report() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path();
    let mut fixture = ReportFixture::new(scan_root);

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
        PairScore {
            structural: 1.0,
            token_jaccard: 0.97,
            embedding_cos: 0.0,
        },
    );
    let report = fixture.render(&[fixture_cluster]);
    let visible_clusters = report
        .clusters
        .iter()
        .map(|cluster| (&cluster.id, &cluster.bucket, cluster.size, cluster.signals))
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
            .all(|(id, _, _, _)| *id != "pytest-fixture-row-builders"),
        "GH #121 fixture builder cluster leaked into the ranked report"
    );

    Ok(())
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
                model_id: "pytest-fixture-fixture".to_owned(),
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
