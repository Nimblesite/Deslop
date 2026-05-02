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

// Implements [FUSION-EMBED-PROVIDER] stub provider: `--embedding-provider=stub
// --embeddings=required` runs the full embedding pipeline with a
// deterministic in-process provider. Exercises the HNSW pair
// generator, the cache round-trip, and the provenance rendering
// without needing a live Ollama.
#[test]
fn stub_provider_records_provenance_and_runs_embedding_pass() -> Result<()> {
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
        .arg("stub")
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"stub\""),
        "provenance provider_id missing: {json}"
    );
    assert!(
        json.contains("\"model_id\": \"blake3-stub\""),
        "provenance model_id missing"
    );
    assert!(
        json.contains("\"model_version\": \"v1\""),
        "provenance model_version missing"
    );
    let txt = fs::read_to_string(&out.txt)?;
    assert!(
        txt.contains("embeddings: stub/blake3-stub@v1"),
        "text provenance missing: {txt}"
    );
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("embeddings: stub/blake3-stub@v1"),
        "html provenance missing"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] cache round-trip: a second run
// against the same scan root must re-use the on-disk embedding
// cache rather than re-embedding from scratch. We verify the cache
// directory exists after the first run and the second run still
// succeeds (the stub provider is deterministic, so a cache miss
// would still produce the same vectors — but we check the directory
// to prove the cache path actually wrote files).
#[test]
fn stub_provider_populates_embedding_cache() -> Result<()> {
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
        .arg("stub")
        .assert()
        .success();
    let cache_dir = scan_root
        .join(".deslop-cache")
        .join("embeddings")
        .join("stub")
        .join("blake3-stub")
        .join("v1");
    assert!(
        cache_dir.is_dir(),
        "embedding cache directory missing: {}",
        cache_dir.display()
    );
    let cached_files = fs::read_dir(&cache_dir)?.count();
    assert!(
        cached_files > 0,
        "cache dir has no entries: {}",
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
        .arg("stub")
        .assert()
        .success();
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=auto` with a
// reachable provider: the pass succeeds and the report carries the
// provenance. Complements the failure-fallback test.
#[test]
fn stub_provider_under_auto_mode_runs_embedding_pass() -> Result<()> {
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
        .arg("stub")
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"stub\""),
        "auto mode with reachable provider must record provenance: {json}"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] unknown-provider rejection.
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

// ===========================================================================
// OLLAMA-LIVE TESTS — require a running local Ollama daemon on
// 127.0.0.1:11434 with the `nomic-embed-text` model pulled.
// The `ollama_` name prefix is the marker: `make ci` filters them
// out via `cargo test ... -- --skip ollama_`; `make ci-ollama` runs
// them via `cargo test ollama_`. Every test below pins
// `--embedding-model nomic-embed-text` so assertions against
// `model_id` stay honest even if a developer's shell exports a
// different default. Reports are parsed via `serde_json` so the
// assertions are schema-aware rather than substring-guessing.
// ===========================================================================

