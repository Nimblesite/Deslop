//! High-level app-layer tests for the `deslop-lsp` process contract.
//!
//! These tests intentionally avoid spawning the binary. They exercise
//! the highest tested layer under the process adapter: argument
//! interpretation, version output, startup dispatch, and exit handling.

use std::{path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use deslop_core::embedding::{EmbeddingMode, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL};
use deslop_core::{config::ClonePolicy, live::transport::IpcMode};
use deslop_lsp::app::{
    action_from_args, run_process, run_process_result, run_startup_with, LspAction, LspStartup,
};
use deslop_lsp::backend::LspEmbeddingConfig;
use serde_json::Value;

/// Plain `--version` prints the exact Deployment Toolkit contract and
/// never attempts to start the server.
#[test]
fn plain_version_is_handled_before_server_startup() -> Result<()> {
    let output = version_output(action_from_args(["deslop-lsp", "--version"])?)?;
    assert_eq!(output, expected_plain_version());
    assert!(output.starts_with("deslop-lsp"));
    assert!(output.ends_with('\n'));

    let mut stdout = Vec::new();
    let mut runner_called = false;
    run_process_result(["deslop-lsp", "--version"], &mut stdout, |_| {
        runner_called = true;
        Ok(())
    })?;
    assert!(!runner_called, "version preflight must not run the server");
    assert_eq!(String::from_utf8(stdout)?, expected_plain_version());
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
    assert_eq!(
        payload.get("version").and_then(Value::as_str),
        Some(expected_version())
    );
    assert_eq!(payload.get("kind"), Some(&Value::from("lsp")));
    assert_eq!(payload.get("language"), Some(&Value::from("rust")));
    assert_eq!(payload.get("product"), Some(&Value::from("deslop")));
    assert!(payload.get("unknown").is_none());
    Ok(())
}

fn expected_plain_version() -> String {
    format!("deslop-lsp {}\n", expected_version())
}

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// A normal invocation parses the supported startup flags into one
/// app-layer configuration object for the server runner.
#[test]
fn serve_action_carries_supported_startup_configuration() -> Result<()> {
    let startup = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/deslop-workspace",
        "--worker-threads",
        "3",
        "--nice",
        "5",
        "--stdio",
    ])?)?;

    assert_eq!(
        startup.workspace_root,
        PathBuf::from("/tmp/deslop-workspace")
    );
    assert_eq!(startup.min_nodes, 30);
    assert_eq!(startup.worker_threads, 3);
    assert_eq!(startup.nice, 5);
    assert_eq!(startup.embedding.mode, EmbeddingMode::Off);
    assert_eq!(startup.embedding.mode.as_str(), "off");
    assert_eq!(startup.embedding.provider_id, "ollama");
    assert_eq!(startup.embedding.model_id, DEFAULT_OLLAMA_MODEL);
    assert_eq!(startup.embedding.endpoint, DEFAULT_OLLAMA_ENDPOINT);
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
    assert_eq!(startup.nice, 0);
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
        ["deslop-lsp", "/tmp/deslop-runner", "--worker-threads", "2"],
        &mut stdout,
        |startup| {
            observed = Some(startup);
            Ok(())
        },
    )?;

    let startup = observed.ok_or_else(|| anyhow!("server runner was not called"))?;
    assert!(stdout.is_empty(), "serve path must not write CLI stdout");
    assert_eq!(startup.workspace_root, PathBuf::from("/tmp/deslop-runner"));
    assert_eq!(startup.min_nodes, 30);
    assert_eq!(startup.worker_threads, 2);
    assert_eq!(startup.nice, 0);
    assert_eq!(startup.embedding.mode, EmbeddingMode::Off);
    Ok(())
}

/// Process-level exit conversion remains above the tested app behavior
/// and reflects the injected runner result.
#[test]
fn process_exit_code_reflects_runner_result() {
    let success = run_process(["deslop-lsp", "/tmp/deslop-ok"], Vec::<u8>::new(), |_| {
        Ok(())
    });
    assert_eq!(success, ExitCode::SUCCESS);

    let failure = run_process(["deslop-lsp", "/tmp/deslop-fail"], Vec::<u8>::new(), |_| {
        Err(anyhow!("server exploded"))
    });
    assert_eq!(failure, ExitCode::from(1));
}

