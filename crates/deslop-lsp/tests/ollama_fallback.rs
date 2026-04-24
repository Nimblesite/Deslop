//! End-to-end coverage for issue #35: when the configured Ollama
//! embedding provider is unreachable, `deslop-lsp` must stay alive
//! and serve a clean protocol — VS Code observed a crash-loop
//! because the backend called `std::process::exit(1)` on provider
//! connect failure.
//!
//! Audience: HUMAN. The editor user configures an Ollama endpoint,
//! starts VS Code before Ollama is up, and expects the extension to
//! keep working (AI embeddings are optional per the issue). The
//! positive invariant: after `initialize`, the LSP child process is
//! still running and its stderr does not record a fatal startup
//! failure. We do not wait on protocol round-trips because a crashed
//! LSP closes stdout and the harness would hang forever — liveness
//! is checked directly on the child.

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};

/// Loopback port nothing should listen on. Connection attempts fail
/// with ECONNREFUSED instantly, reproducing the unreachable provider
/// scenario without waiting on DNS.
const UNREACHABLE_ENDPOINT: &str = "http://127.0.0.1:1";

/// How long we give the LSP to run past the point where it would have
/// crashed on the unreachable-provider path. The backend aborts during
/// initialise, well under 1 second; 3 seconds is generous.
const LIVENESS_WINDOW: Duration = Duration::from_secs(3);

/// Audience: HUMAN. Issue #35. The LSP must not exit during startup
/// when the configured Ollama endpoint is unreachable. Positive
/// invariant: the child process is still alive after the liveness
/// window and its stderr has not reported "backend failed to
/// initialise" (the fatal message preceding `process::exit(1)`).
#[test]
fn lsp_survives_when_configured_ollama_endpoint_is_unreachable() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let mut child = spawn_lsp_with_embeddings(
        workspace.path(),
        EmbeddingLaunch {
            mode: "auto",
            provider: "ollama",
            model: "nomic-embed-text",
            endpoint: UNREACHABLE_ENDPOINT,
        },
    )?;
    // Nudge the server into its initialize handler so any fatal
    // startup path runs before the liveness probe fires. Keep the
    // stdin handle alive through the liveness window so closing
    // the pipe doesn't make the LSP exit cleanly on EOF — we're
    // measuring fatal crashes, not clean shutdowns.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("child stdin missing"))?;
    let init = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#;
    let header = format!("Content-Length: {}\r\n\r\n", init.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(init)?;
    stdin.flush()?;

    let deadline = Instant::now() + LIVENESS_WINDOW;
    while Instant::now() < deadline {
        match child.try_wait()? {
            Some(status) => {
                let stderr = drain_stderr(&mut child);
                let _ = child.kill();
                return Err(anyhow!(
                    "deslop-lsp exited during initialise with status {status:?} — \
                     the LSP must stay alive when the configured Ollama endpoint is unreachable. \
                     stderr:\n{stderr}"
                ));
            }
            None => thread::sleep(Duration::from_millis(100)),
        }
    }

    // Keep stdin alive until the test finishes so the kept-open
    // handle is what ends the liveness window, not an EOF-triggered
    // clean shutdown.
    drop(stdin);
    let _ = child.kill();
    Ok(())
}

#[derive(Copy, Clone)]
struct EmbeddingLaunch {
    mode: &'static str,
    provider: &'static str,
    model: &'static str,
    endpoint: &'static str,
}

fn spawn_lsp_with_embeddings(workspace_root: &Path, launch: EmbeddingLaunch) -> Result<Child> {
    let bin = resolve_bin();
    Ok(Command::new(bin)
        .arg(workspace_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--embeddings")
        .arg(launch.mode)
        .arg("--embedding-provider")
        .arg(launch.provider)
        .arg("--embedding-model")
        .arg(launch.model)
        .arg("--embedding-endpoint")
        .arg(launch.endpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?)
}

fn resolve_bin() -> PathBuf {
    assert_cmd::cargo::cargo_bin("deslop-lsp")
}

fn drain_stderr(child: &mut Child) -> String {
    use std::io::Read as _;
    let Some(mut stderr) = child.stderr.take() else {
        return String::new();
    };
    let mut buf = String::new();
    let _ = stderr.read_to_string(&mut buf);
    buf
}
