//! Mock-Ollama coverage for [FUSION-EMBED-PROVIDER] CLI plumbing.
//!
//! [REMOVE-STUB] These tests previously used `--embedding-provider stub`
//! to exercise the embedding pipeline without a live Ollama. The stub
//! is no longer a production provider, so the same flows are now
//! driven against an in-process mock Ollama HTTP server.

use super::support::*;
use crate::mock_ollama::MockOllama;

/// Runs the full ollama embedding pass for `mode` (`required`/`auto`)
/// against `scan_root`, writing to `output_prefix`, and asserts the CLI
/// succeeds. Shared by every mock-Ollama happy-path test so the
/// `--embedding-provider ollama` arg array lives in one place.
fn run_ollama_pass(
    scan_root: &Path,
    output_prefix: &Path,
    mode: &str,
    endpoint: &str,
) -> Result<()> {
    let mut cmd = deslop_command(scan_root, output_prefix)?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "8",
            "--embeddings",
            mode,
            "--embedding-provider",
            "ollama",
            "--embedding-model",
            "nomic-embed-text",
            "--embedding-endpoint",
            endpoint,
        ])
        .assert()
        .success();
    Ok(())
}

/// Runs the CLI with `args` against the `csharp-small` fixture and
/// asserts it exits non-zero with `expected` on stderr. Shared by the
/// argument/provider rejection tests.
fn assert_cli_rejects(args: &[&str], expected: &str) -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = fixture_command("csharp-small", &tmp.path().join("report"))?;
    let _assertion = cmd.args(args).assert().failure().stderr(contains(expected));
    Ok(())
}

#[test]
fn default_run_records_embeddings_off_provenance() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = fixture_command("csharp-small", &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"embedding_provenance\": null"),
        "default run must record embeddings=off: {json}"
    );
    let txt = fs::read_to_string(&out.txt)?;
    assert!(txt.contains("embeddings: off"), "text provenance missing");
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=required` fails
// hard when the provider is unreachable. Uses an endpoint we know
// cannot resolve (port 1) so the probe always fails regardless of
// whether Ollama happens to be running on the developer machine.
#[test]
fn embeddings_required_hard_fails_when_provider_unreachable() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = fixture_command("csharp-small", &tmp.path().join("report"))?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "8",
            "--embeddings",
            "required",
            "--embedding-endpoint",
            "http://127.0.0.1:1",
        ])
        .assert()
        .failure()
        .stderr(contains("unreachable"));
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=auto` falls back
// silently when the provider is unreachable — the pipeline must
// still produce a report with `embedding_provenance: null`.
#[test]
fn embeddings_auto_falls_back_when_provider_unreachable() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = fixture_command("csharp-small", &tmp.path().join("report"))?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "8",
            "--embeddings",
            "auto",
            "--embedding-endpoint",
            "http://127.0.0.1:1",
        ])
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"embedding_provenance\": null"),
        "auto must fall back to off when provider is down: {json}"
    );
    Ok(())
}

// Implements [CLI-ARG-EMBEDDINGS]: invalid `--embeddings` values are
// rejected with a clear error message.
#[test]
fn embeddings_flag_rejects_unknown_values() -> Result<()> {
    assert_cli_rejects(&["--embeddings", "maybe"], "invalid --embeddings value")
}

// Implements [FUSION-EMBED-PROVIDER]: `--embeddings=required` runs
// the full embedding pipeline end-to-end. Uses a mock Ollama HTTP
// server so the production ollama provider, registry lookup, and
// cache round-trip all exercise live code without needing a real
// Ollama install.
#[test]
fn mock_ollama_records_provenance_and_runs_embedding_pass() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-type3"), &scan_root)?;
    run_ollama_pass(
        &scan_root,
        &tmp.path().join("report"),
        "required",
        server.endpoint(),
    )?;
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"ollama\""),
        "provenance provider_id missing: {json}"
    );
    assert!(
        json.contains("\"model_id\": \"nomic-embed-text\""),
        "provenance model_id missing"
    );
    let txt = fs::read_to_string(&out.txt)?;
    assert!(
        txt.contains("embeddings: ollama/nomic-embed-text@"),
        "text provenance missing: {txt}"
    );
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("embeddings: ollama/nomic-embed-text@"),
        "html provenance missing"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] cache round-trip: a second run
// against the same scan root must re-use the on-disk embedding cache.
// With a deterministic mock Ollama the cache key is stable and the
// directory must be populated after the first pass.
#[test]
fn mock_ollama_populates_embedding_cache() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    run_ollama_pass(
        &scan_root,
        &tmp.path().join("first"),
        "required",
        server.endpoint(),
    )?;
    let cache_dir = scan_root
        .join(".deslop/cache")
        .join("embeddings")
        .join("ollama")
        .join("nomic-embed-text");
    assert!(
        cache_dir.is_dir(),
        "embedding cache directory missing: {}",
        cache_dir.display()
    );
    run_ollama_pass(
        &scan_root,
        &tmp.path().join("second"),
        "required",
        server.endpoint(),
    )?;
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=auto` with a
// reachable provider: the pass succeeds and the report carries the
// provenance. Complements the failure-fallback test.
#[test]
fn mock_ollama_under_auto_mode_runs_embedding_pass() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    run_ollama_pass(
        &scan_root,
        &tmp.path().join("report"),
        "auto",
        server.endpoint(),
    )?;
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"ollama\""),
        "auto mode with reachable provider must record provenance: {json}"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] unknown-provider rejection. The
// production CLI no longer accepts the deterministic stub provider —
// the only registered production provider is `ollama`.
#[test]
fn unknown_embedding_provider_is_rejected() -> Result<()> {
    assert_cli_rejects(
        &[
            "--embeddings",
            "auto",
            "--embedding-provider",
            "imaginary-provider",
        ],
        "unknown embedding provider",
    )
}

// [REMOVE-STUB] The deterministic BLAKE3 stub is not a product
// provider. Passing `--embedding-provider stub` must be rejected the
// same way any unknown provider id is, so the CLI never silently
// accepts test infrastructure.
#[test]
fn stub_embedding_provider_is_rejected_in_production() -> Result<()> {
    assert_cli_rejects(
        &["--embeddings", "auto", "--embedding-provider", "stub"],
        "unknown embedding provider",
    )
}