/// Startup dispatch builds the runtime layer and forwards exactly the
/// parsed config to the async server function. (Tracing is installed at
/// the process boundary in `run_process`, not here.)
#[test]
fn startup_dispatch_invokes_async_server_with_config() -> Result<()> {
    let startup = LspStartup {
        workspace_root: PathBuf::from("/tmp/deslop-async"),
        min_nodes: 11,
        worker_threads: 1,
        nice: 0,
        embedding: LspEmbeddingConfig {
            mode: EmbeddingMode::Required,
            provider_id: "ollama".to_owned(),
            model_id: "async-model".to_owned(),
            endpoint: "http://127.0.0.1:1234".to_owned(),
        },
        ipc_mode: IpcMode::Tcp,
        ranking_structural_only: None,
    };
    let mut observed: Option<LspStartup> = None;

    run_startup_with(startup, |workspace_root, min_nodes, embedding, ipc_mode| {
        observed = Some(LspStartup {
            workspace_root,
            min_nodes,
            worker_threads: 0,
            nice: 0,
            embedding,
            ipc_mode,
            ranking_structural_only: None,
        });
        std::future::ready(Ok(()))
    })?;

    let observed = observed.ok_or_else(|| anyhow!("async server runner was not called"))?;
    assert_eq!(observed.workspace_root, PathBuf::from("/tmp/deslop-async"));
    assert_eq!(observed.min_nodes, 11);
    assert_eq!(observed.embedding.mode, EmbeddingMode::Required);
    assert_eq!(observed.embedding.provider_id, "ollama");
    assert_eq!(observed.embedding.model_id, "async-model");
    assert_eq!(observed.embedding.endpoint, "http://127.0.0.1:1234");
    assert_eq!(observed.ipc_mode, IpcMode::Tcp);
    Ok(())
}

/// Async startup errors propagate back to the process adapter instead
/// of being swallowed inside the runtime layer.
#[test]
fn startup_dispatch_propagates_async_server_error() -> Result<()> {
    let startup = serve_startup(action_from_args(["deslop-lsp", "/tmp/deslop-error"])?)?;
    let error = run_startup_with(
        startup,
        |_workspace_root, _min_nodes, _embedding, _ipc_mode| {
            std::future::ready(Err(anyhow!("async server failed")))
        },
    )
    .err()
    .ok_or_else(|| anyhow!("startup dispatch should have returned an error"))?;
    assert!(format!("{error:#}").contains("async server failed"));
    Ok(())
}

/// Invalid argv is rejected before server startup, with concrete
/// messages users can act on.
#[test]
fn invalid_arguments_return_user_facing_errors() -> Result<()> {
    assert_error_contains(["deslop-lsp"], "usage: deslop-lsp")?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--worker-threads"],
        "--worker-threads requires",
    )?;
    assert_error_contains(["deslop-lsp", "/tmp/ws", "--nice"], "--nice requires")?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--nice", "20"],
        "--nice must be in the range",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--unknown"],
        "unsupported LSP startup flag",
    )?;
    Ok(())
}

/// [LIVE-IPC-TCP] `--ipc-transport` selects either transport, falls
/// back to the platform default, and rejects unknown or missing
/// values with actionable messages.
#[test]
fn ipc_transport_flag_parses_defaults_and_rejects() -> Result<()> {
    let tcp = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/ws",
        "--ipc-transport",
        "tcp",
    ])?)?;
    assert_eq!(tcp.ipc_mode, IpcMode::Tcp);
    let unix = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/ws",
        "--ipc-transport",
        "unix",
    ])?)?;
    assert_eq!(unix.ipc_mode, IpcMode::Unix);
    let default = serve_startup(action_from_args(["deslop-lsp", "/tmp/ws"])?)?;
    assert_eq!(default.ipc_mode, IpcMode::platform_default());
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--ipc-transport", "carrier-pigeon"],
        "--ipc-transport must be `unix` or `tcp`",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--ipc-transport"],
        "--ipc-transport requires",
    )?;
    Ok(())
}

/// [RANK-STRUCTURAL-ONLY] / [VSIX-SETTINGS-RANKING]:
/// `--ranking-structural-only` parses every policy, defaults to
/// deferring to `.deslop.toml`, rejects unknown values, and reaches
/// the process-wide override when startup dispatch runs.
#[test]
fn ranking_structural_only_flag_parses_applies_and_rejects() -> Result<()> {
    let demote = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/ws",
        "--ranking-structural-only",
        "demote",
    ])?)?;
    assert_eq!(demote.ranking_structural_only, Some(ClonePolicy::Demote));
    let keep = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/ws",
        "--ranking-structural-only",
        "keep",
    ])?)?;
    assert_eq!(keep.ranking_structural_only, Some(ClonePolicy::Keep));
    let unset = serve_startup(action_from_args(["deslop-lsp", "/tmp/ws"])?)?;
    assert_eq!(unset.ranking_structural_only, None);
    assert_error_contains(
        [
            "deslop-lsp",
            "/tmp/ws",
            "--ranking-structural-only",
            "shout",
        ],
        "--ranking-structural-only: expected demote|ignore|keep",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--ranking-structural-only"],
        "--ranking-structural-only requires",
    )?;

    // Startup dispatch records the override in the central state
    // module so every later config load sees the editor's choice.
    let ignore = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/ws",
        "--ranking-structural-only",
        "ignore",
    ])?)?;
    run_startup_with(
        ignore,
        |_workspace_root, _min_nodes, _embedding, _ipc_mode| std::future::ready(Ok(())),
    )?;
    assert_eq!(
        deslop_core::state::structural_only_override(),
        Some(ClonePolicy::Ignore),
        "run_startup_with must record the [RANK-STRUCTURAL-ONLY] override \
         in deslop-core::state before the server starts"
    );
    Ok(())
}

