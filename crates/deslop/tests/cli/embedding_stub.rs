//! Mock-Ollama coverage for [FUSION-EMBED-PROVIDER] CLI plumbing.
//!
//! [REMOVE-STUB] These tests previously used `--embedding-provider stub`
//! to exercise the embedding pipeline without a live Ollama. The stub
//! is no longer a production provider, so the same flows are now
//! driven against an in-process mock Ollama HTTP server.

use crate::mock_ollama::MockOllama;
use crate::support::*;

#[test]
fn default_run_records_embeddings_off_provenance() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-endpoint")
        .arg("http://127.0.0.1:1")
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-endpoint")
        .arg("http://127.0.0.1:1")
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
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("maybe")
        .assert()
        .failure()
        .stderr(contains("invalid --embeddings value"));
    Ok(())
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
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
    let mut first = Command::cargo_bin("deslop")?;
    let _assertion = first
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("first"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
    let cache_dir = scan_root
        .join(".deslop-cache")
        .join("embeddings")
        .join("ollama")
        .join("nomic-embed-text");
    assert!(
        cache_dir.is_dir(),
        "embedding cache directory missing: {}",
        cache_dir.display()
    );
    let mut second = Command::cargo_bin("deslop")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
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
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("imaginary-provider")
        .assert()
        .failure()
        .stderr(contains("unknown embedding provider"));
    Ok(())
}

// [REMOVE-STUB] The deterministic BLAKE3 stub is not a product
// provider. Passing `--embedding-provider stub` must be rejected the
// same way any unknown provider id is, so the CLI never silently
// accepts test infrastructure.
#[test]
fn stub_embedding_provider_is_rejected_in_production() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("stub")
        .assert()
        .failure()
        .stderr(contains("unknown embedding provider"));
    Ok(())
}
