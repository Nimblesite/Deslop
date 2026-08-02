use crate::mock_ollama::{MockOllama, MOCK_CONTEXT_TOKENS};
use crate::support::*;

// different default. Reports are parsed via `serde_json` so the
// assertions are schema-aware rather than substring-guessing.
// ===========================================================================

/// Walks every cluster in `json` and returns the first whose
/// occurrences cover every file name in `required`. Used to pick
/// out the cross-file Type-4 cluster (Recursive.cs + Iterative.cs)
/// from the many within-file sibling-window clusters the fixture
/// also produces.
fn find_cross_file_cluster(
    json: &serde_json::Value,
    required: &[&str],
) -> Option<serde_json::Value> {
    let clusters = json.get("clusters")?.as_array()?;
    clusters
        .iter()
        .find(|cluster| {
            let Some(occurrences) = cluster.get("occurrences").and_then(|v| v.as_array()) else {
                return false;
            };
            let names: std::collections::HashSet<&str> = occurrences
                .iter()
                .filter_map(|occ| occ.get("path").and_then(|p| p.as_str()))
                .filter_map(|p| std::path::Path::new(p).file_name().and_then(|n| n.to_str()))
                .collect();
            required.iter().all(|needle| names.contains(needle))
        })
        .cloned()
}

/// Reads the JSON report at `path` and parses it into a
/// `serde_json::Value`. Tests assert against the parsed value so
/// trivial formatting changes in the renderer don't break them.
fn load_report_json(path: &Path) -> Result<serde_json::Value> {
    let raw = fs::read_to_string(path)?;
    let value = serde_json::from_str(&raw)?;
    Ok(value)
}

/// Creates a temp dir and seeds `<tmp>/src` from the named fixture,
/// returning the live `TempDir` (kept alive by the caller) and the
/// `src` scan root. Embedding/cache tests need a mutable scan root so
/// they can write cache siblings next to the sources.
fn seed_scan(fixture_name: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture(fixture_name), &scan_root)?;
    Ok((tmp, scan_root))
}

/// Runs the `deslop` binary over `scan_root` writing to `output_prefix`
/// with the given trailing `args` against a freshly-spawned happy-path
/// mock Ollama, asserting the process succeeds. The mock stays alive for
/// the synchronous run; its deterministic vectors keep the cache
/// round-trip tests converging across separate invocations.
fn run_deslop(scan_root: &Path, output_prefix: &Path, args: &[&str]) -> Result<()> {
    let server = MockOllama::spawn()?;
    let mut cmd = deslop_command(scan_root, output_prefix)?;
    let _assertion = cmd
        .args(args)
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
    Ok(())
}

/// Builds a valid C# file whose single method body exceeds
/// `minimum_chars`, used to prove a large subtree survives the
/// embedding pass when the provider declares room for it.
fn huge_csharp_source(minimum_chars: usize) -> String {
    let mut statements = String::new();
    while statements.chars().count() < minimum_chars {
        statements.push_str("            total = total + 1;\n");
    }
    format!(
        "namespace Big {{ public class Huge {{ public int Run(int seed) {{ var total = seed;\n{statements}            return total; }} }} }}\n"
    )
}

// GH#286 [FUSION-EMBED-PROVIDER]: the per-input budget belongs to the
// model, not the pipeline. The mock reports a 32,768-token context via
// `/api/show`, so a ~12k-char method must reach the provider instead of
// being dropped by the old hard-coded 6,000-char constant. An F# user
// lost 14,723 of 175,160 subtrees (8.4%) to that constant — silently,
// and precisely at the large end where re-derived duplication hides.
#[test]
fn issue_286_large_subtree_survives_when_the_model_declares_the_context() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-type4")?;
    let oversized = 12_000;
    assert!(
        u64::try_from(oversized).unwrap_or(u64::MAX) < MOCK_CONTEXT_TOKENS.saturating_mul(3),
        "fixture must fit the declared budget or the test proves nothing"
    );
    fs::write(scan_root.join("Huge.cs"), huge_csharp_source(oversized))?;
    let out = outputs_under(tmp.path());

    run_deslop(
        &scan_root,
        &tmp.path().join("report"),
        &["--min-nodes", "15", "--embeddings", "required"],
    )?;

    let json = load_report_json(&out.json)?;
    let provenance = object_field(
        &json,
        "embedding_provenance",
        "embedding_provenance missing or not an object",
    )?;
    let failed = provenance
        .get("failed_subtrees")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        failed, 0,
        "a {oversized}-char subtree must reach a provider declaring \
         {MOCK_CONTEXT_TOKENS} tokens of context, but it was dropped: {provenance:?}"
    );
    let indexed = provenance
        .get("indexed_subtrees")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    assert!(
        indexed > 0,
        "the oversized file must contribute indexed subtrees: {provenance:?}"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] Type-4 end-to-end: the fixture
