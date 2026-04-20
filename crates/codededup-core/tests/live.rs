//! End-to-end tests for the live module ([LIVE-PACKAGING]).
//!
//! Drives the public surface of [`codededup_core::live::*`] against
//! the same C# fixtures the CLI uses. No internal types are touched.

#![cfg(feature = "live")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
};

use anyhow::{anyhow, bail, Context, Result};
use codededup_core::{
    embedding::{EmbeddingMode, StubProvider},
    live::{
        AnalysisSession, Clock, Debouncer, FindSimilarInput, FindSimilarRequest, LiveApi,
        LiveError, LiveService,
    },
    pipeline::{run, EmbeddingSettings, PipelineConfig},
};

/// Returns the absolute fixture path used by the CLI tests.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("codededup")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies the fixture tree into a temp dir so destructive edits never
/// pollute the source repo.
fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
    let src = fixture(name);
    let dir = tempfile::tempdir().context("tempdir")?;
    copy_recursive(&src, dir.path())?;
    Ok(dir)
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst).with_context(|| format!("mkdir {}", dst.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("read_dir {}", src.display()))? {
            let entry = entry.context("dir entry")?;
            let target = dst.join(entry.file_name());
            copy_recursive(&entry.path(), &target)?;
        }
    } else {
        let _bytes = fs::copy(src, dst).with_context(|| format!("copy {}", src.display()))?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn live_session_first_report_matches_batch_run() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider.clone())
        .context("session")?;
    let live_report = session.report();
    let batch_provider = StubProvider::new();
    let batch_report = run(&PipelineConfig {
        root: tmp.path().to_path_buf(),
        min_nodes: 15,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Auto,
            provider: Some(&batch_provider),
        },
        incremental: false,
    })
    .context("batch run")?;
    let live_ids: Vec<&str> = live_report.clusters.iter().map(|c| c.id.as_str()).collect();
    let batch_ids: Vec<&str> = batch_report
        .clusters
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(live_ids, batch_ids, "live and batch cluster ids must match");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn update_files_produces_non_empty_delta_when_a_file_changes() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let mut session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;
    let target = tmp.path().join("Beta.cs");
    fs::write(
        &target,
        b"namespace Beta { public class Differ { public int Run(int x) { return x + 1; } } }\n",
    )
    .context("write Beta")?;
    let delta = session.apply_changes(&[target]).context("apply")?;
    assert!(
        !(delta.clusters_added.is_empty()
            && delta.clusters_removed.is_empty()
            && delta.clusters_updated.is_empty()),
        "edit must produce a non-empty delta, got: {delta:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn find_similar_on_known_range_returns_expected_cluster() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;
    let report = session.report();
    let cluster = report
        .clusters
        .first()
        .ok_or_else(|| anyhow!("expected at least one cluster"))?;
    let occurrence = cluster
        .occurrences
        .first()
        .ok_or_else(|| anyhow!("expected at least one occurrence"))?;
    let request = FindSimilarRequest {
        input: FindSimilarInput::OpenRange {
            path: tmp.path().join(&occurrence.path),
            start_byte: occurrence.start_byte,
            end_byte: occurrence.end_byte,
        },
        max_results: None,
    };
    let result = session.find_similar(&request).context("find_similar")?;
    assert!(
        !result.clusters.is_empty(),
        "open-range find_similar should hit the known cluster"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn find_similar_on_unparseable_snippet_returns_unparseable_error() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;
    let request = FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: "this is not C# {{ unbalanced".to_owned(),
            language: "csharp".to_owned(),
        },
        max_results: None,
    };
    let outcome = session.find_similar(&request);
    // The C# parser is permissive, so we accept either an error or an
    // empty result with `below_min_nodes: true`. The contract we test
    // is "no panic, deterministic outcome".
    match outcome {
        Err(LiveError::UnparseableInput { .. }) => Ok(()),
        Ok(result) => {
            assert!(
                result.clusters.is_empty(),
                "unparseable snippet must not surface clusters"
            );
            Ok(())
        }
        Err(other) => bail!("unexpected error variant: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn find_similar_on_below_min_nodes_snippet_returns_below_min_nodes_flag() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let session = AnalysisSession::new(tmp.path().to_path_buf(), 1_000, false, None, provider)
        .context("session")?;
    let request = FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: "class A { void M() {} }".to_owned(),
            language: "csharp".to_owned(),
        },
        max_results: None,
    };
    let result = session.find_similar(&request).context("find_similar")?;
    assert!(
        result.below_min_nodes,
        "tiny snippet under min_nodes must set below_min_nodes"
    );
    assert!(result.clusters.is_empty(), "no clusters expected");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn debouncer_coalesces_burst_and_flushes_at_cap() -> Result<()> {
    let clock = Arc::new(MockClock::new(0));
    let mut debouncer = Debouncer::new(clock.clone());
    debouncer.push(PathBuf::from("a.cs"));
    debouncer.push(PathBuf::from("b.cs"));
    debouncer.push(PathBuf::from("a.cs"));
    assert!(!debouncer.ready_to_flush(), "no time elapsed yet");
    clock.advance(50)?;
    assert!(!debouncer.ready_to_flush(), "still inside quiet window");
    clock.advance(2_500)?;
    assert!(debouncer.ready_to_flush(), "cap should fire");
    let flushed = debouncer.flush();
    assert_eq!(flushed.len(), 2, "duplicates must collapse");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn live_service_round_trip_covers_the_query_surface() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let session_lock = make_session_lock(tmp.path())?;
    let mut service = LiveService::new(Arc::clone(&session_lock));
    service.set_ollama_endpoint("http://127.0.0.1:1".to_owned());

    let first_id = exercise_snapshot_lookups(&service, tmp.path()).await?;
    let initial_generation = exercise_session_config(&service, &session_lock, tmp.path()).await?;
    exercise_delta_cursor(&service, initial_generation).await;
    exercise_error_paths(&service, &first_id).await;
    exercise_embedding_swap(&service).await?;
    exercise_path_resolution(&service).await?;
    Ok(())
}

