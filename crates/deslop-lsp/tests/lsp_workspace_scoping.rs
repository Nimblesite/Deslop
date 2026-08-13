//! [CONFIG-EXCLUDE-BUILTIN] / [CONFIG-EXCLUDE-DEPENDENCIES]: what the
//! live LSP admits into a workspace. Drives the real `deslop-lsp` binary
//! over stdio; no pipeline internals are called.

mod common;

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::Result;
use common::{
    at, handshake, path as json_path,
    reports::{
        assert_all_occurrences_visible, assert_report_shell, copy_fixture_files,
        dependency_workspace, has_fragment, has_suffix, occurrence_paths, wait_for_report,
    },
    spawn_lsp_guarded,
};
use serde_json::Value;

const CSHARP_FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];

/// [CONFIG-EXCLUDE-BUILTIN] / [CONFIG-EXCLUDE-DEPENDENCIES]: the live LSP
/// must scope built-ins to the selected workspace, exclude dependencies by
/// default, honour the explicit opt-in, and never admit build output.
#[test]
fn lsp_scopes_builtin_exclusions_and_dependency_opt_in_to_workspace() -> Result<()> {
    let default_report = dependency_report(false)?;
    assert_report_shell(&default_report, 2);
    assert_default_dependency_paths(&default_report)?;

    let included_report = dependency_report(true)?;
    assert_report_shell(&included_report, 4);
    assert_included_dependency_paths(&included_report)?;
    assert!(
        json_path(&included_report, &["metrics", "analysed_loc"])
            .as_u64()
            .unwrap_or_default()
            > json_path(&default_report, &["metrics", "analysed_loc"])
                .as_u64()
                .unwrap_or_default(),
        "opting dependencies in must increase analysed LOC: {included_report:#}"
    );
    Ok(())
}

fn dependency_report(include_dependencies: bool) -> Result<Value> {
    let (_workspace, root) = dependency_workspace()?;
    seed_dependency_workspace(&root, include_dependencies)?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(&root)?;
    let _initialize = handshake(&mut stdin, &mut stdout)?;
    let expected = if include_dependencies { 4 } else { 2 };
    wait_for_report(&mut stdin, &mut stdout, |report| {
        at(report, "files_analysed").as_u64() == Some(expected)
    })
}

fn seed_dependency_workspace(root: &Path, include_dependencies: bool) -> Result<()> {
    copy_fixture_files("csharp-small", &CSHARP_FILES, root)?;
    copy_fixture_files(
        "csharp-small",
        &CSHARP_FILES,
        &root.join("node_modules/pkg"),
    )?;
    copy_fixture_files("csharp-small", &CSHARP_FILES, &root.join("target/gen"))?;
    if include_dependencies {
        fs::write(
            root.join(".deslop.toml"),
            "[analysis]\ninclude_dependencies = true\n",
        )?;
    }
    Ok(())
}

/// The two first-party files and the build-output exclusion hold under
/// every dependency setting — only whether `node_modules` is *also*
/// scanned changes. Asserted once so a scoping regression cannot be
/// masked by one of the two callers drifting.
fn assert_first_party_scope(paths: &BTreeSet<String>) {
    assert!(
        has_suffix(paths, "Alpha.cs"),
        "first-party Alpha missing: {paths:?}"
    );
    assert!(
        has_suffix(paths, "Beta.cs"),
        "first-party Beta missing: {paths:?}"
    );
    assert!(
        !has_fragment(paths, "target/gen"),
        "build output leaked: {paths:?}"
    );
}

fn assert_default_dependency_paths(report: &Value) -> Result<()> {
    let paths = occurrence_paths(report)?;
    assert_first_party_scope(&paths);
    assert!(
        !has_fragment(&paths, "node_modules/pkg"),
        "dependency leaked: {paths:?}"
    );
    assert_all_occurrences_visible(report)?;
    Ok(())
}

fn assert_included_dependency_paths(report: &Value) -> Result<()> {
    let paths = occurrence_paths(report)?;
    assert_first_party_scope(&paths);
    assert!(
        has_fragment(&paths, "node_modules/pkg/Alpha.cs"),
        "dependency Alpha missing: {paths:?}"
    );
    assert!(
        has_fragment(&paths, "node_modules/pkg/Beta.cs"),
        "dependency Beta missing: {paths:?}"
    );
    assert_all_occurrences_visible(report)?;
    Ok(())
}
