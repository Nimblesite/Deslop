//! [LANG-CAND-OSPREY] Saved manifest selection drives the real watcher pipeline.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::ChildStdin,
    sync::mpsc::Receiver,
    time::Instant,
};

use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};

use crate::{
    common::{
        at, copy_fixture,
        reports::{assert_report_shell, cluster_paths, report_clusters, REPORT_TIMEOUT},
    },
    notifications::{recv_method, request_response, spawn_lsp_with_initial_report, FrameResult},
};

/// Duplicate Python fixture with an optional Osprey manifest.
const FIXTURE: &str = "osprey-python-project";
/// Project selection file observed by the watcher.
const MANIFEST: &str = "osprey.toml";
/// Exact source paths preserved through both transitions.
const FILES: [&str; 2] = ["alpha.py", "beta.py"];
/// Expected number of source files.
const FILE_COUNT: u64 = 2;
/// Expected number of duplicate clusters.
const CLUSTER_COUNT: usize = 1;
/// Expected number of occurrences and per-file metrics.
const OCCURRENCE_COUNT: usize = 2;
/// Expected ranking of the sole cluster.
const FIRST_RANK: u64 = 1;
/// Start of the generation history requested for synchronization.
const ZERO_GENERATION: u64 = 0;
/// Ordinary Python classification without a manifest.
const PYTHON: &str = "python";
/// Osprey Python classification selected by the manifest.
const OSPREY_PYTHON: &str = "osprey-python";
/// Exact-duplicate bucket expected for both sources.
const IDENTICAL: &str = "identical";
/// Read-only report request method.
const REPORT_GET: &str = "deslop/reportGet";
/// Read-only generation history request method.
const REPORT_DELTA: &str = "deslop/reportDelta";
/// Server notification proving the watcher completed analysis.
const REPORT_CHANGED: &str = "deslop/reportChanged";
/// Response result field.
const RESULT: &str = "result";
/// Cluster bucket field.
const KIND: &str = "kind";
/// Cluster ranking field.
const RANK: &str = "rank";
/// Cluster occurrence count field.
const COUNT: &str = "occurrence_count";
/// Classification field shared by occurrences and file metrics.
const LANGUAGE: &str = "language";
/// Source path field in per-file metrics.
const PATH: &str = "path";
/// Cluster occurrence array pointer.
const OCCURRENCES: &str = "/occurrences";
/// Per-file metrics array pointer.
const PER_FILE: &str = "/metrics/per_file";
/// Published notification generation pointer.
const NOTIFIED_GENERATION: &str = "/params/generation";
/// Generation history response pointer.
const DELTA_GENERATION: &str = "/result/to_generation";
/// History request baseline field.
const SINCE_GENERATION: &str = "since_generation";

/// Original source bytes indexed by fixture path.
type Sources = BTreeMap<&'static str, Vec<u8>>;
/// Real server input and its decoded stdout channel.
type Client<'a> = (&'a mut ChildStdin, &'a Receiver<FrameResult>);

/// Reads a required protocol array.
fn array<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("missing array {pointer}: {value}"))
}

/// Reads a required generation number.
fn number(value: &Value, pointer: &str) -> Result<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("missing generation {pointer}: {value}"))
}

/// Checks both occurrence and file classifications.
fn assert_language(report: &Value, language: &str) -> Result<()> {
    assert_file_languages(report, language)?;
    let clusters = report_clusters(report)?;
    let cluster = clusters.first().context("expected duplicate cluster")?;
    for occurrence in array(cluster, OCCURRENCES)? {
        assert_eq!(at(occurrence, LANGUAGE), language);
    }
    Ok(())
}

/// Checks exact metric paths, count, and classification.
fn assert_file_languages(report: &Value, language: &str) -> Result<()> {
    let metrics = array(report, PER_FILE)?;
    assert_eq!(metrics.len(), OCCURRENCE_COUNT);
    let expected: BTreeSet<_> = FILES.into_iter().map(str::to_owned).collect();
    let paths: BTreeSet<_> = metrics.iter().map(metric_path).collect::<Result<_>>()?;
    assert_eq!(expected, paths);
    for file in metrics {
        assert_eq!(at(file, LANGUAGE), language);
    }
    Ok(())
}

