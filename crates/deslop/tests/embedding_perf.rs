//! End-to-end regression coverage for issue #6's deterministic
//! embedding-pass waste: duplicate subtree snippets must not all enter
//! the ANN index.
//!
//! [REMOVE-STUB] The original test used the deterministic stub
//! provider. Production no longer ships the stub, so we drive the
//! same code path through an inline mock Ollama HTTP server.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::{
    approx, cluster_bucket, cluster_size, embeddings::run_mock_embedding_report,
    expect_cluster_spanning, occurrence_files, signal,
};
use crate::mock_ollama::MockOllama;

/// Clone files each corpus writes.
const CLONE_FILES: usize = 8;

/// Identical statements the single-file corpus repeats. Six is enough to
/// out-number `TOP_K`, which is what makes a collapsed point's nearest
/// neighbours its own enclosing and nested windows.
const REPEATED_STATEMENTS: usize = 6;

/// Whether each clone file declares its own namespace.
#[derive(Clone, Copy)]
enum Namespace {
    /// One namespace per file: the files differ, and only the class and
    /// method subtrees inside them are byte-identical.
    PerFile,
    /// One shared namespace: whole files are byte-identical, so the
    /// occurrences the report renders are themselves owners of a single
    /// collapsed ANN point.
    Shared,
}

/// One scan's report plus the embedding provenance it recorded.
struct CloneRun {
    /// Parsed JSON report.
    report: Value,
    /// The report's `embedding_provenance` object.
    provenance: Value,
}

/// [FUSION-EMBED-PROVIDER] Byte-identical subtrees are embedded once and
/// **indexed** once. `attempted_subtrees` counts occurrences, so the gap
/// between it and `indexed_subtrees` is the duplicate work the pass no
/// longer does: N identical points cost N insertions and N queries to
/// return each other, and crowd genuine neighbours out of top-k.
#[test]
fn duplicate_subtree_embeddings_are_collapsed_before_ann() -> Result<()> {
    let run = run_clone_corpus(Namespace::PerFile)?;
    assert_collapse_provenance(&run.provenance);
    let cluster = clone_cluster(&run.report)?;
    assert_every_occurrence_survives(cluster);
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "byte-identical bodies must still reach structural identity: {cluster:#}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "byte-identical bodies must still reach token identity: {cluster:#}"
    );
    Ok(())
}

/// [REPAIR-COSINE-MERGE] Collapsing eight byte-identical occurrences onto
/// one ANN point must cost the report nothing. The vector belongs to
/// every owner, not just the one that reached the index first: an
/// expansion that kept only the first would leave no rendered occurrence
/// pair holding two vectors, and the cluster would render
/// `embedding_cos = 0.0` — "measured, and found unrelated" — about the
/// most perfect duplicate the corpus contains.
///
/// `issue_372_identical_snippet_cosine` pins the same figure for a
/// *pair*. This is the many-owner case the collapse introduced, and it
/// pins it alongside the provenance that proves the collapse happened.
#[test]
fn every_owner_of_a_collapsed_ann_point_keeps_its_measured_cosine() -> Result<()> {
    let run = run_clone_corpus(Namespace::Shared)?;
    assert_collapse_provenance(&run.provenance);
    let cluster = clone_cluster(&run.report)?;
    assert_every_occurrence_survives(cluster);
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "byte-identical files are an identical clone: {cluster:#}"
    );
    let cosine = signal(cluster, "embedding_cos");
    assert!(
        (cosine - 1.0).abs() < f64::EPSILON,
        "all {CLONE_FILES} occurrences share one vector, so their measured cosine \
         is exactly 1.0; got {cosine:.17} — the collapse dropped owners: {cluster:#}"
    );
    Ok(())
}

/// [FUSION-CLUSTER-SIGNALS] Within-file mass duplication survives the
/// collapse. Six identical statements in one file are one clone cluster
/// with the embedding pass off, and must stay one with it on.
///
/// Collapsing duplicates frees the whole top-k for *different* snippets,
/// and inside a single file the nearest different snippets are the
/// statement's own enclosing window and its own sub-expression. Linking
/// them fuses the file's nested windows into one transitive component,
/// the same-file overlap collapse reduces that to a single occurrence,
/// and the cluster drops below the two-member floor — reported before,
/// gone after, with nothing in the report to say so. `Footprint` in
/// `pipeline::embedding_batch` is what refuses those links.
///
/// Read the pass/fail honestly: this fixture is red against a real
/// `nomic-embed-text` (measured: the six-occurrence cluster disappears),
/// and green either way against [`MockOllama`], whose bag-of-shingles
/// vectors rank a statement's enclosing window differently from a
/// semantic model's. So it holds the invariant, and the blind spot it
/// exposes is the mock's, not this suite's.
#[test]
fn within_file_duplication_survives_the_collapsed_index() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    fs::write(scan_root.join("Repeat.cs"), repeated_statement_source())?;
    let report = run_mock_embedding_report(
        &scan_root,
        &tmp.path().join("report"),
        "2",
        server.endpoint(),
    )?;
    let cluster = expect_cluster_spanning(&report, &["Repeat.cs"])?;
    let expected = u64::try_from(REPEATED_STATEMENTS).unwrap_or_default();
    assert_eq!(
        cluster_size(cluster),
        expected,
        "every repeated statement must still be reported: {report:#}"
    );
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "the repeated statements are byte-identical: {cluster:#}"
    );
    assert!(
        approx(signal(cluster, "embedding_cos"), 1.0),
        "they share one collapsed vector, so their cosine is 1.0: {cluster:#}"
    );
    Ok(())
}

