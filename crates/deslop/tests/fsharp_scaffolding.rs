//! [CLONE-NOISE-FSHARP-IMPORTS] Import-dominated scaffolding is not a clone,
//! while a copied F# implementation remains a reported finding.

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

const FIXTURE: &str = "fsharp-import-scaffolding";
const MIN_NODES: u32 = 20;
const FILES_ANALYSED: u64 = 6;
const REPORTED_CLUSTERS: usize = 1;
const COPY_FILES: [&str; 2] = ["copy_a.fs", "copy_b.fs"];
const COPY_OCCURRENCES: u64 = 2;
const COPY_KIND: &str = "nearly_identical";
const COPY_RANK: u64 = 1;
const COPY_MASS: u64 = 81;
const PROVIDER_FILES: [&str; 2] = ["JsonProvider.fs", "XmlProvider.fs"];
const SCAFFOLD_LAST_LINES: [(&str, u64); 4] = [
    ("JsonProvider.fs", 16),
    ("XmlProvider.fs", 17),
    ("InferenceTests.fs", 10),
    ("HtmlRuntimeTypes.fs", 10),
];

#[test]
fn fsharp_import_lists_are_not_clones_of_implementation() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    assert_eq!(clusters(&report).len(), REPORTED_CLUSTERS, "{report:#}");

    for cluster in clusters(&report) {
        assert!(
            !import_only_cluster(cluster),
            "import-only prologues have no duplicated implementation: {cluster:#}"
        );
        assert!(
            !provider_only_cluster(cluster),
            "different provider implementations are not a clone: {cluster:#}"
        );
    }
    assert_copy(&report)
}

fn assert_copy(report: &Value) -> Result<()> {
    let copy = expect_cluster_spanning(report, &COPY_FILES)?;
    assert_eq!(
        field(copy, "occurrence_count").as_u64(),
        Some(COPY_OCCURRENCES)
    );
    assert_eq!(field(copy, "kind").as_str(), Some(COPY_KIND));
    assert_eq!(field(copy, "rank").as_u64(), Some(COPY_RANK));
    assert_eq!(field(copy, "mass").as_u64(), Some(COPY_MASS));
    Ok(())
}

fn provider_only_cluster(cluster: &Value) -> bool {
    let members = occurrences(cluster);
    members.len() == PROVIDER_FILES.len()
        && PROVIDER_FILES.iter().all(|file| {
            members
                .iter()
                .any(|member| occurrence_path(member).ok() == Some(*file))
        })
}

/// An AST-delimited import run cannot gain implementation from a later line.
fn import_only_cluster(cluster: &Value) -> bool {
    let members = occurrences(cluster);
    members.len() >= 2 && members.iter().all(import_only_member)
}

fn import_only_member(member: &Value) -> bool {
    occurrence_path(member).is_ok_and(|path| {
        SCAFFOLD_LAST_LINES
            .iter()
            .find(|(name, _)| *name == path)
            .is_some_and(|(_, last)| occurrence_line_span(member).1 <= *last)
    })
}
