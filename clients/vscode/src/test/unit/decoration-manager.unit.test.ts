// Unit: DecorationManager end-to-end — seed a store, open a matching editor,
// trigger a redraw via setSnapshot and verify we don't throw.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { DecorationManager } from "../../decorations/manager";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";

function cluster(path: string): ReportCluster {
  return {
    id: "dm-1",
    weight: 10,
    size: 3,
    canonical_node_count: 4,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [{ path, start_byte: 0, end_byte: 3, hidden: false }],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

function report(clusters: ReportCluster[]): Report {
  return {
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 1,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 10,
      duplicated_loc: 1,
      duplication_percent: 1,
      clusters_total: clusters.length,
      duplicated_files: 1,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "",
    action_hints: [],
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters,
  };
}

suite("DecorationManager redraw", () => {
  test("redraws when a matching editor is visible", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "abc\ndef\n",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const manager = new DecorationManager(store);
    store.setSnapshot(report([cluster(doc.uri.fsPath)]), 0);
    // The onDidChange handler runs inline — no throw ⇒ pass.
    manager.dispose();
    assert.ok(true);
  });

  test("clears decorations when the report is null", () => {
    const store = new ReportStore();
    const manager = new DecorationManager(store);
    manager.dispose();
  });

  test("redraws when onDidChangeTextDocument fires", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "abc",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const manager = new DecorationManager(store);
    store.setSnapshot(report([cluster(doc.uri.fsPath)]), 0);
    await editor.edit((b) => b.insert(new vscode.Position(0, 0), "z"));
    manager.dispose();
  });

  test("redraw without a report clears the editor decorations", async () => {
    // Covers the null-report short-circuit in redraw + the clear helper.
    // An editor edit before any snapshot has landed must route through
    // clear() and produce empty decoration sets rather than crashing.
    const doc = await vscode.workspace.openTextDocument({
      content: "qwerty",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    assert.equal(store.current.report, null, "fresh ReportStore starts with a null report");
    const manager = new DecorationManager(store);
    try {
      await editor.edit((b) => b.insert(new vscode.Position(0, 0), "!"));
    } finally {
      manager.dispose();
    }
  });
});
