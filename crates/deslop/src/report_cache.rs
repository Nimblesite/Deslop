//! [OUTPUT-REPORT-REUSE] Reuse completed calculations only for identical inputs.

use std::{fs, io::Write as _, path::PathBuf};

use anyhow::{Context, Result};
use blake3::{Hash, Hasher, OUT_LEN};
use deslop_core::{
    discover::discover_files,
    pipeline::{build_extension_map, default_parsers},
    EmbeddingMode, EmbeddingProvider, ParsedDiff, Report,
};

use crate::{load_run_config, Cli};

/// One replaceable entry bounds disk usage independently of the number of scans.
const CACHE_FILE: &str = "completed-report.bin";
/// A cache record begins with its input identity and payload checksum.
const HEADER_HASHES: usize = 2;
/// Byte offset at which the canonical report JSON begins.
const HEADER_BYTES: usize = OUT_LEN.saturating_mul(HEADER_HASHES);
/// Versioned domain separates this entry from other BLAKE3-keyed caches.
const CACHE_DOMAIN: &[u8] = b"deslop-completed-report-v1";

/// An input identity captured before analysis, with no new report wire model.
pub(crate) struct ReportCache {
    /// Workspace-owned cache entry, replaced atomically after a successful scan.
    path: PathBuf,
    /// Content, membership, language, executable, and analysis-setting identity.
    identity: Hash,
}

impl ReportCache {
    /// Discovers the actual corpus and hashes bytes, never timestamps alone.
    pub(crate) fn capture(
        args: &Cli,
        mode: EmbeddingMode,
        provider: Option<&dyn EmbeddingProvider>,
        diff: Option<&ParsedDiff>,
    ) -> Result<Option<Self>> {
        let Some(config) = cache_config(args)? else {
            return Ok(None);
        };
        let root = fs::canonicalize(&args.path).context("resolve report cache root")?;
        let mut identity = settings_identity(args, mode, provider, diff)?;
        hash_part(&mut identity, root.as_os_str().as_encoded_bytes());
        hash_config(&mut identity, config.source_path())?;
        hash_corpus(&mut identity, &root, &config)?;
        Ok(Some(Self {
            path: deslop_core::paths::cache_dir(&root).join(CACHE_FILE),
            identity: identity.finalize(),
        }))
    }

    /// A missing, incompatible, truncated or corrupt entry is a cache miss.
    pub(crate) fn load(&self) -> Option<Report> {
        let bytes = fs::read(&self.path).ok()?;
        let payload = verified_payload(&bytes, self.identity)?;
        match serde_json::from_slice(payload) {
            Ok(report) => {
                tracing::info!("reusing completed report calculations");
                Some(report)
            }
            Err(error) => {
                tracing::warn!(%error, "completed report cache rejected");
                None
            }
        }
    }

    /// Never associates an analysis with inputs that changed while it ran.
    pub(crate) fn store_if_current(
        &self,
        args: &Cli,
        mode: EmbeddingMode,
        provider: Option<&dyn EmbeddingProvider>,
        diff: Option<&ParsedDiff>,
        report: &Report,
    ) {
        match Self::capture(args, mode, provider, diff) {
            Ok(Some(current)) if current.identity == self.identity => {
                if let Err(error) = self.write(report) {
                    tracing::warn!(%error, "completed report cache could not be saved");
                }
            }
            Ok(_) => tracing::info!("inputs changed during analysis; completed report not cached"),
            Err(error) => {
                tracing::warn!(%error, "completed report freshness could not be verified");
            }
        }
    }

    /// Atomic replacement preserves the previous entry if writing fails.
    fn write(&self, report: &Report) -> Result<()> {
        let parent = self.path.parent().context("report cache directory")?;
        fs::create_dir_all(parent).context("create report cache directory")?;
        let payload = serde_json::to_vec(report).context("encode completed report")?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(self.identity.as_bytes())?;
        temporary.write_all(blake3::hash(&payload).as_bytes())?;
        temporary.write_all(&payload)?;
        let _persisted = temporary.persist(&self.path)?;
        tracing::debug!("completed report calculations cached");
        Ok(())
    }
}