/// One C# method whose body repeats a single statement verbatim.
fn repeated_statement_source() -> String {
    let body: String = (0..REPEATED_STATEMENTS)
        .map(|_repeat| "            total = total + 1;\n".to_owned())
        .collect();
    format!(
        "namespace Repeat\n\
         {{\n\
         public class Repeat\n\
         {{\n\
         public int Run(int seed)\n\
         {{\n\
         var total = seed;\n\
         {body}         return total;\n\
         }}\n\
         }}\n\
         }}\n"
    )
}

/// Asserts the pass indexed fewer points than it took in occurrences,
/// and that it did so by deduplication rather than rejection.
fn assert_collapse_provenance(provenance: &Value) {
    let attempted = metric(provenance, "attempted_subtrees");
    let indexed = metric(provenance, "indexed_subtrees");
    assert!(
        indexed > 0,
        "ANN input count must be surfaced: {provenance}"
    );
    assert!(
        indexed < attempted,
        "duplicate subtrees must collapse before ANN indexing: {provenance}"
    );
    assert_eq!(
        metric(provenance, "failed_subtrees"),
        0,
        "the collapse must be deduplication, not rejection — a subtree counted \
         failed lost its embedding signal entirely: {provenance}"
    );
}

/// Asserts the clone cluster still reports one occurrence per file.
fn assert_every_occurrence_survives(cluster: &Value) {
    let expected = u64::try_from(CLONE_FILES).unwrap_or_default();
    assert_eq!(
        cluster_size(cluster),
        expected,
        "every clone occurrence must survive the ANN collapse: {cluster:#}"
    );
    let mut files = occurrence_files(cluster);
    files.sort();
    files.dedup();
    assert_eq!(
        files,
        clone_file_names(),
        "each clone file must be reported exactly once: {cluster:#}"
    );
}

/// The cluster whose occurrences span every clone file in the corpus.
fn clone_cluster(report: &Value) -> Result<&Value> {
    let names = clone_file_names();
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    expect_cluster_spanning(report, &borrowed)
}

/// Every clone file name the fixture writes, in sorted order.
fn clone_file_names() -> Vec<String> {
    (0..CLONE_FILES)
        .map(|index| format!("Clone{index}.cs"))
        .collect()
}

/// Scans a fresh clone corpus through the deterministic mock embedder.
fn run_clone_corpus(namespace: Namespace) -> Result<CloneRun> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_duplicate_fixture(&scan_root, namespace)?;
    let report = run_mock_embedding_report(
        &scan_root,
        &tmp.path().join("report"),
        "4",
        server.endpoint(),
    )?;
    let provenance = embedding_provenance(tmp.path())?;
    Ok(CloneRun { report, provenance })
}

fn write_duplicate_fixture(dir: &Path, namespace: Namespace) -> Result<()> {
    fs::create_dir_all(dir)?;
    for (index, name) in clone_file_names().into_iter().enumerate() {
        fs::write(dir.join(name), clone_source(namespace, index))?;
    }
    Ok(())
}

fn clone_source(namespace: Namespace, index: usize) -> String {
    let suffix = match namespace {
        Namespace::PerFile => index.to_string(),
        Namespace::Shared => String::new(),
    };
    format!(
        "namespace Perf{suffix}\n\
         {{\n\
         public class Clone\n\
         {{\n\
         public int Sum(int limit)\n\
         {{\n\
         int total = 0;\n\
         for (int i = 0; i < limit; i = i + 1) {{ total = total + i; }}\n\
         return total;\n\
         }}\n\
         }}\n\
         }}\n"
    )
}

fn embedding_provenance(tmp: &Path) -> Result<Value> {
    let mut path: PathBuf = tmp.join("report");
    let _replaced = path.set_extension("json");
    let report: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    report
        .get("embedding_provenance")
        .cloned()
        .ok_or_else(|| anyhow!("embedding_provenance missing: {report}"))
}

fn metric(provenance: &Value, field: &str) -> u64 {
    provenance
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}
