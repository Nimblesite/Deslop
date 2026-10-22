//! [MCP-START-RECOVERY] Structured JSON-first recovery without an engine connection.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[cfg(all(test, unix))]
mod tests;

/// socket -> cache -> .deslop -> pinned workspace.
const WORKSPACE_ANCESTOR: usize = 3;
/// Default CLI output, independent of the editor's warm-start snapshot.
const REPORT_PATH: &str = ".deslop/deslop-report.json";
/// Existing client retry guidance; recovery does not block on this timer.
const RETRY_AFTER_MS: u64 = 500;
/// Canonical startup and snapshot-validation guide.
const GUIDE: &str = "https://deslop.live/docs/for-ai/";

/// A path as JSON text. Serializing a `Path` directly fails on a non-UTF-8
/// name, and `json!` unwraps that failure; the lossy display cannot fail.
fn shown(path: &Path) -> String {
    path.display().to_string()
}

/// The CLI from this MCP's own bundle, never a potentially unrelated PATH binary.
fn bundled_cli() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    executable
        .parent()
        .map(|directory| directory.join(format!("deslop{}", std::env::consts::EXE_SUFFIX)))
}

/// A JSON-only scan uses the normal cache unless repository configuration opts out.
fn cli_fallback(root: Option<&Path>) -> Value {
    json!({
        "command": bundled_cli().as_deref().map(shown),
        "arguments": root.map(|path| json!([shown(path), "--notext", "--nohtml", "--no-color"])),
        "output": root.map(|path| shown(&path.join(REPORT_PATH))),
        "cache_enabled_by_default": true,
        "note": "Use only if no usable JSON snapshot exists or source changed. Respect [analysis] incremental = false; do not pass --no-incremental or delete caches. This scan does not search unwritten snippets.",
    })
}

/// Keeps existing endpoint/cache fields while supplying a complete fallback workflow.
pub(super) fn lsp_recovery_data(socket_path: &Path) -> Value {
    let cache = socket_path.parent();
    let root = socket_path.ancestors().nth(WORKSPACE_ANCESTOR);
    json!({
        "reason": "lsp_not_running", "retry_after_ms": RETRY_AFTER_MS,
        "socket_path": shown(socket_path),
        "port_file_path": cache.map(|dir| shown(&dir.join(deslop_core::live::transport::IPC_PORT_FILE_NAME))),
        "fallback_order": ["report_fallback", "cache_fallback", "cli_fallback"],
        "report_fallback": {"path": root.map(|path| shown(&path.join(REPORT_PATH))), "format": "report-json"},
        "cache_fallback": {
            "path": cache.map(|dir| shown(&dir.join(deslop_core::live::LIVE_REPORT_FILE_NAME))),
            "format": "live-report-json", "note": "Read this JSON snapshot if the CLI report is absent. It can predate incremental edits; verify tool_version, source paths and ranges before editing.",
        },
        "cli_fallback": cli_fallback(root), "guide": GUIDE,
    })
}
