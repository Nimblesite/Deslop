//! GH #398: the fixture harness registered one path into a fresh
//! `FileId` per cluster member, so every same-file cluster reached the
//! production router as cross-file — taking [CLONE-BUCKETS-ROUTING]'s
//! lenient cross-file promotion floor (`CONTENT_SUPPORT_FLOOR`)
//! instead of the single-file one (`CONTENT_PROMOTE_FLOOR`) — and the
//! metrics counted the path once per member. A harness that
//! misdescribes the corpus makes every assertion built on it vacuous:
//! these tests pin the harness itself to reality.

mod common;

use anyhow::{bail, Result};
use common::ReportFixture;
use deslop_core::{content::ContentEvidence, pair::PairScore};

/// Two same-shaped loaders in one file whose identifiers and literals
/// diverge inconsistently: structural evidence saturates while the
/// measured content support lands between the cross-file and
/// single-file promotion floors, so the two routing branches disagree
/// about the verdict — the discriminating case for gh #398.
const FIRST_FN: &str = "def load_alpha(cfg):\n    path = cfg.root + \"/alpha.json\"\n    data = read_json(path)\n    return data[\"alpha\"]\n";
const SECOND_FN: &str = "def load_beta(cfg):\n    target = cfg.root + \"/beta.json\"\n    rows = read_json(target)\n    return rows[\"gamma\"]\n";

fn saturated_shape() -> PairScore {
    PairScore {
        structural: 1.0,
        token_jaccard: 1.0,
        embedding_cos: 0.0,
    }
}

/// Measured content evidence sitting between `CONTENT_SUPPORT_FLOOR`
/// (0.7, the cross-file promotion bar) and `CONTENT_PROMOTE_FLOOR`
/// (0.85, the single-file one), so the two routing branches disagree
/// about the verdict — the discriminating input for gh #398.
fn gap_content() -> ContentEvidence {
    ContentEvidence {
        agreement: 0.75,
        rename_consistency: 0.0,
        literal_fraction: 0.0,
        substance_varies: true,
        verbatim_dominated: false,
        measured: true,
    }
}

fn rendered_same_file_report(scan_root: &std::path::Path) -> deslop_core::Report {
    let mut fixture = ReportFixture::new(scan_root, "python");
    let cluster = fixture.cluster_with_content(
        "same-file-pair",
        vec![("pair.py", FIRST_FN), ("pair.py", SECOND_FN)],
        24,
        saturated_shape(),
        gap_content(),
    );
    fixture.render(&[cluster])
}

#[test]
fn same_file_cluster_is_one_file_with_distinct_member_spans() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let report = rendered_same_file_report(tmp.path());
    assert_eq!(
        report.files_analysed, 1,
        "one path is one file, however many cluster members it carries"
    );
    let [cluster] = report.clusters.as_slice() else {
        bail!("exactly one fabricated cluster must render, got {}", report.clusters.len());
    };
    let paths: Vec<&str> = cluster
        .occurrences
        .iter()
        .filter_map(|occurrence| occurrence.path.to_str())
        .collect();
    assert_eq!(paths, ["pair.py", "pair.py"], "both members live in pair.py");
    let spans: Vec<(usize, usize)> = cluster
        .occurrences
        .iter()
        .map(|occurrence| (occurrence.start_byte, occurrence.end_byte))
        .collect();
    assert_eq!(
        spans,
        [
            (0, FIRST_FN.len()),
            (
                FIRST_FN.len(),
                FIRST_FN.len().saturating_add(SECOND_FN.len())
            )
        ],
        "two members of one file occupy distinct slices of that file, \
         not two copies of byte zero"
    );
    Ok(())
}

#[test]
fn same_file_metrics_count_the_path_once() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let report = rendered_same_file_report(tmp.path());
    let rows: Vec<(std::path::PathBuf, u64)> = report
        .metrics
        .per_file
        .iter()
        .map(|row| (row.path.clone(), row.analysed_loc))
        .collect();
    assert_eq!(
        rows,
        [(std::path::PathBuf::from("pair.py"), 8)],
        "pair.py is one 8-line file (two 4-line functions), not two \
         phantom files"
    );
    Ok(())
}

#[test]
fn same_file_cluster_promotes_only_at_the_single_file_floor() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let report = rendered_same_file_report(tmp.path());
    let [cluster] = report.clusters.as_slice() else {
        bail!("exactly one fabricated cluster must render, got {}", report.clusters.len());
    };
    let support = cluster
        .signals
        .agreement
        .max(cluster.signals.rename_consistency);
    assert!(
        (0.7..0.85).contains(&support),
        "the supplied content evidence must land between \
         CONTENT_SUPPORT_FLOOR and CONTENT_PROMOTE_FLOOR so the two \
         routing branches disagree; rendered agreement={} \
         rename_consistency={}",
        cluster.signals.agreement,
        cluster.signals.rename_consistency
    );
    assert_eq!(
        cluster.bucket, "structural_only",
        "a single-file cluster below the single-file promotion floor \
         must not promote to nearly_identical — promotion here means \
         the router was told the members span two files"
    );
    Ok(())
}
