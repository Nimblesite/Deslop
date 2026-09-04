use super::support::*;
use crate::common::signals::assert_no_pair_surface_on_cluster;
use crate::mock_ollama::{MockOllama, MOCK_CONTEXT_TOKENS};

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

/// Runs an Ollama-backed scan, varying only what the cases here
/// actually differ in: the output prefix, `min_nodes` and the embedding
/// mode. The model is the same in every case, so restating the whole
/// argument list per test is how a flag rename would silently reach only
/// some of them.
/// Pins one run's `cache_stats` hit/miss counters. The two-run cache
/// proofs each asserted the same two counters the same way; Deslop scored
/// the copies `structural_only` against this repo's own corpus.
fn assert_cache_counters(
    json: &serde_json::Value,
    missing: &str,
    hits: u64,
    why_hits: &str,
    misses: u64,
    why_misses: &str,
) -> Result<()> {
    let stats = object_field(json, "cache_stats", missing)?;
    assert_eq!(
        stats.get("hits").and_then(serde_json::Value::as_u64),
        Some(hits),
        "{why_hits}"
    );
    assert_eq!(
        stats.get("misses").and_then(serde_json::Value::as_u64),
        Some(misses),
        "{why_misses}"
    );
    Ok(())
}

/// One string field of the report's `embedding_provenance` block.
fn provenance_str<'a>(
    json: &'a serde_json::Value,
    missing: &str,
    key: &str,
) -> Result<Option<&'a str>> {
    Ok(object_field(json, "embedding_provenance", missing)?
        .get(key)
        .and_then(serde_json::Value::as_str))
}

/// Seeds `fixture` into a fresh temp scan root, runs `deslop` against a
/// live Ollama at `min_nodes` in `mode`, and returns the temp dir (bind
/// it — dropping it deletes the tree) with the paths the run wrote.
fn ollama_run(fixture: &str, min_nodes: &str, mode: &str) -> Result<(TempDir, RunOutputs)> {
    let (tmp, scan_root) = seed_scan(fixture)?;
    let outputs = outputs_under(tmp.path());
    run_ollama_scan(
        &scan_root,
        &tmp.path().join(REPORT_OUTPUT_STEM),
        min_nodes,
        mode,
    )?;
    Ok((tmp, outputs))
}

fn run_ollama_scan(
    scan_root: &Path,
    output_prefix: &Path,
    min_nodes: &str,
    mode: &str,
) -> Result<()> {
    run_deslop(
        scan_root,
        output_prefix,
        &[
            "--min-nodes",
            min_nodes,
            "--embeddings",
            mode,
            "--embedding-model",
            "nomic-embed-text",
        ],
    )
}

/// The behaviour-equivalence ground truth of the `csharp-type4`
/// fixture, declared to the mock embedder.
///
/// `Recursive.cs` and `Iterative.cs` implement the same three functions
/// two ways — the fixture's own comments say so — which is a Type-4
/// clone: equal behaviour, different text. No statistic over the text
/// can measure that, and the GH #369 mock is an honest content
/// statistic (a feature hash of 5-byte shingles), so it scores the pair
/// far below `MIN_COSINE`. Declaring the equivalence is what lets a
/// deterministic mock stand in for a model that has read both files;
/// the real `nomic-embed-text` measures this pair at cosine 0.97.
/// Every pair the groups do not name keeps its honest shingle cosine.
const TYPE4_BEHAVIOUR_GROUPS: &[&[&str]] = &[&["class Recursive", "class Iterative"]];

/// Runs the `deslop` binary over `scan_root` writing to `output_prefix`
/// with the given trailing `args` against a freshly-spawned happy-path
/// mock Ollama, asserting the process succeeds. The mock stays alive for
/// the synchronous run; its deterministic vectors keep the cache
/// round-trip tests converging across separate invocations.
fn run_deslop(scan_root: &Path, output_prefix: &Path, args: &[&str]) -> Result<()> {
    let server = MockOllama::spawn_semantic(TYPE4_BEHAVIOUR_GROUPS)?;
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

// GH#286 [FUSED-EMBED-PROVIDER]: the per-input budget belongs to the
// model, not the pipeline. The mock reports a 32,768-token context via
// `/api/show`, so a ~12k-char method must reach the provider instead of
// being dropped by the old hard-coded 6,000-char constant. An F# user
// lost 14,723 of 175,160 subtrees (8.4%) to that constant — silently,
// and precisely at the large end where re-derived duplication hides.
#[test]
fn issue_286_large_subtree_survives_when_the_model_declares_the_context() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-type4")?;
    let server = MockOllama::spawn()?;
    let oversized = 12_000;
    assert!(
        u64::try_from(oversized).unwrap_or(u64::MAX) < MOCK_CONTEXT_TOKENS.saturating_mul(3),
        "fixture must fit the declared budget or the test proves nothing"
    );
    fs::write(scan_root.join("Huge.cs"), huge_csharp_source(oversized))?;
    let out = outputs_under(tmp.path());

    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "15", "--embeddings", "required"])
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();

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
    assert!(
        server.max_embed_input_chars() >= oversized,
        "the provider declared room for {oversized} characters, but production sent no input larger than {} — a prefix vector must never represent the full subtree",
        server.max_embed_input_chars(),
    );
    assert!(
        !server.embed_truncation_enabled(),
        "every input passed the provider-derived budget; asking Ollama to truncate can silently associate a prefix vector with a full source range",
    );
    Ok(())
}

