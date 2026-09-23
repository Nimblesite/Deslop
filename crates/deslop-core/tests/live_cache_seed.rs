//! [LIVE-CACHE-SEED] What a session may say while it is still answering from
//! the last run's report and its own pass has not finished.
//!
//! A snippet `find-similar` is matched against the fingerprints of a
//! completed pass, and a seed carries none. Before the pass installs, the
//! only honest answer to a snippet query is "not ready yet": an empty
//! cluster list reads as "nothing similar here", which is exactly the
//! answer an agent takes as permission to write the copy (#425).

#![cfg(feature = "live")]

use std::{fs, path::Path, sync::Arc};

use anyhow::{anyhow, bail, ensure, Context, Result};
use deslop_core::{
    embedding::{test_support::StubProvider, EmbeddingMode},
    live::{AnalysisSession, FindSimilarInput, FindSimilarRequest, LiveError},
    report::ReportOccurrence,
    EmbeddingProvider,
};

use crate::common::{copy_fixture, DEFAULT_LIVE_MIN_NODES};

/// A fixture with a known cross-file clone.
const FIXTURE: &str = "csharp-small";
/// The fixture's language id.
const LANGUAGE: &str = "csharp";

/// A snippet query for `snippet` in the fixture's language.
fn snippet_request(snippet: &str) -> FindSimilarRequest {
    FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: snippet.to_owned(),
            language: LANGUAGE.to_owned(),
        },
        max_results: None,
    }
}

/// A range query over `occurrence` under `root`.
fn range_request(root: &Path, occurrence: &ReportOccurrence) -> FindSimilarRequest {
    FindSimilarRequest {
        input: FindSimilarInput::OpenRange {
            path: root.join(&occurrence.path),
            start_byte: occurrence.start_byte,
            end_byte: occurrence.end_byte,
        },
        max_results: None,
    }
}

/// The exact source text of `occurrence` — a verbatim copy of clustered code.
fn occurrence_text(root: &Path, occurrence: &ReportOccurrence) -> Result<String> {
    let source = fs::read(root.join(&occurrence.path)).context("read occurrence file")?;
    let bytes = source
        .get(occurrence.start_byte..occurrence.end_byte)
        .ok_or_else(|| anyhow!("occurrence range outside its file"))?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

#[test]
fn a_snippet_query_in_the_seed_window_says_not_ready_never_nothing_similar() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let root = tmp.path().to_path_buf();
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
    let warm = AnalysisSession::new(
        root.clone(),
        DEFAULT_LIVE_MIN_NODES,
        false,
        None,
        Arc::clone(&provider),
    )
    .context("warm session")?;
    warm.persist_seed_cache();
    let report = warm.report();
    let occurrence = report
        .clusters
        .first()
        .and_then(|cluster| cluster.occurrences.first())
        .ok_or_else(|| anyhow!("the fixture must cluster"))?
        .clone();
    let snippet = occurrence_text(&root, &occurrence)?;

    // Control: once a pass has run, the verbatim snippet finds its cluster.
    let ready = warm.find_similar(&snippet_request(&snippet))?;
    ensure!(
        !ready.clusters.is_empty() && !ready.below_min_nodes,
        "control: a completed pass must find a verbatim copy of clustered code"
    );
    drop(warm);

    let seeded = AnalysisSession::try_seeded_from_cache(
        root.clone(),
        DEFAULT_LIVE_MIN_NODES,
        false,
        None,
        provider,
        EmbeddingMode::Off,
    )
    .context("the warm run's seed must be served")?;
    ensure!(seeded.is_seed_only(), "the seeded session has no pass yet");
    match seeded.find_similar(&snippet_request(&snippet)) {
        Err(LiveError::AnalysisNotReady) => {}
        Ok(answer) => bail!(
            "the seed window answered a snippet query as if it had looked: {} clusters, \
             below_min_nodes = {} — indistinguishable from \"nothing similar\"",
            answer.clusters.len(),
            answer.below_min_nodes
        ),
        Err(other) => bail!("expected AnalysisNotReady, got {other:?}"),
    }

    // The seed still answers range queries ([LIVE-CACHE-SEED]).
    let from_seed = seeded.find_similar(&range_request(&root, &occurrence))?;
    ensure!(
        !from_seed.clusters.is_empty(),
        "a range query in the seed window answers from the seed report"
    );
    Ok(())
}
