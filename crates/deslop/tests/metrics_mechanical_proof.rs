//! [METRICS-REPO] / [METRICS-REPO-WEIGHTED] / [CLONE-KIND-TESTING] One
//! mechanical duplication figure, proved on a mixed corpus.
//!
//! The corpus holds one true clone pair, two clean files and one
//! shape-only pair ([CLONE-BUCKETS-STRUCTURAL-ONLY]). Every figure below
//! is hand-counted from those files. The suite pins the repository,
//! file, folder and diff figures, the one gate, the rendered text and
//! HTML, and that neither pair evidence nor presentation can move any of
//! them.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::{
    cli::bucket_groups::information::SHAPE_ONLY_DISJOINT, common::*,
    metrics_folder_rollup::seed_nested_corpus,
};

/// The scan root's directory name; diff paths are written against it.
const SCAN_DIR: &str = "repo";
/// The clone pair clusters at this size ([METRICS-REPO] fixtures).
const MIN_NODES: &str = "8";
/// Where the shape-only pair is seeded: the otherwise clean `lib` folder.
const SHAPES_FILE: &str = "lib/shapes.rs";
/// 8 + 12 + 8 + 7 + 23 analysed lines across the five files.
const ANALYSED_LOC: u64 = 58;
/// The seven-line clone body, counted once in each of its two files.
const DUPLICATED_LOC: u64 = 14;
/// The clone pair is the only clone group; the shape-only pair is not one.
const CLONE_GROUPS: u64 = 1;
/// Only `alpha.rs` and `beta.rs` hold a duplicated line.
const DUPLICATED_FILES: u64 = 2;
/// `(path, analysed_loc, duplicated_loc)` per file, in reported order.
const FILE_ROWS: [(&str, u64, u64); 5] = [
    ("src/a/alpha.rs", 8, 7),
    ("src/b/beta.rs", 8, 7),
    ("lib/gamma.rs", 7, 0),
    (SHAPES_FILE, 23, 0),
    ("src/a/clean.rs", 12, 0),
];
/// `(path, analysed_loc, duplicated_loc)` per folder, worst first. `lib`
/// holds the shape-only pair and no clone, so it has no row.
const FOLDER_ROWS: [(&str, u64, u64); 3] = [("src/b", 8, 7), ("src", 28, 14), ("src/a", 20, 7)];
/// The clone body's lines in each copy, below the one-line file comment.
const CLONE_LINES: (u64, u64) = (2, 8);
/// The two shape-only functions' lines in `lib/shapes.rs`.
const SHAPE_LINES: [(u64, u64); 2] = [(1, 11), (13, 23)];
/// The headline every text and HTML report states: 100 × 14 / 58.
const HEADLINE: &str = "24.1% duplicated (14 / 58 LOC, 1 clusters across 2 files)";
/// The phrase that marks a repository percentage on a rendered surface.
const PERCENT_PHRASE: &str = "% duplicated (";
/// A gate just above and just below the 24.14% figure.
const GATE_ABOVE: &str = "24.2";
const GATE_BELOW: &str = "24.1";
/// [EXIT-CODES] A breached gate exits three; a clean run exits zero.
const EXIT_OK: i32 = 0;
const EXIT_BREACHED: i32 = 3;
/// `beta.rs` is eight added lines, seven of them the clone body.
const DIFF_ADDED_LOC: u64 = 8;
const DIFF_DUPLICATED_LOC: u64 = 7;
const DIFF_HEADLINE: &str = "diff: 87.5% of added lines duplicated (7 / 8 added LOC)";
/// The metric fields that state the mechanical figure. `per_kind` and
/// `threshold` describe classification and the gate, not coverage.
const FIGURE_FIELDS: [&str; 7] = [
    "analysed_loc",
    "duplicated_loc",
    "duplication_percent",
    "clusters_total",
    "duplicated_files",
    "per_file",
    "folders",
];
/// Consistent renames applied to `beta.rs`: a different relation with
/// different pair evidence that covers the same lines.
const BETA_RENAMES: [(&str, &str); 4] = [
    ("items", "entries"),
    ("item", "entry"),
    ("total", "sum"),
    ("compute", "tally"),
];
/// Literal swaps inside the shape-only pair: its evidence changes, its
/// shape and its line count do not.
const SHAPE_LITERAL_SWAPS: [(&str, &str); 2] = [("invoice", "voucher"), ("nightly", "hourly")];
const IDENTICAL_KIND: &str = "identical";
const NEARLY_IDENTICAL_KIND: &str = "nearly_identical";
/// Every `.td` model and both generated wire modules, from the workspace root.
const WIRE_MODELS: [&str; 3] = [
    "docs/models/live-ipc.td",
    "crates/deslop-core/src/wire_generated.rs",
    "clients/vscode/src/types/wire-generated.ts",
];
/// The stem every retired evidence-weighted name carried.
const RETIRED_STEM: &str = "weight";