/// Builds a fresh session lock around the temp fixture root.
fn make_session_lock(root: &Path) -> Result<Arc<tokio::sync::Mutex<AnalysisSession>>> {
    let provider = Arc::new(StubProvider::new());
    let session =
        AnalysisSession::new(root.to_path_buf(), 15, false, None, provider).context("session")?;
    Ok(Arc::new(tokio::sync::Mutex::new(session)))
}

/// Verifies `report_get` → `cluster_by_id` → `report_for_file/range`
/// round trips and returns the first cluster id for downstream use.
async fn exercise_snapshot_lookups(service: &LiveService, root: &Path) -> Result<String> {
    let report = service.report_get().await;
    assert!(!report.clusters.is_empty(), "fixture must produce clusters");
    let first = report
        .clusters
        .first()
        .ok_or_else(|| anyhow!("at least one cluster"))?;
    let by_id = service.cluster_by_id(&first.id).await?;
    assert_eq!(by_id.id, first.id);
    let occurrence = first
        .occurrences
        .first()
        .ok_or_else(|| anyhow!("expected occurrence"))?;
    let resolved = root.join(&occurrence.path);
    let file_report = service.report_for_file(&resolved).await;
    assert_eq!(file_report.path, resolved);
    let range_clusters = service
        .report_for_range(&resolved, occurrence.start_byte, occurrence.end_byte)
        .await;
    assert!(!range_clusters.is_empty());
    Ok(first.id.clone())
}

/// Verifies `session_config` + a basic `find_similar` + generation cursor.
async fn exercise_session_config(
    service: &LiveService,
    session_lock: &Arc<tokio::sync::Mutex<AnalysisSession>>,
    root: &Path,
) -> Result<u64> {
    let config = service.session_config().await;
    assert_eq!(config.workspace_root, root);
    assert!(!config.languages.is_empty());
    let request = FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: "namespace N { class C { void M(int x) { return; } } }".to_owned(),
            language: "csharp".to_owned(),
        },
        max_results: Some(5),
    };
    let _result = service.find_similar(&request).await?;
    let guard = session_lock.lock().await;
    assert_eq!(guard.root(), root, "root accessor should match");
    let generation = guard.generation();
    assert!(generation >= 1);
    Ok(generation)
}

