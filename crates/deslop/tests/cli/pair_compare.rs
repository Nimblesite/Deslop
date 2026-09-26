//! [PAIR-COMPARE-CLI] `--compare` answers with the engine's own verdict on
//! exactly two occurrences, and refuses anything else.
//!
//! Evidence is pair-scoped and recomputed on demand — never stored on a
//! cluster and never carried in a rendered report ([FUSED-PAIR-SIGNALS]).
//! The LSP (`pair/compare`) and MCP (`compare_pair`) surfaces could ask
//! for it; the CLI could not, so the corpus gate — which drives the CLI
//! black-box — had no way to tell a curated duplicate the engine admitted
//! from one it merely reported ([CORPUS-RECALL]).

use std::{path::Path, process::Output};

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::support::*;
use crate::common::{
    clone_corpus::{DUPLICATE_FN, MIN_NODES},
    run_report,
    scan_dir::temp_scan_dir,
    write_identical_pair, IDENTICAL_KIND, NEARLY_IDENTICAL_KIND,
};

const COMPARE_FLAG: &str = "--compare";
const EMBEDDINGS_FLAG: &str = "--embeddings";
const EMBEDDINGS_OFF: &str = "off";
const RUST_EXTENSION: &str = "rs";
const SCAN_DIR: &str = "tree";

/// Every field an engine-authored pair verdict carries ([FUSED-PAIR-SIGNALS]).
const EVIDENCE_FIELDS: [&str; 14] = [
    "structural",
    "token_jaccard",
    "embedding_cos",
    "content_measurement",
    "agreement",
    "rename_consistency",
    "literal_fraction",
    "text_identity",
    "fused_score",
    "content_required",
    "content_ok",
    "admitted",
    "classification",
    "explanation",
];
/// Fields whose values are shares, so they must sit inside the unit interval.
const UNIT_INTERVAL_FIELDS: [&str; 6] = [
    "structural",
    "token_jaccard",
    "agreement",
    "rename_consistency",
    "literal_fraction",
    "fused_score",
];
/// The share two byte-identical copies score on every shape and token axis.
const WHOLE: f64 = 1.0;
const BYTE_IDENTICAL_TEXT: &str = "byte_identical";
const DIFFERENT_TEXT: &str = "different";

/// The wording each refusal must carry, so a caller knows what to fix.
const NEEDS_TWO_ENDPOINTS: &str = "exactly 2 endpoints";
const NEEDS_DISTINCT_ENDPOINTS: &str = "two distinct endpoints";
const UNKNOWN_ENDPOINT: &str = "unknown pair endpoint";
/// A one-byte range inside a real file that no fingerprint covers.
const RANGE_THE_SCAN_NEVER_FINGERPRINTED: &str = "a.rs:1:2";

/// The `<path>:<start_byte>:<end_byte>` triple of one rendered occurrence
/// — or of one echoed endpoint, which carries the same three fields.
fn endpoint(occurrence: &Value) -> Result<String> {
    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("no `path` in {occurrence}"))?;
    let byte = |name: &str| {
        occurrence
            .get(name)
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("no `{name}` in {occurrence}"))
    };
    Ok(format!(
        "{path}:{}:{}",
        byte("start_byte")?,
        byte("end_byte")?
    ))
}

/// The endpoints of the first two occurrences of the top-ranked cluster,
/// read out of a rendered report exactly as the corpus gate reads them.
fn first_pair(report: &Value) -> Result<(String, String)> {
    let occurrences = report
        .get("clusters")
        .and_then(Value::as_array)
        .and_then(|clusters| clusters.first())
        .and_then(|cluster| cluster.get("occurrences"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("the scan must report a cluster to compare: {report}"))?;
    let [first, second, ..] = occurrences.as_slice() else {
        return Err(anyhow!("a pair needs two occurrences, got {occurrences:?}"));
    };
    Ok((endpoint(first)?, endpoint(second)?))
}

/// Runs `deslop <scan_root> --compare <e> ...` for every endpoint given.
fn compare(scan_root: &Path, output: &Path, endpoints: &[&str]) -> Result<Output> {
    let mut command = deslop_command(scan_root, output)?;
    let _args = command.args([
        NO_INCREMENTAL_FLAG,
        MIN_NODES_FLAG,
        MIN_NODES_VALUE,
        EMBEDDINGS_FLAG,
        EMBEDDINGS_OFF,
    ]);
    for endpoint in endpoints {
        let _arg = command.arg(COMPARE_FLAG).arg(endpoint);
    }
    Ok(command.output()?)
}

/// The one JSON verdict a successful comparison prints, and nothing else.
fn verdict(run: &Output) -> Result<Value> {
    assert!(
        run.status.success(),
        "a well-formed comparison must succeed; stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8(run.stdout.clone())?;
    serde_json::from_str(stdout.trim()).map_err(|error| {
        anyhow!("`--compare` must print one JSON pair verdict on stdout and nothing else: {error}. Got: {stdout}")
    })
}

/// The evidence object of a verdict, with every field the wire model
/// promises and every share inside the unit interval.
fn evidence(verdict: &Value) -> Result<&Value> {
    let evidence = verdict
        .get("evidence")
        .ok_or_else(|| anyhow!("the verdict must carry `evidence`: {verdict}"))?;
    for field in EVIDENCE_FIELDS {
        assert!(
            evidence.get(field).is_some(),
            "pair evidence must carry `{field}` — the corpus gate reads exactly these to tell \
             an admitted duplicate from one merely reported: {evidence}"
        );
    }
    for field in UNIT_INTERVAL_FIELDS {
        let share = evidence
            .get(field)
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("`{field}` must be a number: {evidence}"))?;
        assert!(
            (0.0..=WHOLE).contains(&share),
            "`{field}` is a share and must sit in 0..=1, got {share}"
        );
    }
    Ok(evidence)
}

/// A refused comparison exits non-zero and says why on stderr.
fn assert_refused(run: &Output, reason: &str, context: &str) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        !run.status.success(),
        "{context} must be refused, yet the CLI exited successfully with stdout: {}",
        String::from_utf8_lossy(&run.stdout)
    );
    assert!(
        stderr.contains(reason),
        "{context}: stderr must say `{reason}` so the caller knows what to fix; got: {stderr}"
    );
    assert!(
        run.stdout.is_empty(),
        "{context}: a refused comparison prints no verdict, got: {}",
        String::from_utf8_lossy(&run.stdout)
    );
}

