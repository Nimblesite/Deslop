//! Regression coverage for GH#82: oversized subtrees must be rejected
//! by the pipeline before an embedding provider sees them.

use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    embedding::EmbeddingMode,
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingProvider, EmbeddingSpec, ProviderError,
};

/// Hard context budget used by the issue fixture provider.
const PROVIDER_CONTEXT_CHARS: usize = 6_000;

/// Embedding dimensionality returned by the issue fixture provider.
const VECTOR_DIMS: usize = 3;

#[test]
fn issue_82_embedding_pass_skips_oversized_subtrees_before_provider_dispatch() -> Result<()> {
    let root = tempfile::tempdir().context("tempdir")?;
    write_issue_82_fixture(root.path()).context("write fixture")?;
    let provider = Arc::new(RecordingBudgetProvider::new());

    let report = run(&PipelineConfig {
        root: root.path().to_path_buf(),
        min_nodes: 2,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Required,
            provider: Some(provider.as_ref()),
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
    .context("pipeline run")?;

    let lengths = provider.recorded_lengths()?;
    let oversized_lengths: Vec<usize> = lengths
        .iter()
        .copied()
        .filter(|length| *length > PROVIDER_CONTEXT_CHARS)
        .collect();
    let provenance = report
        .embedding_provenance
        .as_ref()
        .ok_or_else(|| anyhow!("expected embedding provenance"))?;

    assert_eq!(provider.probe_calls(), 1, "provider should be probed once");
    assert!(
        provider.batch_calls() > 0,
        "small subtrees must still embed"
    );
    assert!(
        !lengths.is_empty(),
        "provider should receive at least one in-budget subtree"
    );
    assert!(
        lengths
            .iter()
            .all(|length| *length <= PROVIDER_CONTEXT_CHARS),
        "provider received oversized inputs: {oversized_lengths:?}"
    );
    assert!(
        provenance.indexed_subtrees > 0,
        "in-budget subtrees should still be indexed"
    );
    assert!(
        provenance.failed_subtrees > 0,
        "oversized subtrees should be counted as skipped failures"
    );
    assert_eq!(
        provenance.attempted_subtrees,
        provenance
            .indexed_subtrees
            .saturating_add(provenance.failed_subtrees),
        "attempted subtrees should equal indexed plus skipped"
    );

    Ok(())
}

/// Writes one very large C# method plus a small file so the test proves
/// oversized snippets are skipped without disabling the whole pass.
fn write_issue_82_fixture(root: &Path) -> Result<()> {
    fs::write(
        root.join("Huge.cs"),
        large_csharp_source(PROVIDER_CONTEXT_CHARS.saturating_add(1_000)),
    )
    .context("write Huge.cs")?;
    fs::write(root.join("Small.cs"), small_csharp_source()).context("write Small.cs")?;
    Ok(())
}

/// Builds a valid C# file with a method body longer than `minimum_chars`.
fn large_csharp_source(minimum_chars: usize) -> String {
    let mut statements = String::new();
    while statements.chars().count() < minimum_chars {
        statements.push_str("            total = total + 1;\n");
    }
    format!(
        "namespace Issue82 {{ public class Huge {{ public int Run(int seed) {{ var total = seed;\n{statements}            return total; }} }} }}\n"
    )
}

/// Builds a small C# file that should remain eligible for embedding.
fn small_csharp_source() -> &'static str {
    "namespace Issue82 { public class Small { public int Run(int seed) { return seed + 1; } } }\n"
}

/// Provider that records every input length the pipeline dispatches.
#[derive(Debug)]
struct RecordingBudgetProvider {
    /// Number of reachability probes performed by the pipeline.
    probe_calls: AtomicUsize,
    /// Number of batch embedding calls performed by the pipeline.
    batch_calls: AtomicUsize,
    /// Character lengths received by `embed_batch`.
    input_lengths: Mutex<Vec<usize>>,
}

impl RecordingBudgetProvider {
    /// Creates an empty recording provider.
    fn new() -> Self {
        Self {
            probe_calls: AtomicUsize::new(0),
            batch_calls: AtomicUsize::new(0),
            input_lengths: Mutex::new(Vec::new()),
        }
    }

    /// Returns how many times `probe` was called.
    fn probe_calls(&self) -> usize {
        self.probe_calls.load(Ordering::SeqCst)
    }

    /// Returns how many times `embed_batch` was called.
    fn batch_calls(&self) -> usize {
        self.batch_calls.load(Ordering::SeqCst)
    }

    /// Returns a copy of the input lengths recorded so far.
    fn recorded_lengths(&self) -> Result<Vec<usize>> {
        self.input_lengths
            .lock()
            .map(|lengths| lengths.clone())
            .map_err(|_| anyhow!("recorded length lock poisoned"))
    }
}

impl EmbeddingProvider for RecordingBudgetProvider {
    fn spec(&self) -> EmbeddingSpec {
        EmbeddingSpec {
            provider_id: "issue-82".to_owned(),
            model_id: "budget-fixture".to_owned(),
            model_version: "test".to_owned(),
            dimensions: VECTOR_DIMS,
        }
    }

    fn probe(&self) -> Result<(), ProviderError> {
        let _previous = self.probe_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError> {
        self.embed_batch(&[input.to_owned()])
            .map(|mut vectors| vectors.pop().unwrap_or_default())
    }

    fn max_batch_size(&self) -> usize {
        16
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        let _previous = self.batch_calls.fetch_add(1, Ordering::SeqCst);
        let mut lengths = self
            .input_lengths
            .lock()
            .map_err(|_| ProviderError::Malformed {
                provider_id: "issue-82".to_owned(),
                message: "recorded length lock poisoned".to_owned(),
            })?;
        lengths.extend(inputs.iter().map(|input| input.chars().count()));
        Ok(inputs.iter().map(|_input| vec![1.0, 0.0, 0.0]).collect())
    }
}
