//! Cargo build script: regenerates `src/wire_generated.rs` from
//! `docs/models/live-ipc.td` via `scripts/typediagram-gen.mjs` before
//! rustc runs. Per CLAUDE.md the generated file is gitignored, so this
//! build script is the only path that produces it on a fresh checkout.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    let script = workspace_root.join("scripts/typediagram-gen.mjs");
    let td_source = workspace_root.join("docs/models/live-ipc.td");
    let generated = workspace_root.join("crates/deslop-core/src/wire_generated.rs");

    println!("cargo:rerun-if-changed={}", script.display());
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
