//! LSP E2E regression for the live half of
//! [CONFIG-EXCLUDE-DEPENDENCIES]. The real filesystem watcher must honour
//! the same opt-in as the cold pipeline scan.

use crate::common;

use std::{fs, path::Path, time::Duration};

use anyhow::Result;
use common::{
    at, fixture, handshake, path as json_path,
    reports::{assert_initialize_contract, assert_report_shell, dependency_workspace},
    spawn_lsp_guarded, wait_for_report_matching,
};

const REPORT_TIMEOUT: Duration = Duration::from_secs(20);
const FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];

/// [PRINCIPLES-LIVE-IS-REACTIVE] `include_dependencies = true` governs the
/// whole watcher → scheduler → report loop. A first-party edit proves the OS
/// watcher is running before a new dependency source file is exercised.
#[test]
fn opted_in_dependency_creation_refreshes_the_live_lsp_report() -> Result<()> {
    let (_workspace, root) = dependency_workspace()?;
    seed_workspace(&root)?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(&root)?;
    let _initialize = handshake(&mut stdin, &mut stdout)?;
    let initial = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(4)
    })?;
    assert_initial_report(&initial);

    let first_party = root.join("Beta.cs");
    let first_party_source = fs::read_to_string(&first_party)?;
    fs::write(&first_party, unrelated_csharp())?;
    let changed = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "clusters") != at(&initial, "clusters")
    })?;
    assert_changed_report(&changed, &initial);

    fs::write(&first_party, first_party_source)?;
    let restored = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "clusters") == at(&initial, "clusters")
    })?;
    assert_eq!(
        at(&restored, "metrics"),
        at(&initial, "metrics"),
        "restore must converge"
    );

    let dependency = root.join("node_modules/pkg/Gamma.cs");
    let _bytes = fs::copy(fixture("csharp-small").join("Alpha.cs"), &dependency)?;
    let dependency_changed =
        wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
            at(report, "files_analysed").as_u64() == Some(5)
        })?;
    assert_eq!(
        at(&dependency_changed, "files_analysed"),
        5,
        "new dependency source must be analysed"
    );
    assert_ne!(
        at(&dependency_changed, "clusters"),
        at(&restored, "clusters")
    );
    assert_ne!(at(&dependency_changed, "metrics"), at(&restored, "metrics"));
    Ok(())
}

/// [PRINCIPLES-LIVE-IS-REACTIVE] Opted-in dependency files are first-class
/// live corpus members after the cold scan: editing one must drive the same
/// watcher → scheduler → report transition as editing a first-party file.
#[test]
fn opted_in_dependency_edit_refreshes_clusters_metrics_and_occurrences() -> Result<()> {
    let (_workspace, root) = dependency_workspace()?;
    seed_workspace(&root)?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(&root)?;
    assert_initialize_contract(&handshake(&mut stdin, &mut stdout)?);

    let initial = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(4)
    })?;
    assert_initial_report(&initial);
    assert_eq!(
        json_path(&initial, &["metrics", "duplicated_files"]),
        4,
        "{initial:#}"
    );

    fs::write(root.join("node_modules/pkg/Beta.cs"), unrelated_csharp())?;
    let changed = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(4)
            && at(report, "clusters") != at(&initial, "clusters")
    })?;

    assert_eq!(
        at(&changed, "files_analysed"),
        4,
        "edited dependency stays analysed"
    );
    assert_ne!(
        at(&changed, "clusters"),
        at(&initial, "clusters"),
        "cluster wire stayed stale"
    );
    assert_ne!(
        at(&changed, "metrics"),
        at(&initial, "metrics"),
        "repo metrics stayed stale"
    );
    assert!(
        json_path(&changed, &["metrics", "duplicated_files"])
            .as_u64()
            .unwrap_or_default()
            < 4,
        "the unrelated dependency must leave the duplicate population: {changed:#}"
    );
    assert!(
        json_path(&changed, &["metrics", "duplication_percent"])
            .as_f64()
            .unwrap_or(100.0)
            < 100.0,
        "the report must stop claiming the edited corpus is 100% duplicated: {changed:#}"
    );
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

/// The cold-scan report: the full published shell over the four
/// first-party files. Uses the shared shell assertion rather than a
/// local subset of it, so this suite also pins the wire contract —
/// `min_nodes`, `tool_version`, the slim `schema_doc`, and the hint
/// arrays — that a local copy silently stopped checking.
fn assert_initial_report(report: &serde_json::Value) {
    assert_report_shell(report, 4);
}

fn assert_changed_report(changed: &serde_json::Value, previous: &serde_json::Value) {
    assert_eq!(
        at(changed, "files_analysed"),
        4,
        "edited source stays analysed"
    );
    assert_ne!(
        at(changed, "clusters"),
        at(previous, "clusters"),
        "cluster wire must refresh"
    );
    assert_ne!(
        at(changed, "metrics"),
        at(previous, "metrics"),
        "metrics must refresh"
    );
}

fn unrelated_csharp() -> &'static str {
    "namespace Unique { public class Isolated { public string Describe(string value) { return value.Length.ToString(); } } }\n"
}
