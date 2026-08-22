//! The three-file seeded Rust corpus shared by the store-accounting
//! suites (`signature_reuse.rs`, `cache_blob_integrity.rs`): one
//! byte-identical clone pair plus one unrelated file, so cold/warm
//! cache stats are exactly `{0,3}` / `{3,0}` and exactly one
//! `identical` cluster spans the pair. One definition, so the suites
//! can never disagree about what the corpus contains.

use std::{fs, path::Path};

use serde_json::Value;

use super::{
    approx, cluster_bucket, cluster_id, cluster_size, clusters_hidden, expect_cluster_spanning,
    field, metric_field, occurrences, per_file_metrics, signal, visible_duplicated_loc, Result,
};

/// Files the seeded corpus contains, as the `u64` the cache counters use.
pub(crate) const SEEDED_FILE_COUNT: u64 = 3;

/// Subtree-size floor the seeded corpus is analysed at.
pub(crate) const SEEDED_MIN_NODES: u32 = 8;

/// Stable id of the authored `alpha.rs`/`beta.rs` clone
/// ([PIPELINE-DETERMINISM]).
///
/// Re-blessed when [PIPELINE-NORMALIZE-AST-OPERATOR] gave each operator
/// leaf its own kind (`__op__+` rather than a shared `__op__`). The id
/// is the canonical subtree's Merkle hash, and the seven operator
/// leaves below now hash by their own tokens, so the hash moved. Nothing
/// the reader sees moved with it: the spans, bucket, category, node
/// count, signals and metrics asserted in this file are all unchanged,
/// which is what proves the change discriminated operators rather than
/// perturbing the corpus.
const SEEDED_CLONE_ID: &str = "f26a1a0312987ef1";

/// `canonical_node_count` of the authored clone.
///
/// Seven higher than the pre-[PIPELINE-NORMALIZE-AST-OPERATOR] count:
/// the body carries seven behaviour-bearing anonymous tokens — `&` in
/// `&[i32]`, `in`, the `*` of `*item`, `>`, `+=`, the `*` of `item * 2`,
/// and `-=` — and each now survives normalisation as a leaf carrying
/// its own token so `+` and `-` can disagree. The reported spans, bucket, category,
/// signals and metrics are all unchanged; only the canonical tree the
/// count and the id are taken from grew.
const SEEDED_CLONE_NODES: u64 = 47;

/// Where each copy is reported: `(file, start_line, end_line,
/// start_byte, end_byte)`. Both bodies are byte-identical and 171 bytes
/// long; the four-byte offset between them is exactly the difference in
/// their banner comments, which is what makes a swapped or misaddressed
/// blob detectable in the rendered spans at all.
const SEEDED_SPANS: &[(&str, u64, u64, u64, u64)] =
    &[("alpha.rs", 2, 8, 30, 201), ("beta.rs", 2, 8, 26, 197)];

/// Every signal of the authored clone, exactly. Embeddings are off and
/// the two bodies are byte-identical, so all four values are determined.
/// `token_jaccard` is the one the audit watched move under a corrupted
/// signature payload while every other field held
/// ([PIPELINE-INCREMENTAL-INTEGRITY]).
const SEEDED_SIGNALS: &[(&str, f64)] = &[
    ("structural", 1.0),
    ("token_jaccard", 1.0),
    ("embedding_cos", 0.0),
    ("fused", 1.0),
];

/// The clone body shared verbatim by `alpha.rs` and `beta.rs`. Seven
/// lines, byte-identical in both files, so one cluster spanning the
/// pair is guaranteed at `--min-nodes 8`.
const CLONE_BODY: &str = "pub fn compute(items: &[i32]) -> i32 {\n\
    \x20   let mut total = 0;\n\
    \x20   for item in items {\n\
    \x20       if *item > 0 { total += item * 2; } else { total -= item; }\n\
    \x20   }\n\
    \x20   total\n\
}\n";

/// A genuinely different function for `gamma.rs` — real code that
/// duplicates nothing, so the corpus has exactly one clone pair.
const DISTINCT_SOURCE: &str = "pub fn label(count: usize) -> String {\n\
    \x20   match count {\n\
    \x20       0 => \"none\".to_owned(),\n\
    \x20       1 => \"one\".to_owned(),\n\
    \x20       other => format!(\"{other} items\"),\n\
    \x20   }\n\
}\n";

/// Seeds three byte-distinct Rust files: the `alpha.rs`/`beta.rs`
/// clone pair (distinct leading comments keep the file bytes — and so
/// the content-addressed store keys — distinct) plus the unrelated
/// `gamma.rs`.
pub(crate) fn seed_corpus(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    fs::write(
        scan_root.join("alpha.rs"),
        format!("// alpha: the canonical copy.\n{CLONE_BODY}"),
    )?;
    fs::write(
        scan_root.join("beta.rs"),
        format!("// beta: the pasted copy.\n{CLONE_BODY}"),
    )?;
    fs::write(scan_root.join("gamma.rs"), DISTINCT_SOURCE)?;
    Ok(())
}

