//! End-to-end regression coverage for the Type-1 / Type-2 distinction:
//! a byte-identical copy and a renamed-identifier copy are different
//! findings, and the report must prove each with the right byte fact
//! ([CLONE-BUCKETS-IDENTICAL], [PIPELINE-CLUSTER-CLOSURE]).
//!
//! Every slice comes from the report's own `start_byte`/`end_byte`
//! occurrence ranges — the engine's byte facts — never from re-parsing
//! the source. Acceptance: csharp-type1 (two byte-identical methods)
//! must report occurrence ranges slicing to byte-equal text, while
//! csharp-small's two renamed methods must differ in raw bytes and
//! relate by exactly the authored identifier renames.

use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::signals::has_verbatim_pair;
use crate::common::*;

/// The Type-1 method signature every reported occurrence must carry.
const TALLY_SIGNATURE: &str = "public int Tally(int bound)";

/// The tail statement of the Type-1 method body, guarding the reported
/// range against ending before the body does.
const TALLY_BODY_TAIL: &str = "return total;";

/// The Type-2 method signatures — one per renamed copy.
const RUN_SIGNATURE: &str = "public int Run(int limit)";
const COMPUTE_SIGNATURE: &str = "public int Compute(int input)";

/// The authored identifier renames between the two Type-2 copies, in
/// application order. The report window spans the enclosing namespace
/// and class, so the mapping covers those identifiers as well. Every
/// needle is a whole token in the source, so plain substitution models
/// the rename mapping exactly.
const TYPE2_RENAMES: [(&str, &str); 6] = [
    ("Beta", "Alpha"),
    ("Summer", "Processor"),
    ("Run", "Compute"),
    ("limit", "input"),
    ("accumulator", "total"),
    ("position", "index"),
];

/// The two Type-1 fixture files the cluster must span, sorted.
const TYPE1_FILES: [&str; 2] = ["Eta.cs", "Zeta.cs"];

/// The authored wrapper renames between the two Type-1 files —
/// namespace and class identifiers only. The method region carries NO
/// renames: that absence is what makes the copy Type-1.
const TYPE1_WRAPPER_RENAMES: [(&str, &str); 2] = [("Eta", "Zeta"), ("Beacon", "Anchor")];

/// The two Type-2 fixture files the cluster must span, sorted.
const TYPE2_FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];

/// Applies an authored rename mapping, in order.
fn apply_renames(text: &str, renames: &[(&str, &str)]) -> String {
    let mut mapped = text.to_owned();
    for (from, to) in renames {
        assert!(
            mapped.contains(from),
            "rename source {from:?} must occur in the window: {mapped}"
        );
        mapped = mapped.replace(from, to);
    }
    mapped
}

/// The report window's method region: everything from the authored
/// signature anchor through the authored tail statement. Both copies
/// are cut at the same anchors, so region equality is a byte fact
/// about the duplicated code — no re-parsing of the source.
fn method_region(window: &str) -> Result<String> {
    let (_, after_signature) = window
        .split_once(TALLY_SIGNATURE)
        .ok_or_else(|| anyhow!("window must carry the Tally signature: {window:?}"))?;
    let (body, _) = after_signature
        .split_once(TALLY_BODY_TAIL)
        .ok_or_else(|| anyhow!("window must carry the method tail: {window:?}"))?;
    Ok(format!("{TALLY_SIGNATURE}{body}{TALLY_BODY_TAIL}"))
}

/// Asserts the cluster reports exactly the fixture's two copies, once
/// each, spanning `expected_files`.
fn assert_two_copy_cluster(cluster: &Value, expected_files: &[&str; 2], label: &str) {
    let mut files = occurrence_files(cluster);
    files.sort();
    files.dedup();
    assert_eq!(files, expected_files, "{label} cluster file set");
    assert_eq!(
        cluster
            .pointer("/occurrence_count")
            .and_then(serde_json::Value::as_u64),
        Some(2),
        "{label} fixture must publish exactly two occurrences: {cluster:#}",
    );
}

