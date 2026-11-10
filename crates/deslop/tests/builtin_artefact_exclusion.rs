//! [CONFIG-EXCLUDE-BUILTIN] Build output and tool trees never reach the
//! report, and a checkout that merely sits under one is still analysed.
//!
//! Black-box: seed one genuine clone pair at the scan root and the same
//! pair under every artefact directory. Each directory that leaked would
//! add two files to `files_analysed` and a path with its name to the
//! clusters. `out/` (TypeScript and VS Code extension output),
//! `.vscode-test/` (the VS Code copy the extension test runner downloads)
//! and `coverage/` (generated coverage reports) were missing, so a VS Code
//! extension repository reported its compiled JavaScript as duplicates of
//! its own TypeScript (gh #298).

use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// `min-nodes` for the seeded fixture, matching `config_include_dependencies`.
const MIN_NODES: u32 = 12;
/// The first-party pair, and nothing else, is analysed.
const FIRST_PARTY_FILES: u64 = 2;
/// Every built-in artefact directory: none of it is source the user wrote.
const ARTEFACT_DIRS: &[&str] = &[
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    ".vscode-test",
    "__pycache__",
    ".dart_tool",
];

/// True when any rendered occurrence path has `name` as a path component.
fn has_component(report: &Value, name: &str) -> bool {
    clusters(report).iter().any(|cluster| {
        occurrence_paths(cluster)
            .iter()
            .any(|path| path.split(['/', '\\']).any(|part| part == name))
    })
}

/// Reports on the pair seeded at `<tmp>/<ancestor>/app` and under every
/// artefact directory below it.
fn report_under(ancestor: &str) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join(ancestor).join("app");
    let pair = fixture("ts-type1-identical");
    seed(&pair, &root)?;
    for dir in ARTEFACT_DIRS {
        seed(&pair, &root.join(dir).join("gen"))?;
    }
    run_report(Path::new(&root), MIN_NODES)
}

/// Asserts only the first-party pair was analysed and reported.
fn assert_first_party_only(report: &Value, ancestor: &str) {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(FIRST_PARTY_FILES),
        "under `{ancestor}`: only the two first-party files may be analysed: {report:#}"
    );
    assert!(
        !clusters(report).is_empty(),
        "under `{ancestor}`: the first-party pair is a genuine clone and must be reported: {report:#}"
    );
    for dir in ARTEFACT_DIRS {
        assert!(
            !has_component(report, dir),
            "under `{ancestor}`: `{dir}/` is build output and must not be reported: {report:#}"
        );
    }
}

#[test]
fn every_artefact_directory_is_excluded_and_a_checkout_under_one_is_still_analysed() -> Result<()> {
    // "workspace" is a plain ancestor; the rest put the checkout itself
    // under an artefact name, which must not exclude it ([CONFIG-EXCLUDE-BUILTIN]
    // corpus scope).
    for ancestor in ["workspace", "out", "coverage", ".vscode-test"] {
        assert_first_party_only(&report_under(ancestor)?, ancestor);
    }
    Ok(())
}
