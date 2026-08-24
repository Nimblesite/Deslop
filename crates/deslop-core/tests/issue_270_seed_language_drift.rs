//! Regression for GH #270 — the cache-seed language list must be driven
//! by the parser registry, not a hand-maintained extension map.
//!
//! `session-config.languages` (and the MCP `language` enum derived from it)
//! is served from [`AnalysisSession::try_seeded_from_cache`] during the
//! window before the background pipeline installs. That path derived the
//! language set from a hand-written extension→id `match` that drifted from
//! the parser registry — it was never updated when PHP shipped (#265), so a
//! seeded PHP repo reported `languages: []` and the `language` enum dropped
//! `php`, exactly the recurring #170/#198 → fsharp/php class of bug.
//!
//! Black-box against the public `deslop_core::live` surface: build a session
//! over a PHP fixture (which persists `.deslop/cache/live-report.json`), then
//! seed a fresh session from that cache and assert `php` survives.

#![cfg(feature = "live")]

use std::sync::Arc;

use anyhow::{Context, Result};
use deslop_core::{
    embedding::{test_support::StubProvider, EmbeddingMode},
    live::AnalysisSession,
    EmbeddingProvider,
};

use crate::common::copy_fixture;

#[test]
fn seeded_session_reports_php_in_languages() -> Result<()> {
    let tmp = copy_fixture("php-small")?;
    let root = tmp.path().to_path_buf();
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());

    // Cold pass over the PHP fixture persists `.deslop/cache/live-report.json`.
    let _session = AnalysisSession::new(root.clone(), 10, false, None, Arc::clone(&provider))
        .context("cold-pass session")?;

    // Seed a fresh session from that cache — pipeline not yet installed, so
    // `session-config` must fall back to the report's occurrence languages.
    let seeded = AnalysisSession::try_seeded_from_cache(
        root,
        10,
        false,
        None,
        Arc::clone(&provider),
        EmbeddingMode::Off,
    )
    .context("seed session from cache")?;

    let languages = seeded.session_config().languages;
    assert!(
        languages.iter().any(|language| language == "php"),
        "issue #270: a cache-seeded PHP repo must report language `php` in \
         session-config (the enum is derived from it), got {languages:?}"
    );
    Ok(())
}