/// Asserts the Type-2 report-sliced bodies differ in raw bytes and
/// relate by exactly the authored renames ([CLONE-BUCKETS-IDENTICAL]).
fn assert_type2_rename_relation(type2_scan: &Path, type2_report: &Value) -> Result<()> {
    for cluster in clusters(type2_report) {
        assert!(
            !has_verbatim_pair(type2_scan, cluster)?,
            "Type-2 cluster must not be a byte-identical copy: {cluster:#}",
        );
    }
    let texts = occurrence_texts(
        type2_scan,
        clusters(type2_report)
            .first()
            .ok_or_else(|| anyhow!("csharp-small must produce a cluster: {type2_report}"))?,
    )?;
    let run_text = texts
        .iter()
        .find(|text| text.contains(RUN_SIGNATURE))
        .ok_or_else(|| anyhow!("a Type-2 occurrence must carry the Run method: {texts:#?}"))?;
    let compute_text = texts
        .iter()
        .find(|text| text.contains(COMPUTE_SIGNATURE))
        .ok_or_else(|| anyhow!("a Type-2 occurrence must carry the Compute method: {texts:#?}"))?;
    assert_ne!(
        run_text, compute_text,
        "Type-2 bodies must differ in raw bytes: {run_text:?} vs {compute_text:?}"
    );
    assert_eq!(
        apply_renames(run_text, &TYPE2_RENAMES),
        compute_text.as_str(),
        "the Type-2 bodies must differ by exactly the authored identifier \
         renames: {run_text:?} vs {compute_text:?}",
    );
    Ok(())
}

/// Asserts every Type-1 occurrence range slices to the SAME method
/// bytes — the strongest byte fact for a genuine byte-identical copy —
/// and that the windows relate by exactly the authored wrapper
/// renames, so the divergence is namespace/class identifiers only.
fn assert_type1_byte_equal(type1_scan: &Path, type1_report: &Value) -> Result<()> {
    let cluster = clusters(type1_report)
        .first()
        .ok_or_else(|| anyhow!("csharp-type1 must produce at least one cluster: {type1_report}"))?;
    let texts = occurrence_texts(type1_scan, cluster)?;
    let first = texts
        .first()
        .ok_or_else(|| anyhow!("the Type-1 cluster must report occurrences: {cluster:#}"))?;
    let first_region = method_region(first)?;
    assert!(
        !first_region.is_empty(),
        "the reported Type-1 range must cover the whole method body: {first:?}",
    );
    for text in texts.iter().skip(1) {
        assert_eq!(
            first_region,
            method_region(text)?,
            "Type-1 occurrence ranges must slice to byte-identical methods: {texts:#?}",
        );
        assert_eq!(
            apply_renames(first, &TYPE1_WRAPPER_RENAMES),
            *text,
            "the Type-1 windows must differ by exactly the authored wrapper              renames: {first:?} vs {text:?}",
        );
    }
    Ok(())
}

/// A Type-1 copy slices to byte-identical methods; a Type-2 rename does
/// not, and its two bodies relate by exactly the authored renames
/// ([CLONE-BUCKETS-IDENTICAL]).
#[test]
fn type1_copies_slice_to_identical_bytes_while_type2_renames_do_not() -> Result<()> {
    let type2_scan = fixture("csharp-small");
    let type2_report = run_report(&type2_scan, 30)?;
    let type1_scan = fixture("csharp-type1");
    let type1_report = run_report(&type1_scan, 30)?;

    assert_two_copy_cluster(
        clusters(&type2_report).first().ok_or_else(|| {
            anyhow!("csharp-small must produce at least one cluster: {type2_report}")
        })?,
        &TYPE2_FILES,
        "Type-2",
    );
    assert_two_copy_cluster(
        clusters(&type1_report).first().ok_or_else(|| {
            anyhow!("csharp-type1 must produce at least one cluster: {type1_report}")
        })?,
        &TYPE1_FILES,
        "Type-1",
    );

    assert_type2_rename_relation(&type2_scan, &type2_report)?;
    assert_type1_byte_equal(&type1_scan, &type1_report)?;

    Ok(())
}