/// Reads the required source path from a per-file metric.
fn metric_path(file: &Value) -> Result<String> {
    at(file, PATH)
        .as_str()
        .map(str::to_owned)
        .context("file path")
}

/// Preserves exact clone identity, bucket, rank, paths, and count.
fn assert_clone(report: &Value, language: &str) -> Result<()> {
    assert_report_shell(report, FILE_COUNT);
    let clusters = report_clusters(report)?;
    assert_eq!(clusters.len(), CLUSTER_COUNT);
    let cluster = clusters.first().context("expected duplicate cluster")?;
    assert_eq!(at(cluster, KIND), IDENTICAL);
    assert_eq!(at(cluster, RANK), FIRST_RANK);
    assert_eq!(at(cluster, COUNT), FILE_COUNT);
    assert_eq!(array(cluster, OCCURRENCES)?.len(), OCCURRENCE_COUNT);
    assert_eq!(
        cluster_paths(cluster),
        FILES.into_iter().map(str::to_owned).collect()
    );
    assert_language(report, language)
}

/// Captures both source files without changing their contents.
fn source_bytes(root: &Path) -> Result<Sources> {
    FILES
        .into_iter()
        .map(|name| Ok((name, fs::read(root.join(name))?)))
        .collect()
}

/// Reads the current published generation without requesting a scan.
fn generation(stdin: &mut ChildStdin, frames: &Receiver<FrameResult>) -> Result<u64> {
    let params = Value::Object(Map::from_iter([(
        SINCE_GENERATION.to_owned(),
        ZERO_GENERATION.into(),
    )]));
    number(
        &request_response(stdin, frames, REPORT_DELTA, &params)?,
        DELTA_GENERATION,
    )
}

/// Awaits a newer real server notification with a bounded deadline.
fn changed_after(frames: &Receiver<FrameResult>, previous: u64) -> Result<u64> {
    let deadline = Instant::now()
        .checked_add(REPORT_TIMEOUT)
        .ok_or_else(|| anyhow!("deadline overflow"))?;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let frame = recv_method(frames, REPORT_CHANGED, remaining)?;
        let next = number(&frame, NOTIFIED_GENERATION)?;
        if next > previous {
            return Ok(next);
        }
    }
}

/// Saves a manifest change and checks the complete reactive result.
fn transition(
    client: &mut Client<'_>,
    root: &Path,
    previous: u64,
    manifest: Option<&[u8]>,
    language: &str,
    sources: &Sources,
) -> Result<u64> {
    write_manifest(root, manifest)?;
    let next = changed_after(client.1, previous)?;
    assert!(next > previous);
    let frame = request_response(client.0, client.1, REPORT_GET, &Value::Object(Map::new()))?;
    assert_clone(at(&frame, RESULT), language)?;
    assert_eq!(source_bytes(root)?, *sources);
    assert!(generation(client.0, client.1)? >= next);
    Ok(next)
}

/// Creates or removes the selection file through the filesystem.
fn write_manifest(root: &Path, manifest: Option<&[u8]>) -> Result<()> {
    match manifest {
        Some(bytes) => fs::write(root.join(MANIFEST), bytes)?,
        None => fs::remove_file(root.join(MANIFEST))?,
    }
    Ok(())
}

/// Verifies selection and restoration preserve both source files.
fn round_trip(
    client: &mut Client<'_>,
    root: &Path,
    manifest: &[u8],
    sources: &Sources,
) -> Result<()> {
    let first = generation(client.0, client.1)?;
    let selected = transition(client, root, first, Some(manifest), OSPREY_PYTHON, sources)?;
    let restored = transition(client, root, selected, None, PYTHON, sources)?;
    assert!(restored > selected);
    assert_eq!(source_bytes(root)?, *sources);
    Ok(())
}

/// [LANG-CAND-OSPREY] Manifest saves alone reclassify unchanged Python sources.
#[test]
fn saved_manifest_reclassifies_unchanged_python_files() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let root = workspace.path();
    let manifest = fs::read(root.join(MANIFEST))?;
    write_manifest(root, None)?;
    let sources = source_bytes(root)?;
    let (_guard, mut stdin, frames, initial) = spawn_lsp_with_initial_report(root)?;
    assert_clone(at(&initial, RESULT), PYTHON)?;
    round_trip(&mut (&mut stdin, &frames), root, &manifest, &sources)?;
    Ok(())
}