/// One CLI run: its exit code and the three reports it rendered.
struct Run {
    code: i32,
    report: Value,
    text: String,
    html: String,
}

/// Seeds the mixed corpus in a fresh directory and returns it with the
/// scan root inside it.
fn mixed_corpus() -> Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join(SCAN_DIR);
    seed_nested_corpus(&scan_root)?;
    fs::write(scan_root.join(SHAPES_FILE), SHAPE_ONLY_DISJOINT)?;
    Ok((tmp, scan_root))
}

/// Scans the corpus under `tmp` with `extra` flags, from `tmp` so diff
/// paths resolve as they do in CI ([CLI-ARG-DIFF]).
fn scan(tmp: &Path, name: &str, extra: &[&str]) -> Result<Run> {
    let prefix = tmp.join(name);
    let mut command = deslop_cmd(&tmp.join(SCAN_DIR), &prefix)?;
    let output = command
        .current_dir(tmp)
        .args(["--min-nodes", MIN_NODES, "--embeddings", "off"])
        .args(extra)
        .output()?;
    Ok(Run {
        code: output
            .status
            .code()
            .ok_or_else(|| anyhow!("no exit code"))?,
        report: load_json(&prefix.with_extension("json"))?,
        text: fs::read_to_string(prefix.with_extension("txt"))?,
        html: fs::read_to_string(prefix.with_extension("html"))?,
    })
}

/// The mechanical figure alone, for whole-value comparison between runs.
fn figures(report: &Value) -> Vec<&Value> {
    FIGURE_FIELDS
        .iter()
        .map(|name| metric_field(report, name))
        .collect()
}

/// Asserts `metrics.<table>` holds exactly `expected`, in order, each
/// percentage being 100 × duplicated / analysed.
fn assert_rows(report: &Value, table: &str, expected: &[(&str, u64, u64)]) {
    let rows = metric_field(report, table)
        .as_array()
        .cloned()
        .unwrap_or_default();
    let paths: Vec<&str> = rows
        .iter()
        .filter_map(|row| field(row, "path").as_str())
        .collect();
    let expected_paths: Vec<&str> = expected.iter().map(|(path, ..)| *path).collect();
    assert_eq!(paths, expected_paths, "{table} paths and order: {rows:#?}");
    for (row, (path, analysed, duplicated)) in rows.iter().zip(expected) {
        assert_eq!(
            field(row, "analysed_loc").as_u64(),
            Some(*analysed),
            "{path} analysed"
        );
        assert_eq!(
            field(row, "duplicated_loc").as_u64(),
            Some(*duplicated),
            "{path} duplicated"
        );
        assert_percent(
            field(row, "duplication_percent"),
            *duplicated,
            *analysed,
            path,
        );
    }
}

/// Asserts `value` is 100 × `duplicated` / `analysed`.
fn assert_percent(value: &Value, duplicated: u64, analysed: u64, label: &str) {
    let expected = 100.0 * f64::from(u32::try_from(duplicated).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(analysed).unwrap_or(u32::MAX));
    assert!(
        value
            .as_f64()
            .is_some_and(|percent| approx(percent, expected)),
        "{label}: {value} must be 100 × {duplicated} / {analysed} = {expected}"
    );
}

/// Asserts the repository totals hand-counted from the corpus.
fn assert_repository_figures(report: &Value) {
    assert_eq!(
        metric_field(report, "analysed_loc").as_u64(),
        Some(ANALYSED_LOC)
    );
    assert_eq!(
        metric_field(report, "duplicated_loc").as_u64(),
        Some(DUPLICATED_LOC)
    );
    assert_eq!(
        metric_field(report, "clusters_total").as_u64(),
        Some(CLONE_GROUPS)
    );
    assert_eq!(
        metric_field(report, "duplicated_files").as_u64(),
        Some(DUPLICATED_FILES)
    );
    assert_percent(
        metric_field(report, "duplication_percent"),
        DUPLICATED_LOC,
        ANALYSED_LOC,
        "repository",
    );
    assert_rows(report, "per_file", &FILE_ROWS);
    assert_rows(report, "folders", &FOLDER_ROWS);
}

/// `(path, start_line, end_line)` for each occurrence of `cluster`.
fn spans(cluster: &Value) -> Vec<(&str, u64, u64)> {
    field(cluster, "occurrences")
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|occurrence| {
            Some((
                field(occurrence, "path").as_str()?,
                field(occurrence, "start_line").as_u64()?,
                field(occurrence, "end_line").as_u64()?,
            ))
        })
        .collect()
}