/// Issue #83: VSIX settings must travel through initialize/configuration,
/// not through legacy process argv flags.
#[test]
fn issue_83_legacy_startup_flags_are_rejected() -> Result<()> {
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--min-nodes", "30"],
        "unsupported LSP startup flag",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--embeddings", "auto"],
        "unsupported LSP startup flag",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--embedding-provider", "ollama"],
        "unsupported LSP startup flag",
    )?;
    assert_error_contains(
        ["deslop-lsp", "/tmp/ws", "--embedding-model", "unit-model"],
        "unsupported LSP startup flag",
    )?;
    assert_error_contains(
        [
            "deslop-lsp",
            "/tmp/ws",
            "--embedding-endpoint",
            "http://127.0.0.1:9999",
        ],
        "unsupported LSP startup flag",
    )?;
    Ok(())
}

/// Issue #201 (showstopper): the transport flag is NEVER a workspace root.
///
/// When VS Code opens with no workspace folder, the extension launches the
/// server with no positional root and `vscode-languageclient` appends its
/// `--stdio` transport flag — so the argv is exactly `deslop-lsp --stdio`.
/// The crash log showed `workspace_root=--stdio`: the parser had taken the
/// flag as the positional root, the file watcher then failed to watch a path
/// literally named `--stdio` ("No path was found"), and the server exited 1
/// on a loop ("crashed 5 times"). A leading flag must resolve to a clean
/// usage error, never a bogus `workspace_root`.
#[test]
fn issue_201_transport_flag_is_never_the_workspace_root() -> Result<()> {
    // The exact argv from the crash log — no folder open, run mode.
    let error = action_from_args(["deslop-lsp", "--stdio"])
        .err()
        .ok_or_else(|| anyhow!("`deslop-lsp --stdio` must fail, not serve a bogus root"))?;
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("usage: deslop-lsp"),
        "no positional root ⇒ usage error, got {rendered:?}",
    );
    assert!(
        !rendered.contains("--stdio"),
        "the transport flag must never surface as a workspace-root path, got {rendered:?}",
    );

    // Debug launch with no folder: `buildServerArgs` yields `["--debug"]` and
    // the client appends `--stdio` → `deslop-lsp --debug --stdio`. Same rule.
    assert_error_contains(["deslop-lsp", "--debug", "--stdio"], "usage: deslop-lsp")?;

    // Happy path preserved: a real root followed by the appended transport
    // flag still resolves to the real root — exactly why a folder-open
    // session never hit this bug.
    let run = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/deslop-201",
        "--stdio",
    ])?)?;
    assert_eq!(run.workspace_root, PathBuf::from("/tmp/deslop-201"));
    let debug = serve_startup(action_from_args([
        "deslop-lsp",
        "/tmp/deslop-201",
        "--debug",
        "--stdio",
    ])?)?;
    assert_eq!(debug.workspace_root, PathBuf::from("/tmp/deslop-201"));

    // Robustness: the root is the first non-flag argument, not strictly
    // `args[1]`. Even if a future `vscode-languageclient` PREPENDED the
    // transport flag instead of appending it, the real root still resolves
    // — so that library change could never silently reopen the crash-loop.
    let prepended = serve_startup(action_from_args([
        "deslop-lsp",
        "--stdio",
        "/tmp/deslop-201",
    ])?)?;
    assert_eq!(prepended.workspace_root, PathBuf::from("/tmp/deslop-201"));
    Ok(())
}

/// Extracts version output from a parsed action.
fn version_output(action: LspAction) -> Result<String> {
    match action {
        LspAction::Version { output } => Ok(output),
        other @ LspAction::Serve(_) => Err(anyhow!("expected version action, got {other:?}")),
    }
}

/// Extracts startup configuration from a parsed action.
fn serve_startup(action: LspAction) -> Result<LspStartup> {
    match action {
        LspAction::Serve(startup) => Ok(startup),
        other @ LspAction::Version { .. } => Err(anyhow!("expected serve action, got {other:?}")),
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
