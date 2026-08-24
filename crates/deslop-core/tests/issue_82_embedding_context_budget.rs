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
    embedding::{EmbeddingMode, DEFAULT_MAX_INPUT_CHARS},
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingProvenance, EmbeddingProvider, EmbeddingSpec, ProviderError, Report,
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

    let report = run_with_provider(root.path(), provider.as_ref())?;

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
    // The coverage identity is in OCCURRENCE units. `indexed_subtrees`
    // is not: [REPORTING-CONTEXT] defines it as the count of *unique*
    // snippets fed into ANN and says outright that it is "lower than
    // `attempted_subtrees` when duplicate snippets collapse before
    // indexing". This assertion previously read
    // `attempted == indexed + failed`, which is that spec's negation —
    // it could only hold while the pass re-indexed one point per
    // occurrence, and it went red the moment GH #357 made the collapse
    // real. The identity a reader actually needs is that no occurrence
    // vanishes silently, so it is stated over `succeeded_subtrees`.
    assert_eq!(
        provenance.attempted_subtrees,
        provenance
            .succeeded_subtrees
            .saturating_add(provenance.failed_subtrees),
        "every attempted occurrence must be accounted for as succeeded or \
         skipped — a subtree that is neither has silently vanished"
    );
    assert!(
        provenance.indexed_subtrees <= provenance.succeeded_subtrees,
        "ANN is fed one point per *distinct* snippet, so index points can \
         never outnumber the occurrences they represent: indexed {} > \
         succeeded {}",
        provenance.indexed_subtrees,
        provenance.succeeded_subtrees
    );
    assert!(
        provenance.succeeded_subtrees > 0,
        "in-budget subtrees must be embedded, not merely attempted"
    );

    Ok(())
}

/// Runs the pipeline over `root` with `provider` in `Required` mode —
/// the shared harness behind every budget test in this file.
fn run_with_provider(root: &Path, provider: &RecordingBudgetProvider) -> Result<Report> {
    run(&PipelineConfig {
        root: root.to_path_buf(),
        min_nodes: 2,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Required,
            provider: Some(provider),
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
    .context("pipeline run")
}

/// GH#286: the per-input character budget is a property of the model
/// behind the provider, but the pipeline applied one hard-coded 6,000
/// to every provider. An F# user lost 14,723 of 175,160 subtrees (8.4%)
/// to it — and because the constant sat upstream of dispatch, the
/// "switch to a 32k-context model" workaround they reasoned their way
/// to would have recovered exactly none of them. A provider that
/// declares a larger budget must have it honoured.
#[test]
fn issue_286_pipeline_honours_the_provider_declared_input_budget() -> Result<()> {
    // Separate roots: both providers report the same spec, so a shared
    // root would let the generous run hit the default run's cache.
    let tight_root = tempfile::tempdir().context("tight tempdir")?;
    let generous_root = tempfile::tempdir().context("generous tempdir")?;
    write_issue_82_fixture(tight_root.path()).context("write tight fixture")?;
    write_issue_82_fixture(generous_root.path()).context("write generous fixture")?;

    let generous = PROVIDER_CONTEXT_CHARS.saturating_mul(4);
    let tight_provider = Arc::new(RecordingBudgetProvider::new());
    let generous_provider = Arc::new(RecordingBudgetProvider::with_budget(generous));

    let tight = run_with_provider(tight_root.path(), tight_provider.as_ref())?;
    let wide = run_with_provider(generous_root.path(), generous_provider.as_ref())?;

    let longest = generous_provider
        .recorded_lengths()?
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
    assert!(
        longest > PROVIDER_CONTEXT_CHARS,
        "provider declaring {generous} chars must receive the oversized subtree, \
         but the longest input dispatched was {longest} — the pipeline is still \
         applying its own {PROVIDER_CONTEXT_CHARS}-char constant"
    );
    assert!(
        longest <= generous,
        "nothing may exceed the declared budget: {longest} > {generous}"
    );

    let tight_provenance = provenance_of(&tight)?;
    let wide_provenance = provenance_of(&wide)?;
    assert!(
        tight_provenance.failed_subtrees > 0,
        "the conservative default must still show the blind spot this test closes"
    );
    assert_eq!(
        wide_provenance.failed_subtrees, 0,
        "no subtree may be dropped when the provider declares it can accept it"
    );
    assert!(
        wide_provenance.indexed_subtrees > tight_provenance.indexed_subtrees,
        "honouring the declared budget must put more of the corpus in the index: \
         {} indexed at {generous} chars vs {} at {PROVIDER_CONTEXT_CHARS}",
        wide_provenance.indexed_subtrees,
        tight_provenance.indexed_subtrees,
    );
    Ok(())
}

/// Borrows a report's embedding provenance or fails with a clear error.
fn provenance_of(report: &Report) -> Result<&EmbeddingProvenance> {
    report
        .embedding_provenance
        .as_ref()
        .ok_or_else(|| anyhow!("expected embedding provenance"))
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
    /// Per-input character budget this provider declares to the
    /// pipeline via [`EmbeddingProvider::max_input_chars`].
    budget: usize,
}

impl RecordingBudgetProvider {
    /// Creates an empty recording provider that declares the same
    /// conservative budget every provider had before the budget became
    /// overridable.
    fn new() -> Self {
        Self::with_budget(DEFAULT_MAX_INPUT_CHARS)
    }

    /// Creates a recording provider that declares `budget` characters
    /// per input — the knob the #286 test turns up.
    fn with_budget(budget: usize) -> Self {
        Self {
            probe_calls: AtomicUsize::new(0),
            batch_calls: AtomicUsize::new(0),
            input_lengths: Mutex::new(Vec::new()),
            budget,
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

    fn max_input_chars(&self) -> usize {
        self.budget
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
