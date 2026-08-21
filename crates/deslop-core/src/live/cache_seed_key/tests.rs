//! Unit tests for [`super`] — [LIVE-CACHE-SEED-KEY].
//!
//! One case per component of the key, each mutating exactly that
//! component and requiring the seed to be refused. A cache seed is
//! served to the editor as an answer, so "accepted" here means "an
//! analysis run under these settings is allowed to speak for a session
//! running under those settings".

use std::path::Path;

use super::*;

/// The floor the reference key is built at.
const MIN_NODES: u32 = 8;

/// A different floor — a session that would cluster different subtrees.
const OTHER_MIN_NODES: u32 = 40;

/// A provider identity to vary against.
fn spec(model_version: &str) -> EmbeddingSpec {
    EmbeddingSpec {
        provider_id: "ollama".to_owned(),
        model_id: "nomic-embed-text".to_owned(),
        model_version: model_version.to_owned(),
        dimensions: 768,
    }
}

/// The reference key: `min_nodes = 8`, incremental, no config file,
/// embeddings off.
fn reference(root: &Path) -> CacheSeedKey {
    CacheSeedKey::new(root, MIN_NODES, true, None, EmbeddingMode::Off, &spec("1"))
}

/// Records `key` beside a fresh cache dir under `root`.
fn record(root: &Path, key: &CacheSeedKey) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(crate::paths::cache_dir(root))?;
    write_seed_key(root, key);
    Ok(())
}

// The baseline: a run that recorded its own key accepts its own seed.
// Without this the refusals below would be satisfied by a predicate
// that refuses everything.
#[test]
fn a_run_accepts_the_seed_it_wrote_itself() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    record(temp.path(), &reference(temp.path()))?;
    assert!(
        seed_key_matches(temp.path(), &reference(temp.path())),
        "an unchanged run must warm-start from its own cache — refusing \
         every seed would make [LIVE-CACHE-SEED] pointless"
    );
    Ok(())
}

// No recorded key at all: a report of unknown provenance. This is also
// the upgrade path — every cache written before the key existed.
#[test]
fn a_seed_with_no_recorded_key_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::create_dir_all(crate::paths::cache_dir(temp.path()))?;
    assert!(
        !seed_key_matches(temp.path(), &reference(temp.path())),
        "a cached report with no recorded provenance could have been \
         produced by any settings, including settings whose byte offsets \
         point somewhere else entirely"
    );
    Ok(())
}

// `min_nodes` decides which subtrees are even candidates, so a report
// computed at another floor is a different analysis.
#[test]
fn a_seed_computed_at_another_node_floor_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    record(temp.path(), &reference(temp.path()))?;
    let other = CacheSeedKey::new(
        temp.path(),
        OTHER_MIN_NODES,
        true,
        None,
        EmbeddingMode::Off,
        &spec("1"),
    );
    assert!(
        !seed_key_matches(temp.path(), &other),
        "a report clustered at {MIN_NODES} nodes does not answer for a \
         session running at {OTHER_MIN_NODES}"
    );
    Ok(())
}

// The incremental flag decides whether the fingerprint cache is
// consulted, which decides what the pass measured.
#[test]
fn a_seed_from_the_other_incremental_setting_is_refused() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    record(temp.path(), &reference(temp.path()))?;
    let other = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        false,
        None,
        EmbeddingMode::Off,
        &spec("1"),
    );
    assert!(
        !seed_key_matches(temp.path(), &other),
        "the incremental setting is part of what produced the report"
    );
    Ok(())
}

// Turning embeddings on adds a whole similarity axis: clusters exist in
// the seeded report that this session would never have found, and vice
// versa.
#[test]
fn a_seed_scored_without_embeddings_does_not_answer_for_a_run_with_them(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    record(temp.path(), &reference(temp.path()))?;
    let other = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        true,
        None,
        EmbeddingMode::Required,
        &spec("1"),
    );
    assert!(
        !seed_key_matches(temp.path(), &other),
        "an embeddings-off report carries no semantic axis; serving it to \
         an embeddings-on session hides every Type-4 cluster the session \
         exists to show"
    );
    Ok(())
}

// Same mode, different model: the same axis measured by a different
// instrument.
#[test]
fn a_seed_scored_by_another_model_version_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let recorded = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        true,
        None,
        EmbeddingMode::Required,
        &spec("1"),
    );
    record(temp.path(), &recorded)?;
    let other = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        true,
        None,
        EmbeddingMode::Required,
        &spec("2"),
    );
    assert!(
        !seed_key_matches(temp.path(), &other),
        "a re-pulled or re-versioned model produces different cosines, so \
         it produces different clusters"
    );
    assert!(
        seed_key_matches(temp.path(), &recorded),
        "the model that wrote the seed still accepts it — the refusal \
         above is about the version and nothing else"
    );
    Ok(())
}

// A config file that has been *edited* keeps its path, so the path
// alone cannot detect it. The digest is what does.
#[test]
fn a_seed_scoped_by_an_edited_config_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join(".deslop.toml");
    std::fs::write(&config, b"[report]\nhide = []\n")?;
    let recorded = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        true,
        Some(config.as_path()),
        EmbeddingMode::Off,
        &spec("1"),
    );
    record(temp.path(), &recorded)?;
    assert!(
        seed_key_matches(temp.path(), &recorded),
        "an untouched config re-uses its seed"
    );

    std::fs::write(&config, b"[report]\nhide = [\"vendor/**\"]\n")?;
    let edited = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        true,
        Some(config.as_path()),
        EmbeddingMode::Off,
        &spec("1"),
    );
    assert!(
        !seed_key_matches(temp.path(), &edited),
        "the config path did not change — only its contents did — so a \
         path-only key would have served a report scoped to files the user \
         has since excluded"
    );
    Ok(())
}

// Adding a config file where there was none is the same class of change
// as editing one.
#[test]
fn a_seed_taken_before_a_config_existed_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    record(temp.path(), &reference(temp.path()))?;
    let config = temp.path().join(".deslop.toml");
    std::fs::write(&config, b"[report]\nhide = [\"vendor/**\"]\n")?;
    let with_config = CacheSeedKey::new(
        temp.path(),
        MIN_NODES,
        true,
        Some(config.as_path()),
        EmbeddingMode::Off,
        &spec("1"),
    );
    assert!(
        !seed_key_matches(temp.path(), &with_config),
        "an unconfigured report does not answer for a configured session"
    );
    Ok(())
}

// The whole file, not a prefix: a key that differs only in its last
// component must be refused too.
#[test]
fn the_comparison_reads_the_whole_key_not_a_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    record(temp.path(), &reference(temp.path()))?;
    let truncated = crate::paths::cache_dir(temp.path()).join(SEED_KEY_FILE_NAME);
    let recorded = std::fs::read_to_string(&truncated)?;
    let head = recorded.lines().take(2).collect::<Vec<_>>().join("\n");
    std::fs::write(&truncated, head)?;
    assert!(
        !seed_key_matches(temp.path(), &reference(temp.path())),
        "a key file holding only the first two components describes only \
         the first two components: {recorded}"
    );
    Ok(())
}
