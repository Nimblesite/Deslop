//! [MCP-TOOL-FINDSIMILAR-EXISTING] A snippet `find-similar` answers for
//! code that exists once, not only for code that is already duplicated.
//!
//! A cluster needs two copies, so a verbatim copy of a method found once in
//! the workspace used to come back as "nothing similar" — the answer an
//! agent reads as permission to write the copy (#309).

#![cfg(feature = "live")]

use std::{fs, path::Path};

use anyhow::{anyhow, ensure, Result};
use deslop_core::live::{AnalysisSession, FindSimilarInput, FindSimilarRequest, FindSimilarResult};

use crate::common::live_session;

/// The language every snippet here is written in.
const LANGUAGE: &str = "csharp";
/// The file holding the one method that exists once.
const LEDGER: &str = "Ledger.cs";
/// A file with nothing in common with the snippet.
const OTHER: &str = "Other.cs";
/// A second ledger, so `Settle` is duplicated and clusters.
const LEDGER_COPY: &str = "LedgerCopy.cs";
/// Rows of the `Ledger` class in [`LEDGER_SOURCE`]. The snippet wraps
/// `Settle` in a one-method class and the ledger is a one-method class, so
/// the two classes are the same normalised code and the outermost match is
/// the class, not only the method inside it.
const LEDGER_CLASS_ROWS: (i64, i64) = (3, 21);

/// A workspace file whose `Settle` method exists nowhere else.
const LEDGER_SOURCE: &str = "namespace Books\n{\n    public static class Ledger\n    {\n        \
public static int Settle(int[] entries, int opening)\n        {\n            var balance = opening;\n            \
foreach (var entry in entries)\n            {\n                if (entry < 0)\n                {\n                    \
balance = balance + entry * 2;\n                }\n                else\n                {\n                    \
balance = balance + entry;\n                }\n            }\n            return balance;\n        }\n    }\n}\n";

/// A second file, so the workspace is not a single file.
const OTHER_SOURCE: &str = "namespace Books\n{\n    public class Other\n    {\n        \
public string Name { get; set; }\n    }\n}\n";

/// What an agent is about to write: `Settle`, copied into a new class.
const COPY_OF_SETTLE: &str = "class Draft\n{\n    public static int Settle(int[] entries, int opening)\n    {\n        \
var balance = opening;\n        foreach (var entry in entries)\n        {\n            if (entry < 0)\n            {\n                \
balance = balance + entry * 2;\n            }\n            else\n            {\n                balance = balance + entry;\n            }\n        }\n        \
return balance;\n    }\n}\n";

/// Code with nothing in common with the workspace, above the node floor.
const UNRELATED: &str = "class Draft\n{\n    public static string Describe(string name, int count)\n    {\n        \
var label = name.Trim().ToUpperInvariant();\n        var suffix = count > 1 ? \"items\" : \"item\";\n        \
return string.Format(\"{0}: {1} {2}\", label, count, suffix);\n    }\n}\n";

/// A two-file workspace with the ledger under `ledger_dir`.
fn workspace(ledger_dir: &str) -> Result<(tempfile::TempDir, AnalysisSession)> {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path().join(ledger_dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(LEDGER), LEDGER_SOURCE)?;
    fs::write(tmp.path().join(OTHER), OTHER_SOURCE)?;
    let session = live_session(tmp.path())?;
    Ok((tmp, session))
}

/// Asks `session` about `snippet`.
fn ask(session: &AnalysisSession, snippet: &str) -> Result<FindSimilarResult> {
    Ok(session.find_similar(&FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: snippet.to_owned(),
            language: LANGUAGE.to_owned(),
        },
        max_results: None,
    })?)
}

#[test]
fn a_verbatim_copy_of_code_that_exists_once_is_found_where_it_lives() -> Result<()> {
    let (_tmp, session) = workspace(".")?;
    ensure!(
        session.report().clusters.is_empty(),
        "the workspace duplicates nothing"
    );
    let answer = ask(&session, COPY_OF_SETTLE)?;
    ensure!(
        !answer.below_min_nodes,
        "the copy is large enough to fingerprint"
    );
    ensure!(answer.clusters.is_empty(), "no cluster exists to return");
    let [found] = answer.existing.as_slice() else {
        return Err(anyhow!(
            "exactly the one existing Settle, outermost match only: {:?}",
            answer.existing
        ));
    };
    ensure!(
        found.path == Path::new(LEDGER),
        "found in the ledger: {found:?}"
    );
    ensure!(
        (found.start_line, found.end_line) == LEDGER_CLASS_ROWS,
        "the outermost match — the whole class, its own rows: {found:?}"
    );
    ensure!(!found.hidden, "an ordinary source file is not hidden");
    Ok(())
}

