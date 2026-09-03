//! [CORPUS-PRECISION] A ranked cluster carries no language of its own,
//! so the boilerplate-rank gate judges its first occurrence in the
//! language of that occurrence's file, resolved the way the engine maps
//! files to parsers.
//!
//! The gate read a `language` field off the cluster until the mass-only
//! wire stopped carrying one, and every corpus run then died with
//! "cluster carries no language" — a gate that cannot judge reports
//! nothing. A second extension table here would be the other way to get
//! it wrong: the report consumer and the engine would disagree about
//! which grammar a path belongs to, and the disagreement would surface
//! as a rule that quietly stopped firing.

use std::{fs, path::PathBuf, time::Duration};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::super::check_boilerplate_not_ranked_first;
use super::{FLAT_DECLARATION, STATELESS_WIDGET};
use crate::corpus::{repo_root, CorpusRun, Failure};

/// Where the scan root for these tests lives: under `target`, the only
/// place a build may leave files.
const GATE_ROOT: &str = "corpus-precision-gate";

/// The Dart file the ranked occurrence lives in.
const WIDGET_PATH: &str = "lib/ledger_tile.dart";

/// A file no registered parser claims.
const NOTES_PATH: &str = "notes/ledger.txt";

/// Bytes of the notes fixture, which stands in for an occurrence span.
const NOTES_SOURCE: &str = "ledger";

/// The check id the boilerplate rule reports under.
const BOILERPLATE_CHECK: &str = "boilerplate_rank";

/// The manifest rule under test: the flat declaration's own supertype,
/// judged over the top five clusters.
fn manifest() -> Value {
    json!({
        "must_not_rank_first": {
            "forbidden_top_supertypes": [STATELESS_WIDGET],
            "top_n": 5,
        }
    })
}

/// A scan root holding the widget and the notes file, written fresh.
fn scan_root() -> Result<PathBuf> {
    let root = repo_root().join("target").join(GATE_ROOT);
    for (path, contents) in [(WIDGET_PATH, FLAT_DECLARATION), (NOTES_PATH, NOTES_SOURCE)] {
        let file = root.join(path);
        let parent = file
            .parent()
            .context("every gate fixture path has a parent directory")?;
        fs::create_dir_all(parent)?;
        fs::write(&file, contents)?;
    }
    Ok(root)
}

/// One ranked cluster whose occurrences span the whole of `path`. It
/// carries what the wire model gives a cluster and no `language`,
/// because the wire model has none.
fn ranked_cluster(path: &str, length: usize) -> Value {
    let occurrence = json!({
        "path": path,
        "start_byte": 0,
        "end_byte": length,
        "hidden": false,
    });
    json!({
        "id": "c0ffee",
        "rank": 0,
        "size": 2,
        "occurrences": [occurrence, occurrence],
    })
}

/// The gate's failures over a report holding just `cluster`.
fn judged(cluster: &Value) -> Result<Vec<Failure>> {
    assert!(
        cluster.get("language").is_none(),
        "the wire model gives a cluster no language; the fixture must not either"
    );
    let run = CorpusRun {
        report: json!({ "clusters": [cluster] }),
        wall: Duration::ZERO,
        peak_rss_mb: 0,
    };
    let mut failures = Vec::new();
    check_boilerplate_not_ranked_first(&manifest(), &scan_root()?, &run, &mut failures)?;
    Ok(failures)
}

#[test]
fn a_dart_occurrence_is_judged_as_dart_without_a_cluster_language() -> Result<()> {
    let failures = judged(&ranked_cluster(WIDGET_PATH, FLAT_DECLARATION.len()))?;
    assert_eq!(
        failures.len(),
        1,
        "the flat declaration names the forbidden supertype, so the rank-0 \
         cluster fails the gate exactly once: {failures:?}"
    );
    let failure = failures.first().context("one failure")?;
    assert_eq!(
        failure.check, BOILERPLATE_CHECK,
        "the failure is reported under the boilerplate check, which is what \
         the corpus summary groups it by"
    );
    assert!(
        failure.detail.contains(STATELESS_WIDGET) && failure.detail.contains(WIDGET_PATH),
        "the failure names the supertype and the occurrence that declares \
         it, so a reader can go to the code: {}",
        failure.detail
    );
    Ok(())
}

#[test]
fn an_occurrence_no_parser_claims_fails_the_gate_rather_than_passing() -> Result<()> {
    let Err(error) = judged(&ranked_cluster(NOTES_PATH, NOTES_SOURCE.len())) else {
        bail!(
            "a file no registered parser claims cannot be judged against a \
             heritage grammar, and passing it would switch the precision gate \
             off silently — exactly the [CORPUS-BASELINE] failure a ratchet \
             then reads as evidence the defect is absent"
        );
    };
    assert!(
        error.to_string().contains(NOTES_PATH),
        "the error names the occurrence's path so the manifest author can \
         see which file could not be judged: {error}"
    );
    Ok(())
}
