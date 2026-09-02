//! End-to-end regression coverage for the Type-1 / Type-2 distinction:
//! a byte-identical copy and a renamed-identifier copy are different
//! findings, and the report must prove each with the right byte fact
//! ([CLONE-BUCKETS-IDENTICAL], [PIPELINE-CLUSTER-CLOSURE]).
//!
//! Acceptance: csharp-small (two methods, same structure, renamed
//! identifiers) must contain no byte-identical pair, and its two method
//! bodies must relate by exactly the authored identifier renames.
//! csharp-type1 (two methods that are byte-identical) must report
//! occurrences whose method slices are byte-equal to each other.

use anyhow::{anyhow, Result};

use crate::common::signals::has_verbatim_pair;
use crate::common::*;

/// The Type-1 method signature every reported occurrence must carry.
const TALLY_SIGNATURE: &str = "public int Tally(int bound)";

/// The tail statement of the Type-1 method body, guarding the slice
/// against ending before the body does.
const TALLY_BODY_TAIL: &str = "return total;";

/// The Type-2 method signatures — one per renamed copy.
const RUN_SIGNATURE: &str = "public int Run(int limit)";
const COMPUTE_SIGNATURE: &str = "public int Compute(int input)";

/// The authored identifier renames between the two Type-2 bodies, in
/// application order. Every needle is a whole token in the source, so
/// plain substitution models the rename mapping exactly.
const TYPE2_RENAMES: [(&str, &str); 4] = [
    ("Run", "Compute"),
    ("limit", "input"),
    ("accumulator", "total"),
    ("position", "index"),
];

/// The two Type-1 fixture files the cluster must span, sorted.
const TYPE1_FILES: [&str; 2] = ["Eta.cs", "Zeta.cs"];

/// The two Type-2 fixture files the cluster must span, sorted.
const TYPE2_FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];

/// Slices `text` from `signature` through the method's closing brace.
fn method_slice(text: &str, signature: &str) -> Result<String> {
    let start = text
        .find(signature)
        .ok_or_else(|| anyhow!("signature {signature:?} must appear in the occurrence"))?;
    let body_open = text[start..]
        .find('{')
        .ok_or_else(|| anyhow!("method body must open after {signature:?}"))?;
    let mut depth = 0_usize;
    for (offset, character) in text[start + body_open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("unbalanced braces after {signature:?}"))?;
                if depth == 0 {
                    let end = start + body_open + offset + character.len_utf8();
                    return Ok(text[start..end].to_owned());
                }
            }
            _ => {}
        }
    }
    Err(anyhow!("method body after {signature:?} never closes"))
}

/// Applies the authored Type-2 renames, in order.
fn apply_type2_renames(text: &str) -> String {
    let mut renamed = text.to_owned();
    for (from, to) in TYPE2_RENAMES {
        assert!(
            renamed.contains(from),
            "rename source {from:?} must occur in the Type-2 body: {renamed}"
        );
        renamed = renamed.replace(from, to);
    }
    renamed
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

    assert!(
        !clusters(&type2_report).is_empty(),
        "csharp-small must produce at least one cluster: {type2_report}",
    );
    let type1_cluster = clusters(&type1_report)
        .first()
        .ok_or_else(|| anyhow!("csharp-type1 must produce at least one cluster: {type1_report}"))?;

    // Both clusters report exactly the fixture's two copies, once each.
    for (report, expected_files, label) in [
        (&type2_report, TYPE2_FILES, "Type-2"),
        (&type1_report, TYPE1_FILES, "Type-1"),
    ] {
        let cluster = clusters(report).first().ok_or_else(|| {
            anyhow!("{label} fixture must produce at least one cluster: {report}")
        })?;
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

    // Type-2: renamed-identifier copies must not be byte-proven, and the
    // two method bodies must relate by exactly the authored renames.
    for cluster in clusters(&type2_report) {
        assert!(
            !has_verbatim_pair(&type2_scan, cluster)?,
            "Type-2 cluster must not be a byte-identical copy: {cluster:#}",
        );
    }
    let type2_texts = occurrence_texts(
        &type2_scan,
        clusters(&type2_report)
            .first()
            .ok_or_else(|| anyhow!("csharp-small must produce a cluster: {type2_report}"))?,
    )?;
    let run_slice = type2_texts
        .iter()
        .find(|text| text.contains(RUN_SIGNATURE))
        .ok_or_else(|| anyhow!("a Type-2 occurrence must carry the Run method"))?;
    let compute_slice = type2_texts
        .iter()
        .find(|text| text.contains(COMPUTE_SIGNATURE))
        .ok_or_else(|| anyhow!("a Type-2 occurrence must carry the Compute method"))?;
    let run_body = method_slice(run_slice, RUN_SIGNATURE)?;
    let compute_body = method_slice(compute_slice, COMPUTE_SIGNATURE)?;
    assert_ne!(
        run_body, compute_body,
        "Type-2 bodies must differ in raw bytes: {run_body:?} vs {compute_body:?}"
    );
    assert_eq!(
        apply_type2_renames(&run_body),
        compute_body,
        "the Type-2 bodies must differ by exactly the authored identifier \
         renames: {run_body:?} vs {compute_body:?}",
    );

    // Type-1: every reported occurrence slices to the SAME method bytes —
    // the strongest byte fact for a genuine byte-identical copy.
    let type1_texts = occurrence_texts(&type1_scan, type1_cluster)?;
    let mut slices = Vec::with_capacity(type1_texts.len());
    for text in &type1_texts {
        let body = method_slice(text, TALLY_SIGNATURE)?;
        assert!(
            body.contains(TALLY_BODY_TAIL),
            "the Type-1 slice must cover the whole method body: {body:?}",
        );
        slices.push(body);
    }
    assert!(
        slices.len() >= 2,
        "both Type-1 copies must be reported: {slices:#?}",
    );
    for slice in &slices[1..] {
        assert_eq!(
            slices[0], *slice,
            "Type-1 occurrences must slice to byte-identical methods: {slices:#?}",
        );
    }

    Ok(())
}
