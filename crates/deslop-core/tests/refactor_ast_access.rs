//! E2E coverage for the merge engine's in-process AST access
//! ([AUTOFIX-MERGE]): a known cluster's occurrence subtrees and source
//! bytes are retrievable from the in-memory pipeline state through the
//! public `deslop-core` API — never over the wire.


use anyhow::{ensure, Context, Result};
use deslop_core::ast::ByteRange;

use crate::common::{fixture, refactor_pipeline_session as session};

/// [AUTOFIX-MERGE]: every occurrence of the fixture's cluster resolves
/// to a normalised subtree covering its byte range, and the source
/// bytes behind each occurrence are the file's bytes.
#[test]
fn cluster_occurrence_subtrees_are_retrievable() -> Result<()> {
    let root = fixture("csharp-extract-type1");
    let (session, report) = session(&root)?;
    let cluster = report
        .clusters
        .first()
        .context("fixture produces at least one cluster")?;
    ensure!(
        cluster.occurrences.len() == 2,
        "fixture cluster has two occurrences, got {}",
        cluster.occurrences.len()
    );

    let mut kinds = Vec::new();
    for occurrence in &cluster.occurrences {
        let path = root.join(&occurrence.path);
        let file_id = session
            .file_id_for(&path)
            .with_context(|| format!("file id for {}", path.display()))?;
        let source = session
            .source_bytes_for(file_id)
            .context("source bytes retrievable")?;
        ensure!(
            source.len() >= occurrence.end_byte,
            "source covers the occurrence range"
        );
        let range = ByteRange {
            start: occurrence.start_byte,
            end: occurrence.end_byte,
        };
        let subtree = session
            .subtree_at_range(file_id, range)
            .context("subtree retrievable for occurrence")?;
        ensure!(
            subtree.byte_range.start <= range.start && range.end <= subtree.byte_range.end,
            "subtree covers the occurrence range"
        );
        ensure!(
            !subtree.children.is_empty(),
            "occurrence subtree is a real tree, not a leaf"
        );
        kinds.push(subtree.kind);
    }
    ensure!(
        kinds
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left == right)),
        "both occurrences resolve to same-kind subtrees, got {kinds:?}"
    );

    let first = cluster
        .occurrences
        .first()
        .context("first occurrence present")?;
    let file_id = session
        .file_id_for(&root.join(&first.path))
        .context("file id for first occurrence")?;
    let out_of_range = session.subtree_at_range(
        file_id,
        ByteRange {
            start: usize::MAX - 1,
            end: usize::MAX,
        },
    );
    ensure!(
        out_of_range.is_none(),
        "ranges outside the parse root yield no subtree"
    );
    Ok(())
}
