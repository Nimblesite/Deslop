//! Black-box guard for GH #372: a byte-identical clone pair must render
//! `embedding_cos = 1.0`.
//!
//! `group_snippets_by_content` collapses byte-identical snippets onto a
//! single vector, so the rendered cosine is `cosine(v, v)` — analytically
//! `1.0` for any `v`, and independent of what the embedder returns. That
//! makes it the one embedding figure whose exact value is knowable without
//! trusting the provider, which is why it is asserted here against the
//! rendered report rather than against the arithmetic.
//!
//! This assertion is green both before and after the #372 fix, and is a
//! guard rather than the regression test. `cosine_from_distance` clamps to
//! `[0, 1]`, so accumulated `f32` error is only visible when it happens to
//! round *down*; at the mock's four lanes it rounds up and the clamp
//! absorbs it. Widening the mock does not fix that — the direction stays a
//! rounding coincidence (measured: ~63% of seed offsets expose it at 4096
//! lanes), and a red test built on that coincidence would be calibrated
//! against noise, the defect class of GH #366. The arithmetic itself is
//! pinned by the unit tests in `deslop-core/src/embedding/pairs.rs`.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

mod common;

use anyhow::Result;
use mock_ollama::MockOllama;

use crate::common::{
    cluster_bucket, clusters, corpora::write_identical_pair, embeddings::run_mock_embedding_report,
    expect_cluster_spanning, occurrence_files, signal,
};

/// A non-trivial C# method: a guard clause, an accumulator loop and a
/// return, so the clone is a real code unit rather than a stub.
const TALLY: &str = "namespace Ledger\n\
    {\n\
    \x20   public class Beacon\n\
    \x20   {\n\
    \x20       public int Tally(int bound)\n\
    \x20       {\n\
    \x20           if (bound < 0)\n\
    \x20           {\n\
    \x20               return 0;\n\
    \x20           }\n\
    \x20           int total = 0;\n\
    \x20           for (int step = 0; step < bound; step = step + 1)\n\
    \x20           {\n\
    \x20               total = total + step;\n\
    \x20           }\n\
    \x20           return total;\n\
    \x20       }\n\
    \x20   }\n\
    }\n";

/// [FUSION-EMBED-PROVIDER] Two byte-identical files share one embedding
/// vector, so the rendered cosine is exactly `1.0` — not `0.999998`.
/// Every other embedding cosine in the report stays inside `[0, 1]`.
#[test]
fn byte_identical_clone_pair_renders_embedding_cosine_of_exactly_one() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    write_identical_pair(workspace.path(), "cs", TALLY)?;
    let output = workspace.path().join("report");
    let report = run_mock_embedding_report(workspace.path(), &output, "10", server.endpoint())?;

    let cluster = expect_cluster_spanning(&report, &["a.cs", "b.cs"])?;
    assert_eq!(
        occurrence_files(cluster),
        vec!["a.cs".to_owned(), "b.cs".to_owned()],
        "the identical files must cluster together: {cluster:#}",
    );
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "a byte-identical file pair is an identical clone: {cluster:#}",
    );

    let cosine = signal(cluster, "embedding_cos");
    assert!(
        cosine > 0.0,
        "fixture never reached the embedder, so the cosine proves nothing: {cluster:#}",
    );
    assert!(
        (cosine - 1.0).abs() < f64::EPSILON,
        "byte-identical snippets share one vector, so the rendered cosine must be \
         exactly 1.0, got {cosine:.17}: {cluster:#}",
    );
    let structural = signal(cluster, "structural");
    assert!(
        (structural - 1.0).abs() < f64::EPSILON,
        "an identical clone must be structurally exact, got {structural:.17}: {cluster:#}",
    );
    let jaccard = signal(cluster, "token_jaccard");
    assert!(
        (jaccard - 1.0).abs() < f64::EPSILON,
        "an identical clone must have exact token overlap, got {jaccard:.17}: {cluster:#}",
    );

    for other in clusters(&report) {
        let value = signal(other, "embedding_cos");
        assert!(
            (0.0..=1.0).contains(&value),
            "embedding_cos escaped [0,1]: {other:#}",
        );
    }
    Ok(())
}