// Implements [FUSED-EMBED-PROVIDER] Type-4 end-to-end: the fixture
// pairs recursive and iterative implementations of factorial /
// fibonacci / sum-to-n. Without embeddings the two files share no
// structural or token signal. With live Ollama, the embedding pass
// must produce a *cross-file* cluster whose `embedding_cos > 0.3`
// and whose fused score preserves the strongest component while staying
// in the public confidence range.
#[test]
fn ollama_type4_cross_file_cluster_has_positive_embedding_signal() -> Result<()> {
    let (_tmp, out) = ollama_run("csharp-type4", "15", "required")?;
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
            anyhow::anyhow!("no cross-file cluster spanning Recursive.cs + Iterative.cs: {json:#}")
        })?;
    // The mass-only wire carries no cluster `signals` object: admission
    // evidence is pair-scoped ([FUSED-PAIR-SIGNALS]). What proves the
    // embedding route reached the report is the cluster's mere presence
    // (a Type-4 pair clusters on embedding evidence alone), the
    // provenance triple above, and the clean surface — no pair-only
    // field may sit on the cluster.
    assert_no_pair_surface_on_cluster(&cluster, "type-4 embedding cluster");
    assert!(
        cluster
            .get("mass")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|mass| mass > 0),
        "the embedding-admitted cluster must carry positive mass: {cluster:#}",
    );
    let occurrences = cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    assert!(
        occurrences >= 2,
        "the embedding-admitted cluster must report both copies: {cluster:#}",
    );
    Ok(())
}

// Implements [FUSED-EMBED-PROVIDER] auto mode: when Ollama is
// reachable, `--embeddings=auto` must silently upgrade to the live
// provider and record provenance. Complements
// `embeddings_auto_falls_back_when_provider_unreachable` which
// exercises the fallback direction against a dead endpoint.
#[test]
fn ollama_auto_mode_populates_provenance_when_reachable() -> Result<()> {
    let (_tmp, out) = ollama_run("csharp-small", "8", "auto")?;
    let json = load_report_json(&out.json)?;
    assert_eq!(
        provenance_str(
            &json,
            "auto mode with reachable Ollama must populate provenance",
            "provider_id"
        )?,
        Some("ollama"),
    );
    Ok(())
}

// Implements [FUSED-EMBED-PROVIDER] cache round-trip: the first
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

    run_ollama_scan(&scan_root, &tmp.path().join("first"), "15", "required")?;

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
    run_ollama_scan(&scan_root, &tmp.path().join("second"), "15", "required")?;
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs() < 10,
        "second run took {elapsed:?} — cache is not being used (cold Ollama runs take 30s+)",
    );

    let json = load_report_json(&tmp.path().join("second.json"))?;
    assert_eq!(
        provenance_str(&json, "second run lost provenance", "model_id")?,
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
    let (_tmp, out) = ollama_run("csharp-small", "8", "required")?;
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

// Implements [FUSED-EMBED-PROVIDER] × [PIPELINE-INCREMENTAL]: the
// two caches live side-by-side under `.deslop/cache/` and
// invalidate independently. The first run populates both
// (`fingerprints/...` and `embeddings/...`); the second run hits
// the fingerprint cache for every file AND reuses every embedding
// from disk, producing the same cross-file cluster as a cold run.
#[test]
fn ollama_incremental_plus_embeddings_second_run_hits_both_caches() -> Result<()> {
    let (tmp, scan_root) = seed_scan("csharp-type4")?;

    run_ollama_scan(&scan_root, &tmp.path().join("first"), "15", "required")?;

    let first_json = load_report_json(&tmp.path().join("first.json"))?;
    assert_cache_counters(
        &first_json,
        "first run missing cache_stats",
        0,
        "first incremental run must be a clean miss",
        2,
        "first incremental run must register both files as misses",
    )?;

    run_ollama_scan(&scan_root, &tmp.path().join("second"), "15", "required")?;
    let second_json = load_report_json(&tmp.path().join("second.json"))?;
    assert_cache_counters(
        &second_json,
        "second run missing cache_stats",
        2,
        "second run must hit the fingerprint cache for both files",
        0,
        "second run must have zero fingerprint-cache misses",
    )?;

    let cluster = find_cross_file_cluster(&second_json, &["Recursive.cs", "Iterative.cs"])
        .ok_or_else(|| anyhow::anyhow!("cached run lost the cross-file cluster"))?;
    // Admission evidence is pair-scoped on the mass-only wire, so the
    // cached run proves the embedding route by the cluster's presence,
    // its mass fields, and the provenance the second run must carry.
    assert!(
        cluster
            .get("mass")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|mass| mass > 0),
        "second run must preserve the embedding-admitted cluster with mass: {cluster:#}",
    );
    assert_no_pair_surface_on_cluster(&cluster, "cached type-4 cluster");
    Ok(())
}
