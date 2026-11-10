//! [CORPUS-CHECK-FIELDS] Every report field a corpus check reads is one the
//! renderer actually writes.
//!
//! The corpus checks read a report only through `ReportField`. This renders
//! real reports and asserts each listed field is present, in the shape the
//! checks read it as, at every place it should sit — then pins the values a
//! check would see, so a renamed field cannot read as zero again (gh #540:
//! `size` and `category` were read for months and never emitted).

use std::{collections::BTreeSet, fs, path::Path};

use deslop_test_support::report_field::{FieldLevel, ReportField};
use serde_json::Value;

use crate::common::*;

/// One clone family to render, and what its report must say.
struct Family {
    /// The scan directory.
    dir: &'static str,
    /// The copied source, and the files it is copied into.
    body: &'static str,
    files: &'static [&'static str],
    /// The copy's last line and end byte in each file.
    end_line: u64,
    end_byte: u64,
    /// The canonical occurrence's node count and the cluster's mass.
    nodes: u64,
    mass: u64,
}

/// A Python function copied into two files.
const PYTHON_PAIR: Family = Family {
    dir: "python_pair",
    body: "def f(a, b):\n    total = 0\n    for item in a:\n        if item > b:\n            total += item * 2\n        else:\n            total -= item\n    return total\n",
    files: &["a.py", "b.py"],
    end_line: 8,
    end_byte: 153,
    nodes: 37,
    mass: 37,
};
/// A Rust function copied into three files.
const RUST_TRIO: Family = Family {
    dir: "rust_trio",
    body: "pub fn tally(values: &[i64], limit: i64) -> i64 {\n    let mut total = 0;\n    for value in values {\n        if *value > limit {\n            total += value * 2;\n        } else {\n            total -= value;\n        }\n    }\n    total\n}\n",
    files: &["x.rs", "y.rs", "z.rs"],
    end_line: 11,
    end_byte: 231,
    nodes: 48,
    mass: 96,
};
/// The node floor both families are scanned at.
const MIN_NODES: u32 = 8;
/// Every file is wholly duplicated, so the whole repository is.
const ALL_DUPLICATED: f64 = 100.0;
/// The one cluster each family renders, first in rank.
const ONE_CLUSTER: usize = 1;
const FIRST_RANK: u64 = 1;
/// Each copy starts the file it sits in.
const FIRST_LINE: u64 = 1;
const FIRST_BYTE: u64 = 0;

#[test]
fn every_field_the_corpus_checks_read_is_rendered_with_the_value_a_check_needs() -> Result<()> {
    let names: BTreeSet<&str> = ReportField::ALL.iter().map(|field| field.name()).collect();
    assert_eq!(
        names.len(),
        ReportField::ALL.len(),
        "each field is listed once, so none hides another"
    );
    for family in [PYTHON_PAIR, RUST_TRIO] {
        let report = render(&family)?;
        assert_every_field_is_rendered(&report, family.dir);
        assert_report_values(&report, &family);
        let [cluster] = ReportField::Clusters.items(&report) else {
            return Err(anyhow::anyhow!(
                "{}: one cluster expected: {report:#}",
                family.dir
            ));
        };
        assert_cluster_values(cluster, &family);
    }
    Ok(())
}

/// Writes `family` into a fresh directory and renders its report.
fn render(family: &Family) -> Result<Value> {
    let (_tmp, root) = temp_scan_dir(family.dir)?;
    for file in family.files {
        fs::write(root.join(file), family.body)?;
    }
    run_report(Path::new(&root), MIN_NODES)
}

/// Every listed field sits, well formed, wherever its level says it does.
fn assert_every_field_is_rendered(report: &Value, dir: &str) {
    let clusters = ReportField::Clusters.items(report);
    let occurrences: Vec<&Value> = clusters
        .iter()
        .flat_map(|cluster| ReportField::Occurrences.items(cluster))
        .collect();
    assert!(
        !occurrences.is_empty(),
        "{dir}: the report renders occurrences"
    );
    for field in ReportField::ALL {
        let holders: Vec<&Value> = match field.level() {
            FieldLevel::Report | FieldLevel::Metrics => vec![report],
            FieldLevel::Cluster => clusters.iter().collect(),
            FieldLevel::Occurrence => occurrences.clone(),
        };
        for holder in holders {
            assert!(
                field.is_well_formed(holder),
                "{dir}: `{}` is not rendered as a {:?} on {holder:#}",
                field.name(),
                field.shape()
            );
        }
    }
}

/// The report-level figures the scope and progress lines read.
fn assert_report_values(report: &Value, family: &Family) {
    let files = u64::try_from(family.files.len()).unwrap_or(u64::MAX);
    assert_eq!(ReportField::FilesAnalysed.count(report), files);
    assert_eq!(
        ReportField::AnalysedLoc.count(report),
        files.saturating_mul(family.end_line)
    );
    assert!(
        (ReportField::DuplicationPercent.number(report) - ALL_DUPLICATED).abs() < f64::EPSILON,
        "{}: every line is duplicated",
        family.dir
    );
    assert_eq!(ReportField::Clusters.items(report).len(), ONE_CLUSTER);
}

/// The cluster and occurrence figures the mass, rank and precision checks read.
fn assert_cluster_values(cluster: &Value, family: &Family) {
    let copies = u64::try_from(family.files.len()).unwrap_or(u64::MAX);
    assert_eq!(ReportField::Rank.count(cluster), FIRST_RANK);
    assert_eq!(ReportField::OccurrenceCount.count(cluster), copies);
    assert_eq!(ReportField::OccurrencesTotal.count(cluster), copies);
    assert_eq!(ReportField::CanonicalNodeCount.count(cluster), family.nodes);
    assert_eq!(ReportField::Mass.count(cluster), family.mass);
    assert_eq!(
        family.mass,
        family.nodes.saturating_mul(copies.saturating_sub(1)),
        "[RANK-MASS-SUM]"
    );
    assert_eq!(ReportField::Kind.text(cluster), Some(IDENTICAL_KIND));
    assert!(ReportField::Id
        .text(cluster)
        .is_some_and(|id| !id.is_empty()));
    let occurrences = ReportField::Occurrences.items(cluster);
    let paths: Vec<&str> = occurrences
        .iter()
        .filter_map(|occurrence| ReportField::Path.text(occurrence))
        .collect();
    assert_eq!(
        paths, family.files,
        "{}: one occurrence per copy",
        family.dir
    );
    for occurrence in occurrences {
        assert_eq!(ReportField::StartLine.count(occurrence), FIRST_LINE);
        assert_eq!(ReportField::EndLine.count(occurrence), family.end_line);
        assert_eq!(ReportField::StartByte.count(occurrence), FIRST_BYTE);
        assert_eq!(ReportField::EndByte.count(occurrence), family.end_byte);
        assert!(
            !ReportField::Hidden.is_set(occurrence),
            "every copy is shown"
        );
    }
}
