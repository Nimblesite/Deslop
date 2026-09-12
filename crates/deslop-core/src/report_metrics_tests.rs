//! [METRICS-REPO] Informational matches cannot change duplication figures.

use anyhow::{anyhow, Result};

use super::*;
use crate::{
    ast::ByteRange,
    buckets::ClusterKind,
    fingerprint::Fingerprint,
    report_render::{cluster_to_report, ReportSources},
    report_weight::rank_by_mass,
};

/// Three two-line files, all retained in the denominator.
const PATHS: [&str; 3] = ["src/a.rs", "src/b.rs", "src/c.rs"];
/// The first line is the established copied range.
const SOURCE: &[u8] = b"first\nsecond\n";
/// End of the first complete line.
const COPIED_END: usize = 6;
/// The copied pair's canonical AST size.
const NODES: usize = 12;
/// Physical lines across the complete fixture.
const ANALYSED: u64 = 6;
/// Two occurrences establish the fixture's clone.
const COPIES: usize = 2;

/// Inputs shared by the metric and report conversion checks.
struct Corpus {
    /// Stable source identities.
    registry: FileRegistry,
    /// Complete measured sources.
    sources: HashMap<FileId, Vec<u8>>,
    /// Both informational and genuine ranges.
    clusters: Vec<Cluster>,
}

impl Corpus {
    /// A clone pair and an overlapping informational group spanning all files.
    fn new() -> Self {
        let mut registry = FileRegistry::new();
        let ids: Vec<_> = PATHS
            .iter()
            .map(|path| registry.register(PathBuf::from(path)))
            .collect();
        let sources = ids.iter().map(|id| (*id, SOURCE.to_vec())).collect();
        let copied_ids: Vec<_> = ids.iter().copied().take(COPIES).collect();
        let clones = make_cluster(ClusterKind::Identical, &copied_ids, COPIED_END);
        let information = make_cluster(ClusterKind::StructuralOnly, &ids, SOURCE.len());
        Self {
            registry,
            sources,
            clusters: vec![clones, information],
        }
    }

    /// Computes all metric projections from a chosen set of findings.
    fn metrics(&self, clusters: &[&Cluster]) -> RepoMetrics {
        let indexed = ReportSources::new(&self.sources);
        let analysed_lines = self
            .sources
            .iter()
            .map(|(id, source)| (*id, count_analysed_lines(source)))
            .collect();
        compute_repo_metrics(&MetricsInputs {
            clusters,
            sources: &self.sources,
            line_indices: indexed.line_indices(),
            file_languages: &HashMap::new(),
            registry: &self.registry,
            exclusion: &ExclusionConfig::empty(),
            analysed_lines: &analysed_lines,
            scan_root: Path::new("."),
            diff: None,
        })
    }
}

/// Explicit classifications isolate metric behavior from detector routing.
fn make_cluster(kind: ClusterKind, files: &[FileId], end: usize) -> Cluster {
    let members = files
        .iter()
        .map(|file_id| Fingerprint {
            hash: [0; 32],
            file_id: *file_id,
            byte_range: ByteRange { start: 0, end },
            node_count: NODES,
        })
        .collect();
    Cluster {
        id: kind.wire_label().to_owned(),
        members,
        mass: 1000,
        kind,
        shape_family: None,
    }
}

#[test]
fn shape_only_information_has_zero_duplication_in_every_projection() -> Result<()> {
    let corpus = Corpus::new();
    let information = corpus
        .clusters
        .last()
        .ok_or_else(|| anyhow!("informational fixture"))?;
    let metrics = corpus.metrics(&[information]);
    assert_eq!(metrics.analysed_loc, ANALYSED);
    assert_eq!(metrics.duplicated_loc, 0);
    assert_eq!(metrics.duplicated_files, 0);
    assert_eq!(metrics.clusters_total, 0);
    assert_eq!(metrics.duplication_percent.to_bits(), 0.0_f64.to_bits());
    assert!(metrics.folders.is_empty());
    assert!(metrics
        .per_file
        .iter()
        .all(|file| file.duplicated_loc == 0 && file.duplication_percent == 0.0));
    assert!(
        !ThresholdSummary::resolve(0.0, ThresholdSource::Config, metrics.duplication_percent)
            .breached
    );
    Ok(())
}

#[test]
fn information_visibility_cannot_hide_or_inflate_overlapping_clones() -> anyhow::Result<()> {
    let corpus = Corpus::new();
    let clones = corpus
        .clusters
        .first()
        .ok_or_else(|| anyhow!("clone fixture"))?;
    let all = corpus.metrics(&corpus.clusters.iter().collect::<Vec<_>>());
    let only_clones = corpus.metrics(&[clones]);
    assert_eq!(
        serde_json::to_value(&all)?,
        serde_json::to_value(&only_clones)?
    );
    assert_eq!(all.analysed_loc, ANALYSED);
    assert_eq!(all.duplicated_loc, 2);
    assert_eq!(all.duplicated_files, 2);
    assert_eq!(all.clusters_total, 1);
    let folder = all.folders.first().ok_or_else(|| anyhow!("src folder"))?;
    assert_eq!(folder.path, PathBuf::from("src"));
    assert_eq!(folder.analysed_loc, ANALYSED);
    assert_eq!(folder.duplicated_loc, 2);
    Ok(())
}

#[test]
fn shape_only_has_zero_weight_and_no_clone_rank() -> Result<()> {
    let corpus = Corpus::new();
    let sources = ReportSources::new(&corpus.sources);
    let mut clusters: Vec<_> = corpus
        .clusters
        .iter()
        .rev()
        .map(|cluster| {
            cluster_to_report(
                cluster,
                &corpus.registry,
                &HashMap::new(),
                Path::new("."),
                &ExclusionConfig::empty(),
                &sources,
            )
        })
        .collect();
    rank_by_mass(&mut clusters);
    let clone = clusters.first().ok_or_else(|| anyhow!("first finding"))?;
    let information = clusters.last().ok_or_else(|| anyhow!("last finding"))?;
    assert_eq!(clone.kind, ClusterKind::Identical);
    assert_eq!(clone.mass, u64::try_from(NODES)?);
    assert_eq!(clone.rank, 1);
    assert_eq!(information.kind, ClusterKind::StructuralOnly);
    assert_eq!(information.mass, 0);
    assert_eq!(information.rank, 0);
    Ok(())
}
