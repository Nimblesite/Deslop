//! Unit tests for the cache-hit validation path in [`super`]
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]): stored signatures are
//! positionally bound to the stored fingerprints, so a blob whose
//! fingerprints disagree with the re-derived set must be rejected as a
//! miss and self-healed — never served.

use std::path::PathBuf;

use super::*;
use crate::state::FileRegistry;

/// One small Rust function that fingerprints at `min_nodes = 8`.
const SOURCE: &[u8] = b"pub fn twice(value: i32) -> i32 {\n    value + value\n}\n";

/// Subtree-size floor used throughout these tests.
const MIN_NODES: usize = 8;

// [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] A stored bundle whose
// fingerprints cannot be reproduced from its own tree must be treated
// as a whole-file miss (its signatures are unattributable), rebuilt
// from source, and overwritten so the very next pass hits cleanly.
#[test]
fn tampered_fingerprints_invalidate_stored_signatures_and_self_heal() -> Result<(), String> {
    let parsers = default_parsers();
    let parser = parser_for_language(&parsers, "rust").ok_or("rust parser must be registered")?;
    let tmp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let cache = FingerprintCache::open(tmp.path(), "rust", 8).map_err(|error| error.to_string())?;
    let file_id = FileRegistry::new().register(PathBuf::from("fixture.rs"));
    let mut honest_build = CorpusBuildState::default();
    let honest = build_cached_file(parser, SOURCE, file_id, MIN_NODES, &mut honest_build)
        .map_err(|error| error.to_string())?;
    assert!(
        !honest.fingerprints.is_empty(),
        "fixture must fingerprint at least one subtree"
    );
    store_forged_bundle(&cache, &honest)?;

    let mut stats = CacheStats::default();
    let mut build_stats = CorpusBuildState::default();
    let served = load_or_parse_file(
        Some(&cache),
        parser,
        SOURCE,
        file_id,
        MIN_NODES,
        &mut stats,
        &mut build_stats,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        (stats.hits, stats.misses),
        (0, 1),
        "a blob whose fingerprints disagree with the re-derived set must miss, never hit"
    );
    assert_eq!(
        build_stats.stats.signatures_reused, 0,
        "no signature may be attached off a rejected blob"
    );
    assert_eq!(
        build_stats.stats.signatures_built,
        signature_count(&served),
        "every signature must be rebuilt from token streams after the rejection"
    );
    assert_eq!(
        served.fingerprints, honest.fingerprints,
        "the served bundle must carry the honest re-derived fingerprints"
    );
    assert_eq!(
        served.signatures, honest.signatures,
        "rebuilt signatures must equal the honest bundle's, slot for slot"
    );
    assert_healed(&cache, parser, file_id, &honest)
}

/// Stores a copy of `honest` whose fingerprints all carry an inflated
/// `node_count`, so re-derivation can never reproduce them.
fn store_forged_bundle(cache: &FingerprintCache, honest: &CachedFile) -> Result<(), String> {
    let mut forged_fingerprints = honest.fingerprints.clone();
    for fingerprint in &mut forged_fingerprints {
        fingerprint.node_count = fingerprint.node_count.saturating_add(1);
    }
    let forged = CachedFile {
        tree: honest.tree.clone(),
        fingerprints: forged_fingerprints,
        signatures: honest.signatures.clone(),
    };
    cache
        .store(SOURCE, &forged)
        .map_err(|error| error.to_string())
}

/// Asserts the pass after the rejection hits cleanly — the rejected
/// blob was overwritten with the honest bundle, and its signatures are
/// attached rather than rebuilt.
fn assert_healed(
    cache: &FingerprintCache,
    parser: &dyn LanguageParser,
    file_id: FileId,
    honest: &CachedFile,
) -> Result<(), String> {
    let mut stats = CacheStats::default();
    let mut build_stats = CorpusBuildState::default();
    let healed = load_or_parse_file(
        Some(cache),
        parser,
        SOURCE,
        file_id,
        MIN_NODES,
        &mut stats,
        &mut build_stats,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        (stats.hits, stats.misses),
        (1, 0),
        "the self-healed blob must hit on the very next pass"
    );
    assert_eq!(
        build_stats.stats.signatures_built, 0,
        "a validated hit must rebuild no signatures"
    );
    assert_eq!(
        build_stats.stats.signatures_reused,
        signature_count(&healed),
        "the healed blob's signatures must be attached from the store"
    );
    assert_eq!(
        healed.fingerprints, honest.fingerprints,
        "healed fingerprints must equal the honest set"
    );
    Ok(())
}

/// A bundle's signature count as the `u64` the counters use.
fn signature_count(cached: &CachedFile) -> u64 {
    u64::try_from(cached.signatures.len()).unwrap_or(u64::MAX)
}

