//! LSP E2E regression for the live half of
//! [CONFIG-EXCLUDE-DEPENDENCIES]. The real filesystem watcher must honour
//! the same opt-in as the cold pipeline scan.

mod common;

use std::{fs, path::Path, time::Duration};

use anyhow::Result;
use common::{fixture, handshake, spawn_lsp_guarded, wait_for_report_matching};

const REPORT_TIMEOUT: Duration = Duration::from_secs(20);
const FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];

/// [PRINCIPLES-LIVE-IS-REACTIVE] `include_dependencies = true` governs the
/// whole watcher → scheduler → report loop. A first-party edit proves the OS
/// watcher is running before a new dependency source file is exercised.
#[test]
fn opted_in_dependency_creation_refreshes_the_live_lsp_report() -> Result<()> {
    // Keep the watcher root and notify event paths in the same canonical
    // namespace. macOS aliases `/var` to `/private/var`, which would make a
    // default tempdir test path canonicalisation instead of this rule.
    let canonical_temp = fs::canonicalize(std::env::temp_dir())?;
    let workspace = tempfile::tempdir_in(canonical_temp)?;
    let root = workspace.path().join("node_modules/workspace");
    seed_workspace(&root)?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(&root)?;
    let _initialize = handshake(&mut stdin, &mut stdout)?;
    let initial = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        report["files_analysed"].as_u64() == Some(4)
    })?;
    assert_initial_report(&initial);

    let first_party = root.join("Beta.cs");
    let first_party_source = fs::read_to_string(&first_party)?;
    fs::write(&first_party, unrelated_csharp())?;
    let changed = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        report["clusters"] != initial["clusters"]
    })?;
    assert_changed_report(&changed, &initial);

    fs::write(&first_party, first_party_source)?;
    let restored = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        report["clusters"] == initial["clusters"]
    })?;
    assert_eq!(
        restored["metrics"], initial["metrics"],
        "restore must converge"
    );

    let dependency = root.join("node_modules/pkg/Gamma.cs");
    let _bytes = fs::copy(fixture("csharp-small").join("Alpha.cs"), &dependency)?;
    let dependency_changed =
        wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
            report["files_analysed"].as_u64() == Some(5)
        })?;
    assert_eq!(
        dependency_changed["files_analysed"], 5,
        "new dependency source must be analysed"
    );
    assert_ne!(dependency_changed["clusters"], restored["clusters"]);
    assert_ne!(dependency_changed["metrics"], restored["metrics"]);
    Ok(())
}

fn seed_workspace(root: &Path) -> Result<()> {
    copy_pair(root)?;
    copy_pair(&root.join("node_modules/pkg"))?;
    fs::write(
        root.join(".deslop.toml"),
        "[analysis]\ninclude_dependencies = true\n",
    )?;
    Ok(())
}

fn copy_pair(destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    let source = fixture("csharp-small");
    for file in FILES {
        let _bytes = fs::copy(source.join(file), destination.join(file))?;
    }
    Ok(())
}

fn assert_initial_report(report: &serde_json::Value) {
    assert_eq!(report["files_analysed"], 4, "{report:#}");
    assert!(report["clusters"]
        .as_array()
        .is_some_and(|value| !value.is_empty()));
    assert!(
        report["metrics"]["analysed_loc"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(
        report["metrics"]["duplicated_loc"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(
        report["metrics"]["duplication_percent"]
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );
}

fn assert_changed_report(changed: &serde_json::Value, previous: &serde_json::Value) {
    assert_eq!(changed["files_analysed"], 4, "edited source stays analysed");
    assert_ne!(
        changed["clusters"], previous["clusters"],
        "cluster wire must refresh"
    );
    assert_ne!(
        changed["metrics"], previous["metrics"],
        "metrics must refresh"
    );
}

fn unrelated_csharp() -> &'static str {
    "namespace Unique { public class Isolated { public string Describe(string value) { return value.Length.ToString(); } } }\n"
}
