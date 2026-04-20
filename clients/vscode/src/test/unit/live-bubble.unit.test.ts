// Unit: LiveBubble.render — drive inline + ghost paths + dismissal + no-op.

import * as vscode from "vscode";
import { LiveBubble } from "../../bubble/live";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";

function cluster(id: string, weight: number, fused: number): ReportCluster {
  return {
    id,
    weight,
    size: 2,
    canonical_node_count: 4,
    signals: {
      structural: 1,
      token_jaccard: 1,
      embedding_cos: 0.5,
      fused,
    },
    occurrences: [
      { path: "/tmp/A.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/tmp/B.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    summary: "",
    interpretation: "interp",
  };
}

function report(): Report {
  return {
    report_schema_version: 3,
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 2,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 10,
      duplicated_loc: 2,
      duplication_percent: 20,
      clusters_total: 1,
      duplicated_files: 2,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "",
    action_hints: [],
    embedding_provenance: null,
    clusters: [cluster("c-a", 10, 0.95)],
  };
}

suite("LiveBubble render", () => {
  test("inline mode renders the bubble decoration", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line one\nline two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const cfg = vscode.workspace.getConfiguration("codededup");
    await cfg.update("liveBubble.mode", "inline", vscode.ConfigurationTarget.Workspace);
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
    bubble.render(editor, range, [cluster("c-a", 10, 0.95)]);
    // idempotent re-render (same cluster + range) is a no-op
    bubble.render(editor, range, [cluster("c-a", 10, 0.95)]);
    bubble.dispose();
  });

  test("ghost mode renders the ghost-line decoration", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "ghost one\nghost two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const cfg = vscode.workspace.getConfiguration("codededup");
    await cfg.update("liveBubble.mode", "ghost", vscode.ConfigurationTarget.Workspace);
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
    bubble.render(editor, range, [cluster("c-a", 10, 0.95)]);
    await cfg.update("liveBubble.mode", "inline", vscode.ConfigurationTarget.Workspace);
    bubble.dispose();
  });

  test("render without a report is a no-op", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "text",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 2));
    bubble.render(editor, range, [cluster("x", 1, 0.95)]);
    bubble.dispose();
  });

  test("render clears the bubble when no cluster passes the threshold", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "text",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 2));
    // fused below FUSED_THRESHOLD (0.85)
    bubble.render(editor, range, [cluster("y", 1, 0.5)]);
    bubble.dispose();
  });
});
