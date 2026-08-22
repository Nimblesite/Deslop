//! `[analysis] include_dependencies` — opting third-party library source
//! into the analysis ([CONFIG-EXCLUDE-DEPENDENCIES]).
//!
//! Built-in exclusion covers two different things and the user's interest
//! in them differs. `node_modules`, `vendor`, `.cargo`, `.pub-cache` and
//! `.venv` hold real, readable **library source** the user did not write;
//! auditing a dependency for duplication, or asking whether first-party
//! code re-implements a library it already depends on, are legitimate
//! requests. `target`, `dist`, `build`, `__pycache__` and `.dart_tool` hold
//! compiler and codegen **output**, and `.git`/`.claude` hold whole extra
//! checkouts of the same repository (#222) — none of that is a library the
//! code depends on, so no setting opts back into it.
//!
//! Default stays `false`: ranking is worst-offenders-first ([RANK-SCORE]),
//! so dependency duplication the user cannot act on would otherwise
//! outrank every first-party finding.
//!
//! Black-box: seed one fixture pair at the scan root and the *same* pair
//! under `node_modules/pkg` and `target/gen`, so a leak in either direction
//! changes `files_analysed` by exactly two.

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// `min-nodes` for the seeded fixture, matching
/// `typescript_byte_identical_pair_is_identical_bucket`.
const MIN_NODES: u32 = 12;

/// Enables the opt-in through the same `.deslop.toml` path users write.
const INCLUDE_DEPENDENCIES: &str = "[analysis]\ninclude_dependencies = true\n";

/// Seeds first-party source at `root`, a library copy under
/// `node_modules/pkg`, and build output under `target/gen`, optionally
/// writing `body` as the scan root's `.deslop.toml`.
fn seed_corpus(root: &Path, config: Option<&str>) -> Result<()> {
    let pair = fixture("ts-type1-identical");
    seed(&pair, root)?;
    seed(&pair, &root.join("node_modules").join("pkg"))?;
    seed(&pair, &root.join("target").join("gen"))?;
    if let Some(body) = config {
        fs::write(root.join(".deslop.toml"), body)?;
    }
    Ok(())
}

/// True when any rendered occurrence path has `name` as a path component.
/// Split on both separators so this holds on Windows runners too.
fn has_component(report: &Value, name: &str) -> bool {
    clusters(report).iter().any(|cluster| {
        occurrence_paths(cluster)
            .iter()
            .any(|path| path.split(['/', '\\']).any(|part| part == name))
    })
}

/// Reports on a corpus seeded at `<tmp>/<ancestors>/app/src`.
fn report_for(ancestors: &[&str], config: Option<&str>) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let root = ancestors
        .iter()
        .fold(tmp.path().to_path_buf(), |path, name| path.join(name))
        .join("app")
        .join("src");
    seed_corpus(&root, config)?;
    run_report(&root, MIN_NODES)
}

#[test]
fn dependency_source_is_excluded_by_default() -> Result<()> {
    let report = report_for(&[], None)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "only the two first-party files may be analysed by default: {report:#}"
    );
    assert!(
        !clusters(&report).is_empty(),
        "the first-party pair is a genuine clone and must still be reported — an \
         empty report would satisfy the guards below without proving anything: {report:#}"
    );
    assert!(
        !has_component(&report, "node_modules"),
        "library source must not be analysed by default: {report:#}"
    );
    assert!(
        !has_component(&report, "target"),
        "build output must not be analysed by default: {report:#}"
    );
    Ok(())
}

#[test]
fn include_dependencies_admits_library_source() -> Result<()> {
    let report = report_for(&[], Some(INCLUDE_DEPENDENCIES))?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(4),
        "the two first-party files plus the two under node_modules/pkg must be \
         analysed once the user opts in: {report:#}"
    );
    assert!(
        has_component(&report, "node_modules"),
        "opting in must surface the library source in the report: {report:#}"
    );
    Ok(())
}

#[test]
fn include_dependencies_never_admits_build_output() -> Result<()> {
    // The load-bearing separation: "analyse the libraries I depend on" is
    // not "analyse my compiler output". `target/gen` holds the same
    // fixture pair, so admitting it would push files_analysed to 6.
    let report = report_for(&[], Some(INCLUDE_DEPENDENCIES))?;
    assert!(
        !has_component(&report, "target"),
        "build output must stay excluded even with include_dependencies set: {report:#}"
    );
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(4),
        "include_dependencies must admit exactly the library source, not \
         build output: {report:#}"
    );
    Ok(())
}

#[test]
fn the_setting_is_independent_of_the_scan_root_ancestry() -> Result<()> {
    // #342 and this setting are orthogonal: a checkout that merely lives
    // under a directory named `vendor` or `node_modules` must behave
    // exactly like one that does not, under either setting.
    for ancestor in ["vendor", "node_modules", "target"] {
        let excluded = report_for(&[ancestor], None)?;
        assert_eq!(
            field(&excluded, "files_analysed").as_u64(),
            Some(2),
            "{ancestor}: ancestry must not change the default corpus: {excluded:#}"
        );
        assert!(
            !clusters(&excluded).is_empty(),
            "{ancestor}: the first-party clone must still be reported: {excluded:#}"
        );
        let included = report_for(&[ancestor], Some(INCLUDE_DEPENDENCIES))?;
        assert_eq!(
            field(&included, "files_analysed").as_u64(),
            Some(4),
            "{ancestor}: ancestry must not change the opted-in corpus: {included:#}"
        );
    }
    Ok(())
}
