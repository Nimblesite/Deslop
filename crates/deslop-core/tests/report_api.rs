//! Public report/config API coverage.

use std::{fs, path::PathBuf};

use deslop_core::{
    ast::ByteRange,
    boilerplate::BoilerplateRange,
    config::ExclusionConfig,
    report::{
        ActionHint, CacheStats, EmbeddingProvenance, Report, ReportCluster, ReportOccurrence,
        ReportSignalSource, ReportSignals,
    },
    report_boilerplate::build_boilerplate_hints,
    report_metrics::RepoMetrics,
    state::FileRegistry,
};

#[test]
fn truncate_for_wire_caps_occurrences_and_blanks_derivable_text() {
    let report = sample_report()
        .truncate_for_wire(WIRE_OCCURRENCE_CAP)
        .truncate_for_wire(WIRE_OCCURRENCE_CAP);
    assert!(report.schema_doc.is_empty());
    let Some(cluster) = report.clusters.first() else {
        assert_eq!(report.clusters.len(), 1);
        return;
    };
    assert_eq!(cluster.occurrences.len(), WIRE_OCCURRENCE_CAP);
    assert_eq!(cluster.occurrences_total, FULL_OCCURRENCE_COUNT);
    assert!(cluster.occurrences_truncated);
    assert_eq!(
        cluster
            .occurrences
            .iter()
            .map(|occurrence| occurrence.path.as_path())
            .collect::<Vec<_>>(),
        EXPECTED_SOURCE_PATHS.map(std::path::Path::new),
        "truncation must retain the elected evidence endpoints, not merely the first occurrences"
    );
    assert_eq!(
        cluster.signal_source,
        Some(ReportSignalSource { left: 0, right: 1 }),
        "signal_source must be reindexed into the truncated occurrence list"
    );
    assert!(cluster.summary.is_empty());
    assert!(cluster.interpretation.is_empty());
}

/// Requested live-wire occurrence budget.
const WIRE_OCCURRENCE_CAP: usize = 2;
/// Occurrences carried by the untruncated sample cluster.
const FULL_OCCURRENCE_COUNT: usize = 3;
/// Original occurrence positions elected as the signal source.
const ORIGINAL_SIGNAL_SOURCE: ReportSignalSource = ReportSignalSource { left: 1, right: 2 };
/// Paths belonging to the elected source after the first occurrence is discarded.
const EXPECTED_SOURCE_PATHS: [&str; 2] = ["file-1.cs", "file-2.cs"];

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
            succeeded_subtrees: 0,
            indexed_subtrees: 0,
            failed_subtrees: 0,
        }),
        clusters: vec![sample_cluster()],
        clusters_outside_diff: None,
    }
}

fn sample_cluster() -> ReportCluster {
    let signals = ReportSignals {
        structural: 1.0,
        token_jaccard: 1.0,
        shape: 1.0,
        embedding_cos: 0.0,
        fused: 1.0,
        agreement: 0.0,
        rename_consistency: 0.0,
        literal_fraction: 0.0,
    };
    ReportCluster {
        id: "abcdef".to_owned(),
        rank: 1,
        rank_band: "faint".to_owned(),
        weight: 1.0,
        size: 3,
        canonical_node_count: 12,
        signals,
        signal_source: Some(ORIGINAL_SIGNAL_SOURCE),
        bucket: "identical".to_owned(),
        category: "logic".to_owned(),
        language: "rust".to_owned(),
        meets_fused_gate: true,
        evidence_verdict: deslop_core::render::signals::content_evidence_verdict(signals),
        occurrences: sample_occurrences(),
        occurrences_total: 0,
        occurrence_count: 3,
        occurrences_truncated: false,
        summary: "summary".to_owned(),
        interpretation: "interpretation".to_owned(),
        intersects_diff: None,
        is_newly_introduced: None,
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
                in_diff: None,
            }
        })
        .collect()
}