/// Asserts the report is *exactly* the report this corpus produces —
/// not merely that some cluster spans the pair.
///
/// Every store-accounting scenario compares against this: a damaged,
/// swapped, or corrupt store must still render precisely these ids,
/// spans, signals and metrics ([PIPELINE-INCREMENTAL-INTEGRITY],
/// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]). The audit's regressions
/// were all *plausible* reports — right file pair, wrong span; right
/// span, wrong `token_jaccard` — so a shape-only check would have passed
/// through every one of them.
pub(crate) fn assert_seeded_corpus(report: &Value, label: &str) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(SEEDED_FILE_COUNT),
        "{label} run must analyse all three seeded files: {report}"
    );
    let clone = expect_cluster_spanning(report, &["alpha.rs", "beta.rs"])?;
    assert_clone_identity(clone, label, report);
    assert_clone_spans(clone, label)?;
    assert_clone_signals(clone, label);
    assert_seeded_metrics(report, label);
    Ok(())
}

/// The clone's identity and size: bucket, stable id, category, node
/// count, and exactly two occurrences.
fn assert_clone_identity(clone: &Value, label: &str, report: &Value) {
    assert_eq!(
        cluster_bucket(clone),
        "identical",
        "{label}: the seeded pair is byte-identical code in distinct files: {report}"
    );
    assert_eq!(
        (
            cluster_id(clone),
            cluster_size(clone),
            field(clone, "canonical_node_count").as_u64(),
            field(clone, "category").as_str(),
        ),
        (SEEDED_CLONE_ID, 2, Some(SEEDED_CLONE_NODES), Some("logic"),),
        "{label}: (id, size, canonical_node_count, category) of the authored \
         clone are all user-visible and all determined by the source: {clone:#}"
    );
    assert_eq!(
        clusters_hidden(report),
        1,
        "{label}: the corpus hides exactly one non-actionable cluster; a \
         change here moves what the report shows: {report}"
    );
}

/// Both occurrences' exact line and byte spans, matched by file name so
/// occurrence order cannot mask a swap.
fn assert_clone_spans(clone: &Value, label: &str) -> Result<()> {
    for (file, start_line, end_line, start_byte, end_byte) in SEEDED_SPANS {
        let occurrence = occurrences(clone)
            .iter()
            .find(|occurrence| {
                field(occurrence, "path")
                    .as_str()
                    .is_some_and(|path| path.ends_with(file))
            })
            .ok_or_else(|| anyhow::anyhow!("{label}: no occurrence for {file}: {clone:#}"))?;
        assert_eq!(
            (
                field(occurrence, "start_line").as_u64(),
                field(occurrence, "end_line").as_u64(),
                field(occurrence, "start_byte").as_u64(),
                field(occurrence, "end_byte").as_u64(),
            ),
            (
                Some(*start_line),
                Some(*end_line),
                Some(*start_byte),
                Some(*end_byte)
            ),
            "{label}: {file} must be reported at its authored offsets — a \
             blob served under the wrong address renders exactly here, one \
             banner-comment's width off: {occurrence:#}"
        );
    }
    Ok(())
}

/// All four signals of the authored clone, exactly.
fn assert_clone_signals(clone: &Value, label: &str) {
    for (name, expected) in SEEDED_SIGNALS {
        let actual = signal(clone, name);
        assert!(
            approx(actual, *expected),
            "{label}: signal `{name}` must be {expected}, got {actual} — a \
             signal that moves while the source does not is the \
             corrupted-payload signature: {clone:#}"
        );
    }
}

/// [METRICS-REPO] The corpus's exact figures, plus the arithmetic that
/// connects them: the per-file rows re-summed, the cluster spans
/// re-counted, and the percentage re-divided.
fn assert_seeded_metrics(report: &Value, label: &str) {
    assert_eq!(
        (
            metric_field(report, "analysed_loc").as_u64(),
            metric_field(report, "duplicated_loc").as_u64(),
            metric_field(report, "clusters_total").as_u64(),
            metric_field(report, "duplicated_files").as_u64(),
        ),
        (Some(23), Some(14), Some(1), Some(2)),
        "{label}: the three-file corpus measures 23 analysed / 14 duplicated \
         LOC in 1 cluster across 2 files: {report}"
    );
    let rows = per_file_metrics(report);
    let summed: u64 = rows
        .iter()
        .map(|row| field(row, "duplicated_loc").as_u64().unwrap_or_default())
        .sum();
    assert_eq!(
        (rows.len() as u64, summed, visible_duplicated_loc(report)),
        (SEEDED_FILE_COUNT, 14, 14),
        "{label}: every file needs a row, and duplicated LOC must equal both \
         the row sum and the lines the visible spans cover: {report}"
    );
    let reported = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!(
        approx(reported, 100.0 * 14.0 / 23.0),
        "{label}: duplication_percent must be 14/23 × 100, got {reported}: {report}"
    );
}