#[test]
fn the_cli_reports_the_engines_verdict_for_two_named_occurrences() -> Result<()> {
    assert_eq!(
        MIN_NODES.to_string(),
        MIN_NODES_VALUE,
        "the report the endpoints are read from and the comparison must share one node floor"
    );
    let (tmp, scan_root) = temp_scan_dir(SCAN_DIR)?;
    write_identical_pair(&scan_root, RUST_EXTENSION, DUPLICATE_FN)?;
    let report = run_report(&scan_root, MIN_NODES)?;
    let (left, right) = first_pair(&report)?;
    let output = tmp.path().join(REPORT_OUTPUT_STEM);

    let run = compare(&scan_root, &output, &[&left, &right])?;
    let verdict = verdict(&run)?;
    for (side, asked) in [("left", &left), ("right", &right)] {
        let echoed = verdict
            .get(side)
            .ok_or_else(|| anyhow!("the verdict must echo `{side}`: {verdict}"))?;
        assert_eq!(
            &endpoint(echoed)?,
            asked,
            "the verdict must echo the `{side}` endpoint it was asked about, so a caller can \
             never mistake which pair was measured"
        );
    }
    let evidence = evidence(&verdict)?;
    assert_eq!(
        evidence.get("admitted"),
        Some(&Value::Bool(true)),
        "two byte-identical copies of one function are an admitted pair; anything else means \
         the engine did not admit its own duplicate: {evidence}"
    );
    assert_eq!(
        evidence.get("classification").and_then(Value::as_str),
        Some(IDENTICAL_KIND),
        "byte-identical copies are the identical kind, and the verdict is where the corpus \
         gate reads that from: {evidence}"
    );
    assert_eq!(
        evidence.get("text_identity").and_then(Value::as_str),
        Some(BYTE_IDENTICAL_TEXT),
        "the two raw ranges are the same bytes: {evidence}"
    );
    for axis in ["structural", "token_jaccard"] {
        assert_eq!(
            evidence.get(axis).and_then(Value::as_f64),
            Some(WHOLE),
            "identical subtrees share every node and every token, so `{axis}` is whole: {evidence}"
        );
    }
    assert!(
        !output.with_extension("json").exists(),
        "a comparison answers on stdout and renders no report"
    );

    assert_refused(
        &compare(&scan_root, &output, &[&left, &left])?,
        NEEDS_DISTINCT_ENDPOINTS,
        "the same endpoint twice",
    );
    assert_refused(
        &compare(&scan_root, &output, &[&left])?,
        NEEDS_TWO_ENDPOINTS,
        "a single endpoint",
    );
    assert_refused(
        &compare(
            &scan_root,
            &output,
            &[&left, RANGE_THE_SCAN_NEVER_FINGERPRINTED],
        )?,
        UNKNOWN_ENDPOINT,
        "a range the scan never fingerprinted",
    );
    Ok(())
}

#[test]
fn a_systematic_rename_is_admitted_as_nearly_identical() -> Result<()> {
    let (tmp, scan_root) = temp_scan_dir(SCAN_DIR)?;
    let _lines = write_clone_pair(&scan_root)?;
    let report = run_report(&scan_root, MIN_NODES)?;
    let (left, right) = first_pair(&report)?;
    let output = tmp.path().join(REPORT_OUTPUT_STEM);
    let run = compare(&scan_root, &output, &[&left, &right])?;
    let verdict = verdict(&run)?;
    let evidence = evidence(&verdict)?;
    assert_eq!(
        evidence.get("admitted"),
        Some(&Value::Bool(true)),
        "one method copied with its identifiers renamed is an admitted pair: {evidence}"
    );
    assert_eq!(
        evidence.get("classification").and_then(Value::as_str),
        Some(NEARLY_IDENTICAL_KIND),
        "a rename is the nearly-identical kind: {evidence}"
    );
    assert_eq!(
        evidence.get("text_identity").and_then(Value::as_str),
        Some(DIFFERENT_TEXT),
        "renamed identifiers make the raw ranges differ: {evidence}"
    );
    let structural = evidence
        .get("structural")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("`structural` must be a number: {evidence}"))?;
    assert!(
        structural > 0.0,
        "a renamed copy keeps its shape, so structural overlap is not zero: {evidence}"
    );
    Ok(())
}
