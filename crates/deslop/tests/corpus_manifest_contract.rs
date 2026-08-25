//! [CORPUS-RECALL] Contract over the corpus manifests themselves.
//!
//! The curated Type-2 recall gate reads `must_find_type2` out of
//! `corpus/*.json`. Every one of those lists was empty at one point, so the
//! gate iterated nothing, reported no failure, and read as green — a check
//! that asserts nothing is indistinguishable from a check that passes. These
//! tests make that state fail loudly instead.
//!
//! [TEST-SELECTION-SKIP] They read JSON only, so unlike the `corpus_repos`
//! gate they need no clone on disk and carry no `#[ignore]` — they run in
//! `make test`. A contract that only ran in `make test-corpus` would have been
//! absent from exactly the pipeline that let the lists go empty, and a name
//! filter would have taken them anyway: `--skip corpus_` matched this file's
//! whole target by its name alone (gh #412).

use std::{fs, path::Path};

use anyhow::{Context, Result};
use deslop_test_support::corpus::repo_root;
use serde_json::Value;

/// The corpus manifests, each with its file stem. `known-failures.json` is
/// the check registry rather than a repository, so it is not one of these.
fn manifests() -> Result<Vec<(String, Value)>> {
    let directory = repo_root().join("corpus");
    let mut found = Vec::new();
    for entry in fs::read_dir(&directory).context("corpus/ must be readable")? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
            && stem(&path) != "known-failures"
        {
            let manifest = read_manifest(&path)?;
            found.push((stem(&path), manifest));
        }
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    assert!(
        found.len() >= 9,
        "the corpus lost manifests: only {:?} remain",
        found.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    Ok(found)
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn read_manifest(path: &Path) -> Result<Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("unreadable: {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("not JSON: {}", path.display()))
}

/// The curated entries of one manifest, empty when the key is absent.
fn curated(manifest: &Value) -> &[Value] {
    manifest
        .get("must_find_type2")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// A string field, empty when absent — every assertion here treats a missing
/// field and a blank one as the same failure.
fn text_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn curated_type2_ground_truth_exists_and_is_not_confined_to_one_language() -> Result<()> {
    let manifests = manifests()?;
    let languages: Vec<&str> = manifests
        .iter()
        .filter(|(_, manifest)| !curated(manifest).is_empty())
        .filter_map(|(_, manifest)| manifest.get("language").and_then(Value::as_str))
        .collect();

    assert!(
        !languages.is_empty(),
        "no manifest curates `must_find_type2`, so the curated Type-2 recall gate iterates \
         nothing and passes while asserting nothing. Hand-verify a rename pair and list it."
    );
    assert!(
        languages.contains(&"rust"),
        "curated Type-2 ground truth covers {languages:?} but not rust; a Rust-only recall \
         regression would pass this gate"
    );
    assert!(
        languages.contains(&"typescript"),
        "curated Type-2 ground truth covers {languages:?} but not typescript; a \
         TypeScript-only recall regression would pass this gate"
    );
    Ok(())
}

#[test]
fn every_curated_type2_entry_names_a_verifiable_cross_file_pair() -> Result<()> {
    for (name, manifest) in manifests()? {
        for entry in curated(&manifest) {
            let files: Vec<&str> = entry
                .get("files")
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .filter_map(Value::as_str)
                .collect();
            assert_ground_truth_pair(&name, &files, entry);
        }
    }
    Ok(())
}

/// One curated entry must name at least two distinct files and carry the
/// human evidence. An entry naming no files satisfies the span predicate
/// vacuously, and one with no `verified` note cannot be re-checked by the
/// next person, so both are silent holes rather than ground truth.
fn assert_ground_truth_pair(name: &str, files: &[&str], entry: &Value) {
    assert!(
        files.len() >= 2,
        "{name}: a curated Type-2 entry names {files:?} — a rename pair spans two files"
    );
    let mut distinct = files.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        files.len(),
        "{name}: a curated Type-2 entry repeats a file in {files:?}, so a single occurrence \
         could satisfy it"
    );
    assert!(
        text_field(entry, "why").len() > 40,
        "{name}: curated entry for {files:?} must say what the duplicated code is"
    );
    assert!(
        text_field(entry, "verified").len() > 40,
        "{name}: curated entry for {files:?} must record how a human verified the rename, so \
         the claim can be re-checked rather than trusted"
    );
}

#[test]
fn a_manifest_status_never_contradicts_its_curated_list() -> Result<()> {
    for (name, manifest) in manifests()? {
        let status = text_field(&manifest, "must_find_type2_status");
        let uncurated = status.contains("NOT YET CURATED");
        if curated(&manifest).is_empty() {
            assert!(
                status.is_empty() || uncurated,
                "{name}: curates no Type-2 pair but its status claims otherwise: {status}"
            );
        } else {
            assert!(
                !status.is_empty() && !uncurated,
                "{name}: curates Type-2 pairs but its status is missing or still says the \
                 repository is uncurated: {status}"
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "[SKIP-UNFINISHED] GH #426 [CORPUS-SCOPE] \
            docs/plans/corpus-assertion.md — seven of the nine manifests carry measured \
            bounds; flutter and fsharp do not, because curating a bound means measuring it \
            and those two are exactly the repositories the gate refuses to scan: both are \
            recorded in known-failures.json under `memory` for exceeding their per-repo \
            ceilings (#166). A local pass reached 7.5 GB on fsharp and was still climbing. \
            The assertions here are unchanged - no bound was relaxed and no manifest was \
            given a placeholder. Run it with `cargo test -p deslop --test suite -- --ignored \
            corpus_manifest_contract::`."]
fn every_manifest_curates_a_non_vacuous_scan_scope() -> Result<()> {
    for (name, manifest) in manifests()? {
        assert_positive_file_floor(&name, &manifest);
        assert_valid_cluster_band(&name, &manifest);
    }
    Ok(())
}

/// Requires a positive lower bound for the number of files reached by a scan.
fn assert_positive_file_floor(name: &str, manifest: &Value) {
    let minimum = manifest.get("expect_files_min").and_then(Value::as_u64);
    assert!(
        minimum.is_some_and(|value| value > 0),
        "{name}: `expect_files_min` must be a positive curated floor; without it, a scan that \
         analysed zero files can pass every cluster assertion"
    );
}

/// Requires a positive, ordered inclusive band for the report's cluster count.
fn assert_valid_cluster_band(name: &str, manifest: &Value) {
    let band = manifest.get("expect_clusters");
    let minimum = band
        .and_then(|value| value.get("min"))
        .and_then(Value::as_u64);
    let maximum = band
        .and_then(|value| value.get("max"))
        .and_then(Value::as_u64);
    assert!(
        matches!((minimum, maximum), (Some(min), Some(max)) if min > 0 && min <= max),
        "{name}: `expect_clusters` must have a positive `min` no greater than `max`; without \
         a curated band, a repository-wide detection collapse or explosion passes silently"
    );
}
