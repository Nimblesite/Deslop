//! Cargo build script: regenerates `src/wire_generated.rs` from
//! `docs/models/live-ipc.td` via `scripts/typediagram/generate.mjs` before
//! rustc runs. Per CLAUDE.md the generated file is gitignored, so this
//! build script is the only path that produces it on a fresh checkout.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    let script = workspace_root.join("scripts/typediagram/generate.mjs");
    let td_source = workspace_root.join("docs/models/live-ipc.td");
    let generated = workspace_root.join("crates/deslop-core/src/wire_generated.rs");

    // Every file the generator reads, not just its entry point: the
    // type configuration (derives, serde attributes, field docs) lives
    // in sibling modules that `generate.mjs` imports, and naming only
    // the entry point meant an edit to one of them left the previously
    // generated module in place. The symptom is a build that succeeds
    // against code the source of truth no longer describes.
    for script_file in generator_inputs(&script) {
        println!("cargo:rerun-if-changed={}", script_file.display());
    }
    println!("cargo:rerun-if-changed={}", td_source.display());

    let status = Command::new("node")
        .arg(&script)
        .current_dir(&workspace_root)
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!(
            "typediagram-gen failed: node exited with status {status}. \
             Run `make typediagram-gen` to reproduce."
        )),
        Err(error) => Err(format!(
            "typediagram-gen failed to spawn node: {error}. \
             Install Node.js (>=20) so the build can regenerate {}.",
            generated.display()
        )),
    }
}

/// Every file in the generator's directory, plus the entry point
/// itself. Falls back to the entry point alone when the directory
/// cannot be listed, so a build never fails over a missing watch.
fn generator_inputs(script: &Path) -> Vec<PathBuf> {
    let mut inputs = vec![script.to_path_buf()];
    if let Some(dir) = script.parent() {
        if let Ok(entries) = fs::read_dir(dir) {
            let mut listed: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && *path != script)
                .collect();
            listed.sort();
            inputs.extend(listed);
        }
    }
    inputs
}

/// Returns the workspace root from Cargo's crate-local manifest directory.
fn workspace_root() -> Result<PathBuf, String> {
    // build.rs runs with CWD = the crate dir. The workspace root is two
    // parents up: crates/deslop-core/build.rs -> crates -> repo root.
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| "CARGO_MANIFEST_DIR must be set by cargo".to_owned())?;
    let root = Path::new(&manifest_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "crate must have a workspace parent".to_owned())?;
    Ok(root.to_path_buf())
}
