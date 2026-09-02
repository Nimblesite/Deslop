//! Regression test for issue #156 ([LIVE-CLUSTER-OFFSET-FRESHNESS]).
//!
//! Reported bug: `cluster-by-id` (and the `top-offenders` cluster
//! payload) returns `start_byte`/`end_byte` and
//! `start_line`/`end_line` that point at where the duplicate **used
//! to be**, not where it is now. After several edits the offsets
//! drift far enough that reading the file at the reported range
//! surfaces unrelated code.
//!
//! Once the file has been re-analysed, every offset on the cluster
//! returned by `cluster-by-id` must point at code that is still part
//! of the duplicate region.

#![cfg(unix)]

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

use crate::common;
use common::{call_tool, lsp_workspace_with_socket, rescan_call, wait_for_state_then_init_mcp};

/// Issue #156: after rescanning, the cluster payload returned by
/// `cluster-by-id` must contain occurrence byte ranges that map onto
/// the post-edit file. Today the offsets stay at their pre-edit
/// values for at least one MCP cycle.
#[test]
fn issue_156_cluster_by_id_returns_post_edit_offsets() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    // Force a refresh first so the baseline is deterministic.
    let baseline = rescan_call(&mut mcp, &[])?;
    let baseline_clusters = baseline
        .pointer("/page/clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("baseline rescan missing clusters: {baseline}"))?;
    ensure!(
        !baseline_clusters.is_empty(),
        "fixture must produce at least one cluster: {baseline}",
    );

    // Pick the first cluster's id and remember the alpha occurrence
    // bytes for the assertion message later.
    let target_id = baseline_clusters
        .first()
        .and_then(|cluster| cluster.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("baseline cluster missing id: {baseline}"))?
        .to_owned();

    // Insert a long padding block at the top of every fixture file so
    // every byte offset on the surviving cluster must shift forward.
    // Make several rounds of edits before the next rescan — the
    // reported failure mode is that several intervening edits drift
    // the offsets without the LSP ever catching up.
    let names = ["Alpha.cs", "Beta.cs", "Gamma.cs", "Delta.cs"];
    let mut padding = String::new();
    for round in 0..5_u32 {
        padding.push_str(&format!("// padding round {round}\n").repeat(20));
        for name in names {
            let path = workspace.path().join(name);
            let original =
                std::fs::read_to_string(&path).with_context(|| format!("read {name}"))?;
            // strip any prior padding lines so the cumulative
            // shift stays exactly `padding.len()`.
            let body = original
                .lines()
                .skip_while(|line| line.starts_with("// padding round "))
                .collect::<Vec<&str>>()
                .join("\n");
            let shifted = format!("{padding}{body}\n");
            std::fs::write(&path, shifted).with_context(|| format!("rewrite {name}"))?;
        }
    }

    // Do NOT call rescan — issue #156 is that cluster-by-id must
    // serve offsets consistent with the on-disk content even when
    // the agent has not explicitly forced a refresh between read
    // calls. Either re-resolve offsets at read time or invalidate
    // stale clusters; either fix keeps the agent from being misled.
    let cluster = call_tool(&mut mcp, "cluster-by-id", &json!({ "id": target_id }))?;
    let occurrences = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("cluster-by-id payload missing occurrences: {cluster}"))?;
    ensure!(
        !occurrences.is_empty(),
        "cluster {target_id} returned zero occurrences: {cluster}",
    );

    let padding_bytes = padding.len();
    for occurrence in occurrences {
        let path = occurrence
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let start_byte = occurrence
            .get("start_byte")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let end_byte = occurrence
            .get("end_byte")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let start_usize = usize::try_from(start_byte)
            .with_context(|| format!("start_byte overflow at {path}"))?;
        let end_usize =
            usize::try_from(end_byte).with_context(|| format!("end_byte overflow at {path}"))?;
        ensure!(
            start_usize >= padding_bytes,
            "issue #156: occurrence at {path} reports stale start_byte={start_byte}; padding shifted every cluster by {padding_bytes} bytes. Full cluster: {cluster}",
        );
        // Read the file and assert the reported byte range still
        // lives inside the file — a stale end_byte often overshoots
        // the new file length and would corrupt agent context.
        let on_disk_path = workspace.path().join(path);
        let body = std::fs::read(&on_disk_path).with_context(|| format!("read on-disk {path}"))?;
        ensure!(
            end_usize <= body.len(),
            "issue #156: occurrence {path} end_byte={end_byte} overruns the post-edit file length {len}: {cluster}",
            len = body.len(),
        );
    }
    Ok(())
}
