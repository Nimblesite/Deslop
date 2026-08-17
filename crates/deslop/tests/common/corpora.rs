//! Authoring a corpus in code: sibling clone files, the F#
//! literal-table generator, and the "build these files, then report on
//! them" shortcut, plus the rank/category lookups those suites read
//! their verdict from.
//!
//! Split from `common` proper, which walks a report the tool already
//! rendered. These helpers run before the tool does.

use std::{fs, path::Path};

use serde_json::Value;

use super::{clusters, field, occurrence_files, run_report, Result};

/// Writes two byte-identical source files (`a.<extension>`, `b.<extension>`)
/// into a freshly created `dir`: the minimal corpus for a fully-duplicated
/// repo, used to prove the duplication metric is language-agnostic.
pub(crate) fn write_identical_pair(dir: &Path, extension: &str, source: &str) -> Result<()> {
    fs::create_dir_all(dir)?;
    for stem in ["a", "b"] {
        fs::write(dir.join(format!("{stem}.{extension}")), source)?;
    }
    Ok(())
}

/// A genuine copy-pasted F# function — byte-identical across two files.
/// Shared recall-guard source for the #331/#336 shape-only fixtures.
pub(crate) const FSHARP_GENUINE_CLONE: &str = "module ParseHelpers\n\n\
    let accumulate (values: int list) (floor: int) =\n\
    \x20   let mutable total = 0\n\
    \x20   for value in values do\n\
    \x20       if value > floor then\n\
    \x20           total <- total + value * 2\n\
    \x20       else\n\
    \x20           total <- total - 1\n\
    \x20   total\n";

/// One F# module holding a numeric array literal. Same length (same
/// shape) across modules, entirely different values — the #336
/// false-positive family.
pub(crate) fn fsharp_table_file(module_name: &str, seed: usize) -> String {
    let values: Vec<String> = (0_usize..24)
        .map(|index| {
            let mixed = seed
                .saturating_mul(37)
                .saturating_add(index.saturating_mul(13));
            (mixed % 97).to_string()
        })
        .collect();
    format!(
        "module {module_name}\n\nlet lookup = [| {} |]\n",
        values.join("; ")
    )
}

/// The four distinct-value F# table files plus the byte-identical
/// genuine clone pair — the canonical #336 corpus.
pub(crate) fn fsharp_tables_corpus() -> Vec<(String, String)> {
    let modules = ["TablesAlpha", "TablesBeta", "TablesGamma", "TablesDelta"];
    let mut files: Vec<(String, String)> = modules
        .iter()
        .enumerate()
        .map(|(index, module_name)| {
            (
                format!("tables_{index}.fs"),
                fsharp_table_file(module_name, index),
            )
        })
        .collect();
    files.extend(genuine_pair(
        "parse_a.fs",
        "parse_b.fs",
        FSHARP_GENUINE_CLONE,
    ));
    files
}

/// The two byte-identical files forming a genuine-clone recall guard.
pub(crate) fn genuine_pair(first: &str, second: &str, source: &str) -> [(String, String); 2] {
    [
        (first.to_owned(), source.to_owned()),
        (second.to_owned(), source.to_owned()),
    ]
}

/// Writes `(file_name, source)` pairs into a temp scan root and returns
/// the rendered report at `min_nodes`. Config rides along as an ordinary
/// `.deslop.toml` entry in `files` when a test needs a policy override.
pub(crate) fn report_for(files: &[(String, String)], min_nodes: u32) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("src");
    fs::create_dir_all(&root)?;
    for (file_name, source) in files {
        fs::write(root.join(file_name), source)?;
    }
    run_report(&root, min_nodes)
}

/// Zero-based rank of the first cluster whose occurrences include a file
/// whose name satisfies `matches`, or `None` when no visible cluster does.
pub(crate) fn rank_where(report: &Value, matches: impl Fn(&str) -> bool) -> Option<usize> {
    clusters(report)
        .iter()
        .position(|cluster| cluster_file_set(cluster).iter().any(|name| matches(name)))
}

/// The `category` wire label of the first cluster touching a matching
/// file name, or `""` when no visible cluster does. Resolves the cluster
/// through [`rank_where`] so the two lookups can never disagree.
pub(crate) fn category_where(report: &Value, matches: impl Fn(&str) -> bool) -> String {
    rank_where(report, matches)
        .and_then(|rank| clusters(report).get(rank))
        .and_then(|cluster| field(cluster, "category").as_str())
        .unwrap_or_default()
        .to_owned()
}
