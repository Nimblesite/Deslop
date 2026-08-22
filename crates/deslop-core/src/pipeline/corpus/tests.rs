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
