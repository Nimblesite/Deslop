//! High-level app-layer tests for the `deslop-lsp` process contract.
//!
//! These tests intentionally avoid spawning the binary. They exercise
//! the highest tested layer under the process adapter: argument
//! interpretation, version output, startup dispatch, and exit handling.

use std::{path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use deslop_core::embedding::{EmbeddingMode, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL};
use deslop_lsp::app::{action_from_args, run_process, run_process_result, LspAction, LspStartup};
use serde_json::Value;

/// Plain `--version` prints the exact Deployment Toolkit contract and
/// never attempts to start the server.
#[test]
fn plain_version_is_handled_before_server_startup() -> Result<()> {
    let output = version_output(action_from_args(["deslop-lsp", "--version"])?)?;
    assert_eq!(output, "deslop-lsp 0.1.0\n");
    assert!(output.starts_with("deslop-lsp"));
    assert!(output.ends_with('\n'));

    let mut stdout = Vec::new();
    let mut runner_called = false;
    run_process_result(["deslop-lsp", "--version"], &mut stdout, |_| {
        runner_called = true;
        Ok(())
    })?;
    assert!(!runner_called, "version preflight must not run the server");
    assert_eq!(String::from_utf8(stdout)?, "deslop-lsp 0.1.0\n");
    Ok(())
}

/// JSON `--version` exposes every field the deployment manifest verifier
/// expects without going through stdio server startup.
#[test]
fn json_version_is_handled_before_server_startup() -> Result<()> {
    let mut stdout = Vec::new();
    run_process_result(["deslop-lsp", "--version", "--json"], &mut stdout, |_| {
        Err(anyhow!(
            "server runner must not be called for version output"
        ))
    })?;

    let payload: Value = serde_json::from_slice(&stdout)?;
    assert_eq!(payload.get("manifestVersion"), Some(&Value::from(1)));
    assert_eq!(payload.get("name"), Some(&Value::from("deslop-lsp")));
    assert_eq!(payload.get("version"), Some(&Value::from("0.1.0")));
    assert_eq!(payload.get("kind"), Some(&Value::from("lsp")));
    assert_eq!(payload.get("language"), Some(&Value::from("rust")));
    assert_eq!(payload.get("product"), Some(&Value::from("deslop")));
    assert!(payload.get("unknown").is_none());
    Ok(())
}

/// A normal invocation parses every startup flag into one app-layer
/// configuration object for the server runner.
#[test]
fn serve_action_carries_full_startup_configuration() -> Result<()> {
    let startup = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/deslop-workspace",
        "--min-nodes",
        "42",
        "--worker-threads",
        "3",
        "--embeddings",
        "auto",
        "--embedding-provider",
        "stub",
        "--embedding-model",
        "unit-model",
        "--embedding-endpoint",
        "http://127.0.0.1:9999",
        "--stdio",
    ])?)?;

    assert_eq!(
        startup.workspace_root,
        PathBuf::from("/tmp/deslop-workspace")
    );
    assert_eq!(startup.min_nodes, 42);
    assert_eq!(startup.worker_threads, 3);
    assert_eq!(startup.embedding.mode, EmbeddingMode::Auto);
    assert_eq!(startup.embedding.mode.as_str(), "auto");
    assert_eq!(startup.embedding.provider_id, "stub");
    assert_eq!(startup.embedding.model_id, "unit-model");
    assert_eq!(startup.embedding.endpoint, "http://127.0.0.1:9999");
    Ok(())
}

/// Defaults stay centralized in the app layer and are visible to tests
/// without driving the binary through subprocess coverage.
#[test]
fn serve_action_applies_documented_defaults() -> Result<()> {
    let startup = serve_startup(action_from_args(["deslop-lsp", "/tmp/deslop-defaults"])?)?;
    assert_eq!(
        startup.workspace_root,
        PathBuf::from("/tmp/deslop-defaults")
    );
    assert_eq!(startup.min_nodes, 30);
    assert_eq!(startup.worker_threads, 0);
    assert_eq!(startup.embedding.mode, EmbeddingMode::Off);
    assert_eq!(startup.embedding.mode.as_str(), "off");
    assert_eq!(startup.embedding.provider_id, "ollama");
    assert_eq!(startup.embedding.model_id, DEFAULT_OLLAMA_MODEL);
    assert_eq!(startup.embedding.endpoint, DEFAULT_OLLAMA_ENDPOINT);
    Ok(())
}

/// `run_process_result` is the testable seam above stdio: it writes no
/// CLI output for serving and hands the parsed startup to the runner.
#[test]
fn process_result_dispatches_serve_action_to_runner() -> Result<()> {
    let mut stdout = Vec::new();
    let mut observed: Option<LspStartup> = None;
    run_process_result(
        ["deslop-lsp", "/tmp/deslop-runner", "--min-nodes", "7"],
        &mut stdout,
        |startup| {
            observed = Some(startup);
            Ok(())
        },
    )?;

    let startup = observed.ok_or_else(|| anyhow!("server runner was not called"))?;
    assert!(stdout.is_empty(), "serve path must not write CLI stdout");
    assert_eq!(startup.workspace_root, PathBuf::from("/tmp/deslop-runner"));
    assert_eq!(startup.min_nodes, 7);
    assert_eq!(startup.worker_threads, 0);
    assert_eq!(startup.embedding.mode, EmbeddingMode::Off);
    Ok(())
}

/// Process-level exit conversion remains above the tested app behavior
/// and reflects the injected runner result.
#[test]
fn process_exit_code_reflects_runner_result() -> Result<()> {
    let success = run_process(["deslop-lsp", "/tmp/deslop-ok"], Vec::<u8>::new(), |_| {
        Ok(())
    });
    assert_eq!(success, ExitCode::SUCCESS);

    let failure = run_process(["deslop-lsp", "/tmp/deslop-fail"], Vec::<u8>::new(), |_| {
        Err(anyhow!("server exploded"))
    });
    assert_eq!(failure, ExitCode::from(1));
    Ok(())
}

/// Invalid argv is rejected before server startup, with concrete
/// messages users can act on.
#[test]
fn invalid_arguments_return_user_facing_errors() -> Result<()> {
    assert_error_contains(["deslop-lsp"], "usage: deslop-lsp")?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--min-nodes"],
        "--min-nodes requires",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--min-nodes", "abc"],
        "invalid digit",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--worker-threads"],
        "--worker-threads requires",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--embeddings", "banana"],
        "expected one of auto/required/off",
    )?;
    Ok(())
}

/// Extracts version output from a parsed action.
fn version_output(action: LspAction) -> Result<String> {
    match action {
        LspAction::Version { output } => Ok(output),
        other => Err(anyhow!("expected version action, got {other:?}")),
    }
}

/// Extracts startup configuration from a parsed action.
fn serve_startup(action: LspAction) -> Result<LspStartup> {
    match action {
        LspAction::Serve(startup) => Ok(startup),
        other => Err(anyhow!("expected serve action, got {other:?}")),
    }
}

/// Asserts that parsing `args` fails with `expected` in the error chain.
fn assert_error_contains<const N: usize>(args: [&str; N], expected: &str) -> Result<()> {
    let error = action_from_args(args).err().ok_or_else(|| {
        anyhow!("expected argument parsing to fail with text containing {expected:?}")
    })?;
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains(expected),
        "error {rendered:?} did not contain {expected:?}"
    );
    Ok(())
}
