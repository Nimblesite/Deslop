//! Regression test for issue #153 ([LIVE-RESCAN-FRESHNESS]).
//!
//! Reported bug: after editing files to eliminate a cluster,
//! `mcp__deslop__rescan` frequently returns the same cluster id and
//! pre-edit byte offsets. A second `rescan` call eventually yields
//! fresh data.
//!
//! The contract of `rescan` is: block until a fresh scan completes,
//! then return *that* scan's report — never one from before the
//! refresh. The MCP `rescan` payload contains both `generation` and
//! `clusters`, and both must come from the same generation.

#![cfg(unix)]

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::Value;

use crate::common;
use common::{lsp_workspace_with_socket, rescan_call, wait_for_state_then_init_mcp};

/// One unique C# file body that shares no normalised subtrees with
/// the rest of the corpus, so writing it eliminates any cluster the
/// original file belonged to.
fn unique_body(seed: u32) -> String {
    format!(
        "namespace Unique{seed} {{ public class Once{seed} {{ public string Tag() => \"{seed}\"; }} }}\n",
    )
}

/// Issue #153: a single `rescan` call must return clusters from the
/// post-refresh generation. After overwriting every fixture file with
/// unique content, the response must show zero clusters in the same
/// payload that reports the post-refresh generation.
#[test]
fn issue_153_single_rescan_reflects_post_edit_state() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    // Baseline must have at least one cluster — otherwise the test
    // cannot prove the rescan is doing real work.
    let baseline = rescan_call(&mut mcp, &[])?;
    let baseline_clusters = clusters_array(&baseline)?;
    ensure!(
        !baseline_clusters.is_empty(),
        "fixture must produce at least one cluster before edits; baseline={baseline}",
    );

    // Eliminate every cluster: overwrite every fixture file with
    // body that shares no normalised subtrees.
    let names = ["Alpha.cs", "Beta.cs", "Gamma.cs", "Delta.cs"];
    let mut paths = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let file = workspace.path().join(name);
        let seed = u32::try_from(index).unwrap_or(0);
        std::fs::write(&file, unique_body(seed)).with_context(|| format!("overwrite {name}"))?;
        paths.push(file.to_string_lossy().into_owned());
    }

    let rescan = rescan_call(&mut mcp, &paths)?;
    let after_clusters = clusters_array(&rescan)?;
    let after_generation = rescan
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("rescan response missing generation field: {rescan}"))?;
    ensure!(
        after_generation > 0,
        "rescan generation must advance past zero: {rescan}",
    );
    ensure!(
        after_clusters.is_empty(),
        "issue #153: a single `rescan` call must surface the post-edit state. \
         Got {count} stale cluster(s) at generation {after_generation}: {rescan}",
        count = after_clusters.len(),
    );
    Ok(())
}

/// Issue #153 companion: a single `rescan` call must surface the
/// post-edit byte/line ranges for surviving clusters. When a comment
/// block is inserted in front of Alpha.cs the cluster id is stable
/// (content hash), but every byte offset for the Alpha occurrence
/// must shift forward by the padding length. Today the first rescan
/// returns the pre-edit offsets and only a second call eventually
/// catches up.
#[test]
fn issue_153_rescan_occurrence_offsets_reflect_post_edit_file() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    // Insert a long leading comment block into Alpha.cs so the
    // surviving Alpha occurrence shifts down by many bytes. The
    // cluster id is stable (content hash) but every byte offset for
    // the Alpha occurrence must move forward.
    let alpha = workspace.path().join("Alpha.cs");
    let original_alpha = std::fs::read_to_string(&alpha)?;
    let inserted_prefix = "// padding line\n".repeat(40);
    let new_alpha = format!("{inserted_prefix}{original_alpha}");
    std::fs::write(&alpha, &new_alpha)?;

    // Call rescan without listing the path explicitly — the MCP wire
    // contract is that rescan does a full refresh regardless of args
    // (the `paths` argument is informational). The first call must
    // already reflect the post-edit Alpha offsets, never the second.
    let rescan = rescan_call(&mut mcp, &[])?;
    let after_clusters = clusters_array(&rescan)?;
    ensure!(
        !after_clusters.is_empty(),
        "Alpha/Beta cluster must survive a comment-only edit: {rescan}",
    );

    // Find the Alpha occurrence in the first surviving cluster and
    // assert its byte/line range matches the post-edit file. The
    // surviving Alpha occurrence must start AFTER the inserted
    // padding bytes — otherwise the offsets are stale.
    let padding_bytes = inserted_prefix.len();
    let mut found_alpha = false;
    for cluster in &after_clusters {
        let Some(occurrences) = cluster.get("occurrences").and_then(Value::as_array) else {
            continue;
        };
        for occurrence in occurrences {
            let path = occurrence
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !path.ends_with("Alpha.cs") {
                continue;
            }
            found_alpha = true;
            let start_byte = occurrence
                .get("start_byte")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let start_usize = usize::try_from(start_byte)
                .with_context(|| format!("start_byte overflow at {path}"))?;
            ensure!(
                start_usize >= padding_bytes,
                "issue #153: surviving Alpha occurrence reports stale start_byte={start_byte} but padding shifted the cluster by {padding_bytes} bytes. Full cluster: {cluster}",
            );
        }
    }
    ensure!(
        found_alpha,
        "Alpha.cs must appear in at least one cluster after edit: {rescan}",
    );
    Ok(())
}

/// Extracts the `clusters` JSON array from a rescan payload.
fn clusters_array(structured: &Value) -> Result<Vec<Value>> {
    structured
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow!("rescan payload missing clusters array: {structured}"))
}
