//! Public report/config API coverage.

use std::{fs, path::PathBuf};

use deslop_core::{
    ast::ByteRange,
    boilerplate::BoilerplateRange,
    config::ExclusionConfig,
    report::{
        ActionHint, CacheStats, EmbeddingProvenance, Report, ReportCluster, ReportOccurrence,
        ReportSignals,
    },
    report_boilerplate::build_boilerplate_hints,
    report_metrics::RepoMetrics,
    state::FileRegistry,
};

#[test]
fn truncate_for_wire_caps_occurrences_and_blanks_derivable_text() {
    let report = sample_report().truncate_for_wire(2).truncate_for_wire(2);
    assert!(report.schema_doc.is_empty());
    let Some(cluster) = report.clusters.first() else {
        assert_eq!(report.clusters.len(), 1);
        return;
    };
    assert_eq!(cluster.occurrences.len(), 2);
    assert_eq!(cluster.occurrences_total, 3);
    assert!(cluster.occurrences_truncated);
    assert!(cluster.summary.is_empty());
    assert!(cluster.interpretation.is_empty());
}

#[test]
fn boilerplate_hints_use_default_recommendation_for_future_languages() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let config_path = tmp.path().join(".deslop.toml");
    fs::write(
        &config_path,
        "[defaults.boilerplate]\nimports = \"report\"\n",
    )?;
    let config = ExclusionConfig::load(&config_path)?;
    assert_eq!(config.source_path(), config_path.as_path());
    let mut registry = FileRegistry::new();
    let file_id = registry.register(tmp.path().join("main.go"));
    let ranges = vec![range(file_id, "go", 0, 10), range(file_id, "go", 11, 20)];
    let hints = build_boilerplate_hints(&ranges, &registry, tmp.path(), &config);
    assert_eq!(hints.len(), 1);
    let Some(hint) = hints.first() else {
        assert_eq!(hints.len(), 1);
        return Ok(());
    };
    assert_eq!(hint.language, "go");
    assert!(hint.recommendation.contains("harder to read"));
    Ok(())
}

fn range(
    file_id: deslop_core::state::FileId,
    language: &'static str,
    start: usize,
    end: usize,
) -> BoilerplateRange {
    BoilerplateRange {
        file_id,
        language,
        byte_range: ByteRange { start, end },
    }
}

fn sample_report() -> Report {
    Report {
        tool_version: "test".to_owned(),
        min_nodes: 3,
        files_analysed: 2,
        clusters_hidden: 0,
        cache_stats: CacheStats::default(),
        metrics: RepoMetrics::default(),
        schema_doc: "schema".to_owned(),
        action_hints: vec![ActionHint {
            pattern: "bucket=identical".to_owned(),
            recommendation: "extract".to_owned(),
        }],
        boilerplate_hints: Vec::new(),
        embedding_provenance: Some(EmbeddingProvenance {
            provider_id: "stub".to_owned(),
            model_id: "model".to_owned(),
            model_version: "v1".to_owned(),
            dimensions: 3,
            attempted_subtrees: 0,
            indexed_subtrees: 0,
            failed_subtrees: 0,
        }),
        clusters: vec![sample_cluster()],
    }
}

fn sample_cluster() -> ReportCluster {
    ReportCluster {
        id: "abcdef".to_owned(),
        weight: 1.0,
        size: 3,
        canonical_node_count: 12,
        signals: ReportSignals {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.0,
            fused: 1.0,
        },
        bucket: "identical".to_owned(),
        occurrences: sample_occurrences(),
        occurrences_total: 0,
        occurrences_truncated: false,
        summary: "summary".to_owned(),
        interpretation: "interpretation".to_owned(),
    }
}

fn sample_occurrences() -> Vec<ReportOccurrence> {
    (0_usize..3)
        .map(|index| {
            let start_byte = index.saturating_mul(10);
            ReportOccurrence {
                path: PathBuf::from(format!("file-{index}.cs")),
                start_byte,
                end_byte: start_byte.saturating_add(5),
                start_line: i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX),
                end_line: i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX),
                hidden: false,
            }
        })
        .collect()
}