/// Asserts the two findings: the clone pair ranked first with `kind`,
/// then the shape-only pair as information with no mass.
fn assert_findings(report: &Value, kind: &str) -> Result<()> {
    let [clone, information] = clusters(report) else {
        return Err(anyhow!(
            "one clone group and one informational finding: {report:#}"
        ));
    };
    assert_eq!(cluster_kind(clone), kind, "the clone pair's kind");
    assert_eq!(field(clone, "rank").as_u64(), Some(CLONE_GROUPS));
    assert_eq!(
        spans(clone),
        vec![
            ("src/a/alpha.rs", CLONE_LINES.0, CLONE_LINES.1),
            ("src/b/beta.rs", CLONE_LINES.0, CLONE_LINES.1),
        ]
    );
    assert_eq!(cluster_kind(information), STRUCTURAL_ONLY_KIND);
    assert_eq!(
        field(information, "mass").as_u64(),
        Some(0),
        "information carries no mass"
    );
    let shapes: Vec<_> = SHAPE_LINES
        .iter()
        .map(|(start, end)| (SHAPES_FILE, *start, *end))
        .collect();
    assert_eq!(spans(information), shapes);
    Ok(())
}

/// Asserts the text and HTML reports each state the one headline once.
fn assert_rendered_headline(run: &Run) {
    for (surface, body) in [("text", &run.text), ("html", &run.html)] {
        assert_contains(
            body,
            HEADLINE,
            &format!("the {surface} report states the engine's figure"),
        );
        assert_eq!(
            body.matches(PERCENT_PHRASE).count(),
            1,
            "the {surface} report states one repository percentage"
        );
    }
}

/// Asserts the run's exit code and the gate block the report records.
fn assert_gate(run: &Run, code: i32, source: &str, breached: bool) {
    let threshold = metric_field(&run.report, "threshold");
    assert_eq!(run.code, code, "exit code for a {source} gate: {threshold}");
    assert_eq!(field(threshold, "source").as_str(), Some(source));
    assert_eq!(field(threshold, "breached").as_bool(), Some(breached));
}

/// [METRICS-REPO] [CLONE-KIND-TESTING] A corpus mixing a true clone with
/// a shape-only pair counts the clone's lines once and nothing else, on
/// the JSON, text and HTML reports alike.
#[test]
fn a_mixed_corpus_counts_only_its_clone_on_every_surface() -> Result<()> {
    let (tmp, _scan_root) = mixed_corpus()?;
    let run = scan(tmp.path(), "plain", &[])?;
    assert_repository_figures(&run.report);
    assert_findings(&run.report, IDENTICAL_KIND)?;
    assert_gate(&run, EXIT_OK, "none", false);
    assert_rendered_headline(&run);
    Ok(())
}

/// [EXIT-CODES] The one gate reads the one percentage: flag or config,
/// breached or not, the figures never move.
#[test]
fn the_one_gate_reads_the_one_percentage_from_flag_or_config() -> Result<()> {
    let (tmp, scan_root) = mixed_corpus()?;
    let within = scan(tmp.path(), "within", &["--fail-over", GATE_ABOVE])?;
    assert_gate(&within, EXIT_OK, "cli", false);
    let over = scan(tmp.path(), "over", &["--fail-over", GATE_BELOW])?;
    assert_gate(&over, EXIT_BREACHED, "cli", true);
    fs::write(
        scan_root.join(".deslop.toml"),
        format!("[threshold]\nmax_duplication_percent = {GATE_BELOW}\n"),
    )?;
    let configured = scan(tmp.path(), "configured", &[])?;
    assert_gate(&configured, EXIT_BREACHED, "config", true);
    let cleared = scan(tmp.path(), "cleared", &["--no-fail-over"])?;
    assert_gate(&cleared, EXIT_OK, "none", false);
    for run in [&within, &over, &configured, &cleared] {
        assert_repository_figures(&run.report);
        assert_rendered_headline(run);
    }
    Ok(())
}

/// Writes a unified diff adding the whole of `beta.rs` and returns its name.
fn write_beta_diff(tmp: &Path, scan_root: &Path) -> Result<&'static str> {
    let path = format!("{SCAN_DIR}/src/b/beta.rs");
    let added: String = fs::read_to_string(scan_root.join("src/b/beta.rs"))?
        .lines()
        .flat_map(|line| ["+", line, "\n"])
        .collect();
    let header = format!(
        "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{DIFF_ADDED_LOC} @@\n"
    );
    fs::write(tmp.join("beta.diff"), format!("{header}{added}"))?;
    Ok("beta.diff")
}

