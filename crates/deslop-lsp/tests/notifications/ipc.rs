//! [LSP-IPC] Startup endpoint failures reach the real editor protocol.

use super::*;

const IPC_SOCKET_PATH: &str = ".deslop/cache/deslop.sock";
const IPC_PORT_PATH: &str = ".deslop/cache/deslop.port";
const IPC_ERROR_NOTIFICATION: &str = "window/showMessage";
const IPC_ERROR_TYPE: u64 = 1;
const IPC_ERROR_TIMEOUT: Duration = Duration::from_secs(15);
const IPC_ERROR_CAUSE: &str = "os error";
const IPC_ERROR_SURFACE: &str = "MCP";
const MINIMUM_FIXTURE_CLUSTERS: usize = 1;
const FIXTURE: &str = "csharp-small";
const INITIALIZED: &str = "initialized";
const TYPE_FIELD: &str = "/params/type";
const MESSAGE_FIELD: &str = "/params/message";

/// [LSP-IPC] An unavailable endpoint must be visible to the editor user.
#[test]
fn ipc_startup_failure_pushes_an_editor_error() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    fs::create_dir_all(workspace.path().join(IPC_SOCKET_PATH))?;
    fs::create_dir_all(workspace.path().join(IPC_PORT_PATH))?;
    let (_guard, mut stdin, frames, initial) = spawn_lsp_with_initial_report(workspace.path())?;
    ensure!(cluster_count(&initial) >= MINIMUM_FIXTURE_CLUSTERS);
    write_frame(
        &mut stdin,
        &notification(INITIALIZED, &serde_json::json!({}))?,
    )?;
    let error = recv_method(&frames, IPC_ERROR_NOTIFICATION, IPC_ERROR_TIMEOUT)?;
    assert_ipc_startup_error(&error, workspace.path())
}

/// Preserve the root and OS cause so users can diagnose an unavailable MCP.
fn assert_ipc_startup_error(error: &serde_json::Value, workspace: &Path) -> Result<()> {
    assert_eq!(json_u64(error, TYPE_FIELD)?, IPC_ERROR_TYPE);
    let message = error
        .pointer(MESSAGE_FIELD)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("missing IPC startup error: {error}"))?;
    ensure!(message.contains(IPC_ERROR_SURFACE), "{message}");
    ensure!(message.contains(IPC_ERROR_CAUSE), "{message}");
    ensure!(
        message.contains(
            workspace
                .to_str()
                .ok_or_else(|| anyhow!("workspace UTF-8"))?
        ),
        "{message}"
    );
    Ok(())
}