/// Rerun commands explicitly request new analysis and need a real mutable session.
fn enabled(args: &Cli) -> bool {
    args.behaviour.incremental()
        && args.rerun_touch.is_empty()
        && args.rerun_remove.is_empty()
        && args.rerun_add.is_empty()
}

/// Length framing prevents boundaries between paths or settings from colliding.
fn hash_part(identity: &mut Hasher, bytes: &[u8]) {
    let _hasher = identity.update(&bytes.len().to_le_bytes()).update(bytes);
}

/// Includes the effective provider revision and executable, even in development builds.
fn settings_identity(
    args: &Cli,
    mode: EmbeddingMode,
    provider: Option<&dyn EmbeddingProvider>,
    diff: Option<&ParsedDiff>,
) -> Result<Hasher> {
    let mut identity = Hasher::new();
    hash_part(&mut identity, CACHE_DOMAIN);
    let executable = std::env::current_exe().context("locate report producer")?;
    let mut executable = fs::File::open(executable).context("read report producer")?;
    let _copied = std::io::copy(&mut executable, &mut identity)?;
    hash_part(&mut identity, &args.min_nodes.to_le_bytes());
    hash_embedding_and_diff(&mut identity, mode, provider, diff);
    hash_environment(&mut identity);
    Ok(identity)
}

/// Config and command opt-outs prevent both reading and writing report entries.
fn cache_config(args: &Cli) -> Result<Option<deslop_core::ExclusionConfig>> {
    if !enabled(args) {
        return Ok(None);
    }
    let config = load_run_config(args)?;
    Ok(config.incremental_enabled().then_some(config))
}

/// Settings are framed separately from source content and output preferences.
fn hash_embedding_and_diff(
    identity: &mut Hasher,
    mode: EmbeddingMode,
    provider: Option<&dyn EmbeddingProvider>,
    diff: Option<&ParsedDiff>,
) {
    let settings = format!(
        "{mode:?}/{:?}/{diff:?}",
        provider.map(EmbeddingProvider::spec)
    );
    hash_part(identity, settings.as_bytes());
}

/// Both the input key and the complete JSON checksum must match before decoding.
fn verified_payload(bytes: &[u8], expected: Hash) -> Option<&[u8]> {
    let identity = bytes.get(..OUT_LEN)?;
    let checksum = bytes.get(OUT_LEN..HEADER_BYTES)?;
    let payload = bytes.get(HEADER_BYTES..)?;
    if identity != expected.as_bytes() || checksum != blake3::hash(payload).as_bytes() {
        tracing::debug!("completed report cache miss");
        return None;
    }
    Some(payload)
}

/// Runtime Deslop policy overrides participate without logging environment values.
fn hash_environment(identity: &mut Hasher) {
    let mut environment: Vec<_> = std::env::vars_os()
        .filter(|(name, _)| name.as_encoded_bytes().starts_with(b"DESLOP_"))
        .collect();
    environment.sort();
    hash_part(identity, format!("{environment:?}").as_bytes());
}

/// Default-discovered and explicit config bytes both invalidate completed calculations.
fn hash_config(identity: &mut Hasher, path: &std::path::Path) -> Result<()> {
    hash_part(identity, path.as_os_str().as_encoded_bytes());
    if !path.as_os_str().is_empty() {
        hash_part(
            identity,
            &fs::read(path).context("read report configuration")?,
        );
    }
    Ok(())
}

/// Shared discovery includes new/deleted/ignored files and contextual Osprey identities.
fn hash_corpus(
    identity: &mut Hasher,
    root: &std::path::Path,
    config: &deslop_core::ExclusionConfig,
) -> Result<()> {
    let parsers = default_parsers();
    let extensions = build_extension_map(&parsers);
    let mut files = discover_files(root, &extensions, config)?.files;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    for file in files {
        hash_part(identity, file.path.as_os_str().as_encoded_bytes());
        hash_part(identity, file.language.as_bytes());
        hash_part(
            identity,
            &fs::read(&file.path).context("read report input")?,
        );
    }
    Ok(())
}