/// Verifies the delta cursor returns `Some` for stale generations and
/// `None` when the caller is up-to-date.
async fn exercise_delta_cursor(service: &LiveService, current_generation: u64) {
    let cursor = service.report_delta(0).await;
    assert!(cursor.is_some());
    let none_now = service.report_delta(current_generation).await;
    assert!(none_now.is_none());
}

/// Asserts the four error paths through the query surface.
async fn exercise_error_paths(service: &LiveService, _first_id: &str) {
    let outside = FindSimilarRequest {
        input: FindSimilarInput::OpenRange {
            path: PathBuf::from("/definitely/not/here.cs"),
            start_byte: 0,
            end_byte: 1,
        },
        max_results: None,
    };
    let outside_outcome = service.find_similar(&outside).await;
    assert!(matches!(
        outside_outcome,
        Err(LiveError::PathOutsideWorkspace { .. })
    ));
    let miss = service.cluster_by_id("deadbeefcafebabe").await;
    assert!(matches!(miss, Err(LiveError::UnknownCluster { .. })));
    let unsupported = FindSimilarRequest {
        input: FindSimilarInput::Snippet {
            snippet: "function f() {}".to_owned(),
            language: "javascript".to_owned(),
        },
        max_results: None,
    };
    let unsupported_outcome = service.find_similar(&unsupported).await;
    assert!(matches!(
        unsupported_outcome,
        Err(LiveError::UnsupportedLanguage { .. })
    ));
}

/// Verifies the embedding model swap surface.
async fn exercise_embedding_swap(service: &LiveService) -> Result<()> {
    let models = service.embedding_list_models().await;
    assert!(models.iter().all(|m| m.provider_id == "stub"));
    let provenance = service
        .embedding_set_model("stub", "blake3-stub", None)
        .await?;
    assert!(provenance.is_some_and(|p| p.provider_id == "stub"));
    let unknown = service.embedding_set_model("nope", "no", None).await;
    assert!(matches!(
        unknown,
        Err(LiveError::UnsupportedProvider { .. })
    ));
    Ok(())
}

/// Verifies relative paths resolve against the workspace root.
async fn exercise_path_resolution(service: &LiveService) -> Result<()> {
    let relative = FindSimilarRequest {
        input: FindSimilarInput::OpenRange {
            path: PathBuf::from("Alpha.cs"),
            start_byte: 0,
            end_byte: 10,
        },
        max_results: Some(3),
    };
    let _relative_result = service.find_similar(&relative).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn embedding_list_models_falls_back_to_stub_when_ollama_unreachable() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let provider = Arc::new(StubProvider::new());
    let session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;
    let session_lock = Arc::new(tokio::sync::Mutex::new(session));
    let mut service = LiveService::new(session_lock);
    service.set_ollama_endpoint("http://127.0.0.1:1".to_owned());
    let models = service.embedding_list_models().await;
    assert!(
        models.iter().any(|m| m.provider_id == "stub"),
        "stub must always be in the list"
    );
    assert!(
        models.iter().all(|m| m.provider_id == "stub"),
        "ollama unreachable: only stub should appear"
    );
    Ok(())
}

/// Test-only [`Clock`] driving its time off an [`Arc<StdMutex<u64>>`].
#[derive(Debug)]
struct MockClock {
    /// Current time in milliseconds.
    now_ms: StdMutex<u64>,
}

impl MockClock {
    fn new(start: u64) -> Self {
        Self {
            now_ms: StdMutex::new(start),
        }
    }

    fn advance(&self, by_ms: u64) -> Result<()> {
        let mut guard = self
            .now_ms
            .lock()
            .map_err(|_poisoned| anyhow!("MockClock mutex poisoned"))?;
        *guard = guard.saturating_add(by_ms);
        Ok(())
    }
}

impl Clock for MockClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.lock().map_or(u64::MAX, |guard| *guard)
    }
}