/// [METRICS-DIFF-SCOPE] A diff adds its own figure — seven duplicated of
/// eight added lines — and leaves the repository figure alone.
#[test]
fn a_diff_states_its_own_figure_beside_the_unchanged_repository_figure() -> Result<()> {
    let (tmp, scan_root) = mixed_corpus()?;
    let diff = write_beta_diff(tmp.path(), &scan_root)?;
    let run = scan(tmp.path(), "diffed", &["--diff", diff])?;
    assert_eq!(run.code, EXIT_OK);
    assert_repository_figures(&run.report);
    let scoped = metric_field(&run.report, "diff");
    assert_eq!(field(scoped, "added_loc").as_u64(), Some(DIFF_ADDED_LOC));
    assert_eq!(
        field(scoped, "duplicated_added_loc").as_u64(),
        Some(DIFF_DUPLICATED_LOC)
    );
    assert_percent(
        field(scoped, "duplication_percent"),
        DIFF_DUPLICATED_LOC,
        DIFF_ADDED_LOC,
        "diff",
    );
    assert_rendered_headline(&run);
    for body in [&run.text, &run.html] {
        assert_contains(body, DIFF_HEADLINE, "the diff figure is rendered verbatim");
    }
    Ok(())
}

/// Rewrites `file` under `scan_root`, replacing each whole word in `swaps`.
fn rewrite(scan_root: &Path, file: &str, swaps: &[(&str, &str)]) -> Result<()> {
    let path = scan_root.join(file);
    let rewritten = swaps
        .iter()
        .fold(fs::read_to_string(&path)?, |source, (from, to)| {
            replace_word(&source, from, to)
        });
    Ok(fs::write(path, rewritten)?)
}

/// Replaces `from` with `to` wherever it stands as a whole identifier.
fn replace_word(source: &str, from: &str, to: &str) -> String {
    let is_word = |letter: char| letter.is_alphanumeric() || letter == '_';
    source
        .split_inclusive(|letter: char| !is_word(letter))
        .map(|token| match token.strip_prefix(from) {
            Some(rest) if rest.chars().all(|letter| !is_word(letter)) => format!("{to}{rest}"),
            _ => token.to_owned(),
        })
        .collect()
}

/// Asserts `run` states the `expected` figures and both findings, the
/// clone pair classified as `kind`.
fn assert_unmoved(run: &Run, expected: &[&Value], kind: &str, change: &str) -> Result<()> {
    assert_eq!(figures(&run.report), expected, "{change} moves no figure");
    assert_findings(&run.report, kind)
}

/// [METRICS-REPO-WEIGHTED] Pair evidence and presentation cannot move a
/// figure: a renamed copy is a different kind over the same lines, a
/// shape-only pair with other literals is still information, and no
/// visibility or rendering choice reaches the metrics.
#[test]
fn evidence_and_presentation_changes_leave_every_figure_alone() -> Result<()> {
    let (tmp, scan_root) = mixed_corpus()?;
    let baseline = scan(tmp.path(), "baseline", &[])?;
    let expected = figures(&baseline.report);
    for flags in [&["--technical"][..], &["--split-by-language"][..]] {
        let run = scan(tmp.path(), "rendered", flags)?;
        assert_unmoved(&run, &expected, IDENTICAL_KIND, "a rendering flag")?;
    }
    for choice in ["keep", "ignore"] {
        let config = format!("[visibility]\ndata_clones = \"{choice}\"\n");
        fs::write(scan_root.join(".deslop.toml"), config)?;
        let run = scan(tmp.path(), "visibility", &[])?;
        assert_unmoved(&run, &expected, IDENTICAL_KIND, "a visibility choice")?;
    }
    fs::remove_file(scan_root.join(".deslop.toml"))?;
    rewrite(&scan_root, SHAPES_FILE, &SHAPE_LITERAL_SWAPS)?;
    let reworded = scan(tmp.path(), "reworded", &[])?;
    assert_unmoved(
        &reworded,
        &expected,
        IDENTICAL_KIND,
        "other literals, same shape",
    )?;
    rewrite(&scan_root, "src/b/beta.rs", &BETA_RENAMES)?;
    let renamed = scan(tmp.path(), "renamed", &[])?;
    assert_unmoved(&renamed, &expected, NEARLY_IDENTICAL_KIND, "a renamed copy")?;
    assert_rendered_headline(&renamed);
    Ok(())
}

/// [METRICS-REPO-WEIGHTED] The typeDiagram model and the Rust and
/// TypeScript modules generated from it declare no weighted field, table
/// or type.
#[test]
fn the_wire_models_declare_nothing_weighted() -> Result<()> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for model in WIRE_MODELS {
        let body = fs::read_to_string(workspace.join(model))?;
        assert!(
            body.contains("duplication_percent"),
            "{model} must be the model that declares the metrics"
        );
        assert_not_contains(
            &body.to_lowercase(),
            RETIRED_STEM,
            &format!("{model} declares no evidence-weighted name"),
        );
    }
    Ok(())
}