// pairs recursive and iterative implementations of factorial /
// fibonacci / sum-to-n. Without embeddings the two files share no
// structural or token signal. With live Ollama, the embedding pass
// must produce a *cross-file* cluster whose `embedding_cos > 0.3`
// and whose fused score preserves the strongest component while staying
// in the public confidence range.
#[test]
fn ollama_type4_cross_file_cluster_has_positive_embedding_signal() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-type4")?;
    let out = outputs_under(tmp.path());
    run_deslop(
        &scan_root,
        &tmp.path().join("report"),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "required",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;
    let json = load_report_json(&out.json)?;
    let provenance = object_field(
        &json,
        "embedding_provenance",
        "embedding_provenance missing or not an object",
    )?;
    assert_eq!(
        provenance.get("provider_id").and_then(|v| v.as_str()),
        Some("ollama"),
        "provider_id must pin to ollama: {provenance:?}",
    );
    assert_eq!(
        provenance.get("model_id").and_then(|v| v.as_str()),
        Some("nomic-embed-text"),
        "model_id must pin to nomic-embed-text: {provenance:?}",
    );
    let model_version = provenance
        .get("model_version")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        !model_version.is_empty(),
        "model_version must be non-empty so cache keys change on weight updates: {provenance:?}",
    );
    assert!(
        provenance
            .get("dimensions")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|d| d > 0),
        "dimensions must be positive: {provenance:?}",
    );
    let cluster =
        find_cross_file_cluster(&json, &["Recursive.cs", "Iterative.cs"]).ok_or_else(|| {
            anyhow::anyhow!("no cross-file cluster spanning Recursive.cs + Iterative.cs")
        })?;
    let signals = object_field(&cluster, "signals", "cluster missing signals object")?;
    let embedding_cos = signals
        .get("embedding_cos")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    let token_jaccard = signals
        .get("token_jaccard")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    let structural = signals
        .get("structural")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    let fused = signals
        .get("fused")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    assert!(
        embedding_cos > 0.3,
        "Type-4 cross-file cluster must carry a meaningful embedding_cos, got {embedding_cos}"
    );
    let deterministic_max = structural.max(token_jaccard);
    assert!(
        fused >= deterministic_max,
        "fused score {fused} must preserve the best deterministic signal {deterministic_max}",
    );
    assert!(
        fused >= embedding_cos,
        "fused score {fused} must preserve the embedding signal {embedding_cos}",
    );
    assert!(
        fused <= 1.0,
        "fused score {fused} must stay in the public confidence range",
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] auto mode: when Ollama is
// reachable, `--embeddings=auto` must silently upgrade to the live
// provider and record provenance. Complements
// `embeddings_auto_falls_back_when_provider_unreachable` which
// exercises the fallback direction against a dead endpoint.
#[test]
fn ollama_auto_mode_populates_provenance_when_reachable() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-small")?;
    let out = outputs_under(tmp.path());
    run_deslop(
        &scan_root,
        &tmp.path().join("report"),
        &[
            "--min-nodes",
            "8",
            "--embeddings",
            "auto",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;
    let json = load_report_json(&out.json)?;
    let provenance = object_field(
        &json,
        "embedding_provenance",
        "auto mode with reachable Ollama must populate provenance",
    )?;
    assert_eq!(
        provenance.get("provider_id").and_then(|v| v.as_str()),
        Some("ollama"),
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] cache round-trip: the first
// run populates `.deslop/cache/embeddings/ollama/<model>/<version>/`
// with one `.bin` per fingerprint; the second run completes in a
// small fraction of the wall time because every embedding is
// served from disk. Each Ollama inference call is network-bound
// (tens of ms minimum), so a full re-embed of the fixture takes 30
// s or more. The 10 s cap catches cache misses without flaking.
#[test]
fn ollama_embedding_cache_persists_across_runs() -> Result<()> {
    use std::time::Instant;

    let (tmp, scan_root) = seed_scan("csharp-type4")?;

    run_deslop(
        &scan_root,
        &tmp.path().join("first"),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "required",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;

    let cache_root = scan_root
        .join(".deslop/cache")
        .join("embeddings")
        .join("ollama");
    let model_dir = fs::read_dir(&cache_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| anyhow::anyhow!("no model subdirectory under {}", cache_root.display()))?;
    let version_dir = fs::read_dir(&model_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| anyhow::anyhow!("no version subdirectory under {}", model_dir.display()))?;
    let cached_blob_count = fs::read_dir(&version_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "bin"))
        .count();
    assert!(
        cached_blob_count > 0,
        "first run must populate the embedding cache, found 0 .bin files in {}",
        version_dir.display(),
    );

    let started = Instant::now();
    run_deslop(
        &scan_root,
        &tmp.path().join("second"),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "required",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs() < 10,
        "second run took {elapsed:?} — cache is not being used (cold Ollama runs take 30s+)",
    );

    let json = load_report_json(&tmp.path().join("second.json"))?;
    let provenance = object_field(&json, "embedding_provenance", "second run lost provenance")?;
    assert_eq!(
        provenance.get("model_id").and_then(|v| v.as_str()),
        Some("nomic-embed-text"),
    );
    Ok(())
}

// Implements the rendered-view contract: both text and HTML views
// must surface the Ollama provenance line so a human or agent
// reading the report knows which model produced the
// `embedding_cos` signals. JSON is canonical; this guards the
// derived views against silent drift.
#[test]
fn ollama_provenance_surfaces_in_text_and_html() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-small")?;
    let out = outputs_under(tmp.path());
    run_deslop(
        &scan_root,
        &tmp.path().join("report"),
        &[
            "--min-nodes",
            "8",
            "--embeddings",
            "required",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;
    let text = fs::read_to_string(&out.txt)?;
    assert!(
        text.contains("embeddings: ollama/nomic-embed-text@"),
        "text renderer must carry the Ollama provenance line: {text}"
    );
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("embeddings: ollama/nomic-embed-text@"),
        "html renderer must carry the Ollama provenance line: {html}"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] × [PIPELINE-INCREMENTAL]: the
// two caches live side-by-side under `.deslop/cache/` and
// invalidate independently. The first run populates both
// (`fingerprints/...` and `embeddings/...`); the second run hits
// the fingerprint cache for every file AND reuses every embedding
// from disk, producing the same cross-file cluster as a cold run.
#[test]
fn ollama_incremental_plus_embeddings_second_run_hits_both_caches() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-type4")?;

    run_deslop(
        &scan_root,
        &tmp.path().join("first"),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "required",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;

    let first_json = load_report_json(&tmp.path().join("first.json"))?;
    let first_stats = object_field(&first_json, "cache_stats", "first run missing cache_stats")?;
    assert_eq!(
        first_stats.get("hits").and_then(serde_json::Value::as_u64),
        Some(0),
        "first incremental run must be a clean miss",
    );
    assert_eq!(
        first_stats
            .get("misses")
            .and_then(serde_json::Value::as_u64),
        Some(2),
        "first incremental run must register both files as misses",
    );

    run_deslop(
        &scan_root,
        &tmp.path().join("second"),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "required",
            "--embedding-model",
            "nomic-embed-text",
        ],
    )?;
    let second_json = load_report_json(&tmp.path().join("second.json"))?;
    let second_stats = object_field(
        &second_json,
        "cache_stats",
        "second run missing cache_stats",
    )?;
    assert_eq!(
        second_stats.get("hits").and_then(serde_json::Value::as_u64),
        Some(2),
        "second run must hit the fingerprint cache for both files",
    );
    assert_eq!(
        second_stats
            .get("misses")
            .and_then(serde_json::Value::as_u64),
        Some(0),
        "second run must have zero fingerprint-cache misses",
    );

    let cluster = find_cross_file_cluster(&second_json, &["Recursive.cs", "Iterative.cs"])
        .ok_or_else(|| anyhow::anyhow!("cached run lost the cross-file cluster"))?;
    let embedding_cos = cluster
        .get("signals")
        .and_then(|s| s.get("embedding_cos"))
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("cached cluster missing embedding_cos"))?;
    assert!(
        embedding_cos > 0.3,
        "second run must preserve embedding signal: got {embedding_cos}",
    );
    Ok(())
}
