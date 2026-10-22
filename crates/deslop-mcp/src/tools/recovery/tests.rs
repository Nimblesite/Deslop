//! [MCP-START-RECOVERY] Recovery guidance survives any workspace path.

use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

use super::lsp_recovery_data;

/// A workspace directory name that is not valid UTF-8, as Linux allows.
const NON_UTF8_SOCKET: &[u8] = b"/srv/w\xffrk/.deslop/cache/deslop.sock";
/// The same path as a lossy display renders it.
const LOSSY_SOCKET: &str = "/srv/w\u{fffd}rk/.deslop/cache/deslop.sock";
const LOSSY_PORT_FILE: &str = "/srv/w\u{fffd}rk/.deslop/cache/deslop.port";
const LOSSY_REPORT: &str = "/srv/w\u{fffd}rk/.deslop/deslop-report.json";
const LOSSY_ROOT: &str = "/srv/w\u{fffd}rk";
const REASON: &str = "lsp_not_running";

/// An unavailable engine in a non-UTF-8 workspace must still hand the agent
/// its recovery paths rather than crash the MCP while building the error.
#[test]
fn recovery_for_a_non_utf8_workspace_reports_every_path_lossily() {
    let socket = Path::new(OsStr::from_bytes(NON_UTF8_SOCKET));
    let data = lsp_recovery_data(socket);
    assert_eq!(
        data.pointer("/reason").and_then(|v| v.as_str()),
        Some(REASON)
    );
    assert_eq!(
        data.pointer("/socket_path").and_then(|v| v.as_str()),
        Some(LOSSY_SOCKET)
    );
    assert_eq!(
        data.pointer("/port_file_path").and_then(|v| v.as_str()),
        Some(LOSSY_PORT_FILE)
    );
    assert_eq!(
        data.pointer("/report_fallback/path")
            .and_then(|v| v.as_str()),
        Some(LOSSY_REPORT)
    );
    assert_eq!(
        data.pointer("/cli_fallback/output")
            .and_then(|v| v.as_str()),
        Some(LOSSY_REPORT)
    );
    assert_eq!(
        data.pointer("/cli_fallback/arguments/0")
            .and_then(|v| v.as_str()),
        Some(LOSSY_ROOT)
    );
}