#[test]
fn code_found_nowhere_returns_no_existing_copy() -> Result<()> {
    let (_tmp, session) = workspace(".")?;
    let answer = ask(&session, UNRELATED)?;
    ensure!(
        !answer.below_min_nodes,
        "the snippet is large enough to fingerprint"
    );
    ensure!(
        answer.clusters.is_empty() && answer.existing.is_empty(),
        "nothing in the workspace resembles the snippet: {:?}",
        answer.existing
    );
    Ok(())
}

#[test]
fn an_existing_copy_in_a_report_hidden_file_says_so() -> Result<()> {
    let (_tmp, session) = workspace("generated")?;
    let answer = ask(&session, COPY_OF_SETTLE)?;
    let [found] = answer.existing.as_slice() else {
        return Err(anyhow!(
            "the hidden ledger is still a place the code exists: {:?}",
            answer.existing
        ));
    };
    ensure!(
        found.hidden,
        "a `generated` path is report-hidden, and the match says so: {found:?}"
    );
    Ok(())
}

#[test]
fn a_snippet_of_clustered_code_is_answered_by_its_cluster_alone() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    fs::write(tmp.path().join(LEDGER), LEDGER_SOURCE)?;
    fs::write(tmp.path().join(LEDGER_COPY), LEDGER_SOURCE)?;
    let session = live_session(tmp.path())?;
    let answer = ask(&session, COPY_OF_SETTLE)?;
    ensure!(
        !answer.clusters.is_empty(),
        "the two ledgers cluster, and the cluster answers"
    );
    ensure!(
        answer.existing.is_empty(),
        "a place a returned cluster lists is not repeated in `existing`: {:?}",
        answer.existing
    );
    Ok(())
}

/// A two-file Rust scaffold duplicated whole — the shape of #264's
/// `ReportFixture`, which the report clusters.
const SCAFFOLD: &str = "pub struct ReportFixture {\n    language: String,\n    floor: usize,\n    rows: Vec<String>,\n}\n\n\
impl ReportFixture {\n    pub fn new(language: &str, floor: usize) -> Self {\n        let rows = Vec::with_capacity(floor);\n        \
Self { language: language.to_owned(), floor, rows }\n    }\n\n    pub fn member(&mut self, name: &str, index: usize) -> String {\n        \
let label = format!(\"{}-{}\", name.trim(), index);\n        let marked = if index > self.floor { format!(\"{label}!\") } else { label };\n        \
self.rows.push(marked.clone());\n        format!(\"{}:{}\", marked, self.language)\n    }\n\n    pub fn cluster(&self, sizes: &[usize]) -> usize {\n        \
sizes.iter().filter(|size| **size >= self.floor).sum()\n    }\n\n    pub fn render(&self) -> String {\n        let mut output = String::new();\n        \
for row in &self.rows {\n            output.push_str(row.trim());\n            output.push('\\n');\n        }\n        output + &self.language\n    }\n}\n";

/// Part of [`SCAFFOLD`] — its struct, `new` and `member` — with one string
/// literal changed, as an agent drafting a variant would write it.
const SCAFFOLD_SUBSET: &str = "pub struct ReportFixture {\n    language: String,\n    floor: usize,\n    rows: Vec<String>,\n}\n\n\
impl ReportFixture {\n    pub fn new(language: &str, floor: usize) -> Self {\n        let rows = Vec::with_capacity(floor);\n        \
Self { language: language.to_owned(), floor, rows }\n    }\n\n    pub fn member(&mut self, name: &str, index: usize) -> String {\n        \
let label = format!(\"{}-{}\", name.trim(), index);\n        let marked = if index > self.floor { format!(\"{label}?\") } else { label };\n        \
self.rows.push(marked.clone());\n        format!(\"{}:{}\", marked, self.language)\n    }\n}\n";

/// The two copies of [`SCAFFOLD`].
const SCAFFOLD_FILES: [&str; 2] = ["issue_121_scaffold.rs", "thresholds_scaffold.rs"];

/// #264 — a near-verbatim *subset* of a duplicated block (two of its four
/// methods, one literal changed) is answered by the block's cluster.
#[test]
fn a_near_verbatim_part_of_a_duplicated_block_finds_its_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    for file in SCAFFOLD_FILES {
        fs::write(tmp.path().join(file), SCAFFOLD)?;
    }
    let session = live_session(tmp.path())?;
    let answer = session.find_similar(&FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: SCAFFOLD_SUBSET.to_owned(),
            language: "rust".to_owned(),
        },
        max_results: None,
    })?;
    let [cluster] = answer.clusters.as_slice() else {
        return Err(anyhow!(
            "the scaffold's one cluster answers the part: {:?}",
            answer.clusters
        ));
    };
    let files: Vec<&Path> = cluster
        .occurrences
        .iter()
        .map(|found| found.path.as_path())
        .collect();
    ensure!(
        files == SCAFFOLD_FILES.map(Path::new),
        "the cluster spans both copies: {files:?}"
    );
    ensure!(
        answer.existing.is_empty(),
        "both places are the cluster's: {:?}",
        answer.existing
    );
    Ok(())
}
