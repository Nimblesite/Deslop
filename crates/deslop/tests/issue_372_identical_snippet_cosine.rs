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

use crate::mock_ollama::MockOllama;
use anyhow::Result;
use deslop_core::report::PairClassification;

use crate::common::{
    clusters,
    embeddings::run_mock_embedding_report,
    expect_cluster_spanning, occurrence_files,
    signals::{
        assert_no_pair_surface_on_cluster, assert_pair_metric, compare_pair_with_embeddings,
        has_verbatim_pair, occurrence_for_file,
    },
    write_identical_pair,
};

const MIN_NODES: u32 = 10;
const LEFT_FILE: &str = "a.cs";
const RIGHT_FILE: &str = "b.cs";
const EXACT_SCORE: f64 = 1.0;

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

/// [FUSED-EMBED-PROVIDER] Two byte-identical files share one embedding
/// vector, so the rendered cosine is exactly `1.0` — not `0.999998`.
/// Every other embedding cosine in the report stays inside `[0, 1]`.
#[test]
fn byte_identical_clone_pair_renders_embedding_cosine_of_exactly_one() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    write_identical_pair(workspace.path(), "cs", TALLY)?;
    let output = workspace.path().join("report");
    let report = run_mock_embedding_report(
        workspace.path(),
        &output,
        &MIN_NODES.to_string(),
        server.endpoint(),
    )?;

    let cluster = expect_cluster_spanning(&report, &[LEFT_FILE, RIGHT_FILE])?;
    assert_eq!(
        occurrence_files(cluster),
        vec!["a.cs".to_owned(), "b.cs".to_owned()],
        "the identical files must cluster together: {cluster:#}",
    );
    // [PIPELINE-CLUSTER-CLOSURE] The embedding cosine is pair-scoped now —
    // it renders only behind an explicit comparison. The wire fact that
    // pins #372's acceptance is the byte-level one: the identical pair is
    // byte-proven from the source, and no cluster surface carries an
    // embedding figure at all.
    assert!(
        has_verbatim_pair(workspace.path(), cluster)?,
        "a byte-identical file pair must be byte-proven: {cluster:#}",
    );
    let comparison = compare_pair_with_embeddings(
        workspace.path(),
        MIN_NODES,
        occurrence_for_file(cluster, LEFT_FILE)?,
        occurrence_for_file(cluster, RIGHT_FILE)?,
    )?;
    let evidence = &comparison.evidence;
    assert_pair_metric(
        evidence.structural,
        EXACT_SCORE,
        "identical structural overlap",
    );
    assert_pair_metric(
        evidence.token_jaccard,
        EXACT_SCORE,
        "identical token Jaccard",
    );
    assert_pair_metric(
        evidence.embedding_cos,
        EXACT_SCORE,
        "identical embedding cosine",
    );
    assert_pair_metric(
        evidence.agreement,
        EXACT_SCORE,
        "identical content agreement",
    );
    assert_pair_metric(
        evidence.rename_consistency,
        EXACT_SCORE,
        "identity rename mapping",
    );
    assert!(
        !evidence.content_required,
        "an exact embedding independently clears the pair content requirement: {comparison:#?}"
    );
    assert!(
        evidence.content_ok,
        "identical pair content must clear the guard: {comparison:#?}"
    );
    assert!(
        evidence.admitted,
        "identical endpoints must form an admitted edge: {comparison:#?}"
    );
    assert_eq!(evidence.classification, Some(PairClassification::Identical));
    for other in clusters(&report) {
        assert_no_pair_surface_on_cluster(other, "issue #372");
    }
    Ok(())
}
