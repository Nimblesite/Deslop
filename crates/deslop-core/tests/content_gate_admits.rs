//! [FUSED-CONTENT-GATE] Pipeline coverage: a same-file pair that varies only its literals is admitted at the same support floor as a cross-file pair, so the copy forms a closure and reaches the report.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use deslop_core::{
    error::CoreError,
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingMode, Report,
};

const DART_FORWARDING_FIXTURE: &str = "../deslop/tests/fixtures/dart-forwarding-business-pair";
const PRICING_FILE: &str = "Pricing.dart";
const MIN_NODES: u32 = 12;
const FILES_ANALYSED: usize = 1;
const VISIBLE_CLUSTERS: usize = 1;
const HIDDEN_CLUSTERS: usize = 0;
const PAIR: usize = 2;
/// `standardTotal` spans lines 35-38 and `premiumTotal` lines 40-43.
const STANDARD_TOTAL_LINES: (i64, i64) = (35, 38);
const PREMIUM_TOTAL_LINES: (i64, i64) = (40, 43);

/// [FUSED-CONTENT-GATE] `standardTotal` and `premiumTotal` are structurally identical and differ in one string and one integer. Nothing is renamed, so rename evidence is 0.0 and positional agreement (0.727) is the pair's whole case; that clears the support floor, and within one file the sibling-family question belongs to [RANK-STRUCTURAL-ONLY-FORWARDING], which reads where the calls go — here, back into the class — and keeps the pair visible.
#[test]
fn a_same_file_literal_only_copy_is_admitted_and_published() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(DART_FORWARDING_FIXTURE);
    let report = run_without_embeddings(root)?;
    assert_eq!(report.files_analysed, FILES_ANALYSED);
    assert_eq!(
        report.clusters.len(),
        VISIBLE_CLUSTERS,
        "the literal-only copy must form one visible closure: {:?}",
        report.clusters
    );
    assert_eq!(
        report.clusters_hidden, HIDDEN_CLUSTERS,
        "nothing in the fixture is scaffolding, so nothing may be hidden"
    );
    let cluster = report
        .clusters
        .first()
        .ok_or_else(|| anyhow!("the visible cluster asserted above is missing"))?;
    assert_eq!(cluster.occurrence_count, PAIR);
    assert_eq!(cluster.occurrences_total, PAIR);
    assert_eq!(cluster.mass, cluster.canonical_node_count as u64);
    let lines: Vec<(i64, i64)> = cluster
        .occurrences
        .iter()
        .map(|occurrence| (occurrence.start_line, occurrence.end_line))
        .collect();
    assert_eq!(lines, vec![STANDARD_TOTAL_LINES, PREMIUM_TOTAL_LINES]);
    assert!(
        cluster
            .occurrences
            .iter()
            .all(|occurrence| occurrence.path.ends_with(PRICING_FILE) && !occurrence.hidden),
        "both occurrences are visible members of the one file: {:?}",
        cluster.occurrences
    );
    Ok(())
}

fn run_without_embeddings(root: PathBuf) -> Result<Report, CoreError> {
    run(&PipelineConfig {
        root,
        min_nodes: MIN_NODES,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Off,
            provider: None,
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
}