/// Files in the shard-parity fixture — enough that every tested worker
/// count produces a different shard split (7 files over 1/2/5/16
/// workers shard as 7 / 4+3 / 2+2+2+1 / 1-each).
const PARITY_FILES: usize = 7;

/// Worker counts the parity pin sweeps: serial, uneven shards, and
/// more workers than files.
const PARITY_WORKER_COUNTS: [usize; 4] = [1, 2, 5, 16];

/// Node floor for the parity fixture — small enough that every file
/// contributes both exact-node and sibling-window fingerprints.
const PARITY_MIN_NODES: u32 = 3;

/// One distinct-per-index Python file: the loop shape fingerprints and
/// carries a real token stream, and the index in names and literals
/// keeps every file's records distinguishable in the merged corpus.
fn parity_source(index: usize) -> String {
    format!(
        "def compute_{index}(values):\n    total = {index}\n    for value in values:\n        if value > {index}:\n            total = total + value\n    return total\n"
    )
}

/// The flattened signature population, across however many segments
/// the sharded merge produced.
fn flattened_signatures(corpus: &FingerprintCorpus) -> Vec<crate::lsh::Signature> {
    corpus.signatures.iter().flatten().copied().collect()
}

/// Comparable view of the boilerplate ranges.
fn boilerplate_keys(corpus: &FingerprintCorpus) -> Vec<(FileId, &'static str, usize, usize)> {
    corpus
        .boilerplate_ranges
        .iter()
        .map(|range| {
            (
                range.file_id,
                range.language,
                range.byte_range.start,
                range.byte_range.end,
            )
        })
        .collect()
}

// [PERF-FLUTTER-TODO-CORPUS] The cold sharded corpus build is a pure
// function of the file list — never of the machine's parallelism. One
// worker is the serial construction, so holding every worker count to
// the one-worker output pins the ordered shard merge end to end:
// fingerprint order, the flattened signature population, per-file
// entries, sources, line counts, and boilerplate ranges
// (`docs/performance-branch-review.md`, "Large parallel paths lack
// black-box parity coverage").
#[test]
fn cold_corpus_is_identical_for_any_worker_count() -> Result<(), String> {
    let tmp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut registry = FileRegistry::new();
    let mut files = Vec::new();
    for index in 0..PARITY_FILES {
        let path = tmp.path().join(format!("parity_{index}.py"));
        std::fs::write(&path, parity_source(index)).map_err(|error| error.to_string())?;
        let file_id = registry.register(path.clone());
        files.push(crate::discover::DiscoveredFile {
            path,
            file_id,
            extension: "py".to_owned(),
            language: "python",
        });
    }
    let parsers = default_parsers();
    let config = PipelineConfig {
        root: tmp.path().to_path_buf(),
        min_nodes: PARITY_MIN_NODES,
        config_path: None,
        embedding: crate::pipeline::config::EmbeddingSettings {
            mode: crate::embedding::EmbeddingMode::Off,
            provider: None,
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    };

    let serial = fingerprint_corpus_with_workers(&files, &parsers, &config, 1)
        .map_err(|error| format!("serial build failed: {error}"))?;
    assert_eq!(
        serial.per_file.len(),
        PARITY_FILES,
        "every fixture file must contribute an entry"
    );
    assert!(
        serial.fingerprints.len() > PARITY_FILES,
        "the fixture must fingerprint, got {}",
        serial.fingerprints.len()
    );
    assert_eq!(
        serial.fingerprints.len(),
        flattened_signatures(&serial).len(),
        "signatures must stay positionally 1:1 with fingerprints"
    );

    for workers in PARITY_WORKER_COUNTS {
        let sharded = fingerprint_corpus_with_workers(&files, &parsers, &config, workers)
            .map_err(|error| format!("{workers}-worker build failed: {error}"))?;
        assert_eq!(
            sharded.fingerprints, serial.fingerprints,
            "{workers} workers: fingerprint order must match the serial build"
        );
        assert_eq!(
            flattened_signatures(&sharded),
            flattened_signatures(&serial),
            "{workers} workers: the flattened signature population must match"
        );
        assert_eq!(
            sharded.per_file, serial.per_file,
            "{workers} workers: per-file entries must match"
        );
        assert_eq!(
            sharded.sources, serial.sources,
            "{workers} workers: retained sources must match"
        );
        assert_eq!(
            sharded.analysed_lines, serial.analysed_lines,
            "{workers} workers: analysed line counts must match"
        );
        assert_eq!(
            boilerplate_keys(&sharded),
            boilerplate_keys(&serial),
            "{workers} workers: boilerplate ranges must match"
        );
    }
    Ok(())
}
