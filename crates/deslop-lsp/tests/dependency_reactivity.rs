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
const CLUSTERS_FIELD: &str = "clusters";
const METRICS_FIELD: &str = "metrics";
const FILES_ANALYSED_FIELD: &str = "files_analysed";
const DUPLICATED_FILES_FIELD: &str = "duplicated_files";

/// The duplicate population once `node_modules/pkg/Beta.cs` is rewritten
/// to unrelated code: of the four seeded copies of the `csharp-small`
/// pair, the three untouched files — `Alpha.cs`, `Beta.cs` and
/// `node_modules/pkg/Alpha.cs` — remain one clone family.
const DUPLICATED_FILES_AFTER_EDIT: u64 = 3;

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
        at(report, FILES_ANALYSED_FIELD).as_u64() == Some(4)
    })?;
    assert_initial_report(&initial);

    let first_party = root.join("Beta.cs");
    let first_party_source = fs::read_to_string(&first_party)?;
    fs::write(&first_party, unrelated_csharp())?;
    let changed = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, CLUSTERS_FIELD) != at(&initial, CLUSTERS_FIELD)
    })?;
    assert_changed_report(&changed, &initial);

    fs::write(&first_party, first_party_source)?;
    let restored = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, CLUSTERS_FIELD) == at(&initial, CLUSTERS_FIELD)
    })?;
    assert_eq!(
        at(&restored, METRICS_FIELD),
        at(&initial, METRICS_FIELD),
        "restore must converge"
    );

    let dependency = root.join("node_modules/pkg/Gamma.cs");
    let _bytes = fs::copy(fixture("csharp-small").join("Alpha.cs"), &dependency)?;
    let dependency_changed =
        wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
            at(report, FILES_ANALYSED_FIELD).as_u64() == Some(5)
        })?;
    assert_eq!(
        at(&dependency_changed, FILES_ANALYSED_FIELD),
        5,
        "new dependency source must be analysed"
    );
    assert_ne!(
        at(&dependency_changed, CLUSTERS_FIELD),
        at(&restored, CLUSTERS_FIELD)
    );
    assert_ne!(
        at(&dependency_changed, METRICS_FIELD),
        at(&restored, METRICS_FIELD)
    );
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
        at(report, FILES_ANALYSED_FIELD).as_u64() == Some(4)
    })?;
    assert_initial_report(&initial);
    assert_eq!(
        json_path(&initial, &[METRICS_FIELD, DUPLICATED_FILES_FIELD]),
        4,
        "{initial:#}"
    );

    fs::write(root.join("node_modules/pkg/Beta.cs"), unrelated_csharp())?;
    let changed = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, FILES_ANALYSED_FIELD).as_u64() == Some(4)
            && at(report, CLUSTERS_FIELD) != at(&initial, CLUSTERS_FIELD)
    })?;

    assert_eq!(
        at(&changed, FILES_ANALYSED_FIELD),
        4,
        "edited dependency stays analysed"
    );
    assert_ne!(
        at(&changed, CLUSTERS_FIELD),
        at(&initial, CLUSTERS_FIELD),
        "cluster wire stayed stale"
    );
    assert_ne!(
        at(&changed, METRICS_FIELD),
        at(&initial, METRICS_FIELD),
        "repo metrics stayed stale"
    );
    assert!(
        dependency_left_duplicate_population(&changed),
        "the rewritten dependency must leave the duplicate population while the three \
         untouched copies stay in it: {changed:#}"
    );
    assert!(
        json_path(&changed, &[METRICS_FIELD, "duplication_percent"])
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
        at(changed, FILES_ANALYSED_FIELD),
        4,
        "edited source stays analysed"
    );
    assert_ne!(
        at(changed, CLUSTERS_FIELD),
        at(previous, CLUSTERS_FIELD),
        "cluster wire must refresh"
    );
    assert_ne!(
        at(changed, METRICS_FIELD),
        at(previous, METRICS_FIELD),
        "metrics must refresh"
    );
}

/// True when `report` proves the rewritten dependency left the duplicate
/// population while the three untouched copies stayed in it
/// ([PRINCIPLES-LIVE-IS-REACTIVE], GH #416). Fail-closed: an absent or
/// non-numeric `metrics.duplicated_files` compares as `Null`, so only the
/// measured population of the surviving copies satisfies the verdict —
/// never a report that stopped rendering the count, and never a blind
/// detector's zero.
fn dependency_left_duplicate_population(report: &serde_json::Value) -> bool {
    json_path(report, &[METRICS_FIELD, DUPLICATED_FILES_FIELD]) == DUPLICATED_FILES_AFTER_EDIT
}

/// GH #416: the duplicate-population verdict must be fail-closed. A report
/// that stops rendering `metrics.duplicated_files`, and a blind detector
/// rendering zero, must both be rejected; only the measured population of
/// the three surviving copies satisfies it.
#[test]
fn duplicate_population_verdict_rejects_absent_and_blind_counts() {
    let absent = serde_json::json!({ METRICS_FIELD: {} });
    assert!(
        !dependency_left_duplicate_population(&absent),
        "a report missing metrics.{DUPLICATED_FILES_FIELD} must never satisfy the verdict"
    );
    let blind = serde_json::json!({ METRICS_FIELD: { DUPLICATED_FILES_FIELD: 0 } });
    assert!(
        !dependency_left_duplicate_population(&blind),
        "a blind detector rendering zero duplicated files must never satisfy the verdict"
    );
    let measured = serde_json::json!({
        METRICS_FIELD: { DUPLICATED_FILES_FIELD: DUPLICATED_FILES_AFTER_EDIT }
    });
    assert!(
        dependency_left_duplicate_population(&measured),
        "the measured population of {DUPLICATED_FILES_AFTER_EDIT} surviving copies must \
         satisfy the verdict"
    );
}

fn unrelated_csharp() -> &'static str {
    "namespace Unique { public class Isolated { public string Describe(string value) { return value.Length.ToString(); } } }\n"
}
