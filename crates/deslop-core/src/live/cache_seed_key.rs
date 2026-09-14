//! [LIVE-CACHE-SEED-KEY] — the compatibility contract a warm-start
//! cache must satisfy before it is served as an answer.
//!
//! [LIVE-CACHE-SEED] lets a live session answer queries instantly from
//! `{root}/.deslop/cache/live-report.json` while the real pass runs in
//! the background. The seed was accepted on one condition: that the
//! bytes deserialise as a [`Report`]. Nothing else was compared — not
//! the tool version that produced it, not the `min_nodes` it was
//! computed at, not the configuration that scoped it, not the embedding
//! provider that scored it. A report analysed at `--min-nodes 4` with
//! embeddings on was therefore served verbatim to a session running at
//! `--min-nodes 40` with embeddings off, and
//! [`super::freshness::FreshnessTracker`] then stamped the current
//! mtimes over those clusters, so the answer read as *fresh* rather
//! than as a placeholder.
//!
//! A stale seed is not a cosmetic problem: it is byte offsets from a
//! different analysis pointing into files the editor has since changed,
//! and a duplication figure computed under settings the user is no
//! longer running.
//!
//! So the writer records what produced the report, in a sibling file,
//! and the loader refuses a seed whose key is absent or different. The
//! report file itself keeps its shape — the MCP `lsp_not_running`
//! fallback documents it as a plain report — and the key lives beside
//! it.
//!
//! Ordering is deliberate: the report is written first and the key
//! second. A crash between them leaves the *previous* key, which either
//! still describes the run (the seed is an ordinary earlier generation,
//! which is all a seed ever is) or does not (the seed is refused). No
//! interleaving can produce an accepted incompatible seed.

use std::path::{Path, PathBuf};

use crate::embedding::{EmbeddingMode, EmbeddingSpec};

/// File written beside the state file, holding the key of the run that
/// produced it.
pub(super) const SEED_KEY_FILE_NAME: &str = "live-report.key";

/// The identity of an analysis run, as far as a cached report's
/// re-usability is concerned. Rendered as one line per component so a
/// mismatch names the component that changed rather than a hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CacheSeedKey {
    /// The rendered key text, compared verbatim.
    text: String,
}

impl CacheSeedKey {
    /// Builds the key for a run.
    pub(super) fn new(
        root: &Path,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<&Path>,
        mode: EmbeddingMode,
        spec: &EmbeddingSpec,
    ) -> Self {
        let embedding = match mode {
            EmbeddingMode::Off => "off".to_owned(),
            EmbeddingMode::Auto | EmbeddingMode::Required => format!(
                "{mode}/{provider}/{model}/{version}/{dimensions}",
                mode = mode.as_str(),
                provider = spec.provider_id,
                model = spec.model_id,
                version = spec.model_version,
                dimensions = spec.dimensions,
            ),
        };
        Self {
            text: format!(
                "tool_version={tool_version}\nroot={root}\nmin_nodes={min_nodes}\n\
                 incremental={incremental}\nconfig={config}\nembedding={embedding}\n",
                tool_version = crate::version(),
                root = normalised_root(root).display(),
                config = config_identity(config_path),
            ),
        }
    }

    /// The key text, for writing and for logging a mismatch.
    fn as_str(&self) -> &str {
        &self.text
    }
}

/// The root as compared: canonicalised when the filesystem can resolve
/// it, so `/tmp/x` and `/private/tmp/x` are one workspace rather than
/// two, and left as given when it cannot.
fn normalised_root(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

/// The configuration's contribution to the key: the resolved path and a
/// digest of its bytes. The digest is what makes an *edited* config
/// invalidate the seed — the path alone would not — and an unreadable
/// or absent file is recorded as such rather than skipped, so "no
/// config" and "a config that has since appeared" are different keys.
fn config_identity(config_path: Option<&Path>) -> String {
    let Some(path) = config_path else {
        return "none".to_owned();
    };
    let digest = match std::fs::read(path) {
        Ok(bytes) => blake3::hash(&bytes).to_hex().to_string(),
        Err(_) => "unreadable".to_owned(),
    };
    format!("{path}#{digest}", path = normalised_root(path).display())
}

/// Writes `key` beside the state file. Best-effort: a key that cannot
/// be written means the next start refuses the seed and runs a cold
/// pass, which is the safe direction.
pub(super) fn write_seed_key(root: &Path, key: &CacheSeedKey) {
    let path = crate::paths::cache_dir(root).join(SEED_KEY_FILE_NAME);
    match std::fs::write(&path, key.as_str()) {
        Ok(()) => tracing::debug!(path = %path.display(), "seed_key_written"),
        Err(error) => tracing::warn!(%error, path = %path.display(), "seed_key_write_failed"),
    }
}

/// True when the key recorded beside the state file is exactly `key`.
/// An absent, unreadable, or different key is a refusal — a seed with
/// no recorded provenance is a seed of unknown provenance.
pub(super) fn seed_key_matches(root: &Path, key: &CacheSeedKey) -> bool {
    let path = crate::paths::cache_dir(root).join(SEED_KEY_FILE_NAME);
    let Ok(recorded) = std::fs::read_to_string(&path) else {
        tracing::info!(path = %path.display(), "seed_key_absent");
        return false;
    };
    if recorded == key.as_str() {
        return true;
    }
    tracing::info!(
        path = %path.display(),
        changed = %first_difference(&recorded, key.as_str()),
        "seed_key_mismatch",
    );
    false
}

/// The first key line that differs, as `recorded -> expected`, so the
/// log says which setting invalidated the seed.
fn first_difference(recorded: &str, expected: &str) -> String {
    recorded
        .lines()
        .zip(expected.lines())
        .find(|(was, now)| was != now)
        .map_or_else(
            || "line count".to_owned(),
            |(was, now)| format!("{was} -> {now}"),
        )
}

/// The key a test uses when the embedding identity is not what it is
/// asserting. Kept beside the real constructor so a test can never
/// drift into building a key by hand that the loader would not accept.
#[cfg(test)]
pub(super) fn test_key(root: &Path, min_nodes: u32, mode: EmbeddingMode) -> CacheSeedKey {
    CacheSeedKey::new(
        root,
        min_nodes,
        true,
        None,
        mode,
        &EmbeddingSpec {
            provider_id: "test".to_owned(),
            model_id: "test-model".to_owned(),
            model_version: "1".to_owned(),
            dimensions: 8,
        },
    )
}

#[cfg(test)]
mod tests;
