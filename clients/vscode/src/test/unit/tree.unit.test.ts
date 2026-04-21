// Unit: tree providers. Drive getChildren() directly against seeded stores.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import {
  TopOffendersProvider,
  FocusedFileProvider,
  SessionProvider,
  StatusTicker,
} from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";

function cluster(id: string, weight: number, path: string): ReportCluster {
  return {
    id,
    weight,
    size: 2,
    canonical_node_count: 4,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [
      { path, start_byte: 0, end_byte: 20, hidden: false },
      { path: `${path}.other`, start_byte: 0, end_byte: 20, hidden: false },
    ],
    summary: "",
    interpretation: `dup in ${path}`,
  };
}

function report(clusters: ReportCluster[]): Report {
  return {
    report_schema_version: 1,
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 5,
    clusters_hidden: 0,
    cache_stats: { hits: 1, misses: 2 },
    metrics: {
      analysed_loc: 100,
      duplicated_loc: 10,
      duplication_percent: 10,
      clusters_total: clusters.length,
      duplicated_files: 2,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "docs",
    action_hints: [],
    embedding_provenance: {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "1",
      dimensions: 768,
    },
    clusters,
  };
}

suite("TopOffendersProvider", () => {
  test("renders an Analysing… placeholder before the first report arrives", () => {
    const store = new ReportStore();
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("renders a 'no duplication' placeholder when the report is empty", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("renders one root node per cluster", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 10, "/f1"), cluster("b", 5, "/f2")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 2);
  });

  test("expanding a cluster node yields OccurrenceNode children", () => {
    const store = new ReportStore();
    const c = cluster("a", 10, "/f1");
    store.setSnapshot(report([c]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const roots = provider.getChildren();
    const kids = provider.getChildren(roots[0]);
    assert.equal(kids.length, c.occurrences.length);
  });

  test("getTreeItem returns the node verbatim", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 10, "/f1")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [root] = provider.getChildren();
    assert.ok(root, "root node must exist");
    assert.strictEqual(provider.getTreeItem(root), root);
  });
});

suite("FocusedFileProvider", () => {
  test("renders 'No active editor' when no editor is focused", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("returns [] when no report is loaded yet", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "x",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 0);
  });

  test("returns cluster overlap for the active editor", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "content",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const activePath = editor.document.uri.fsPath;
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("a", 10, activePath), cluster("b", 5, "/other")]),
      0,
    );
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.ok(nodes.length >= 1);
    const kids = provider.getChildren(nodes[0]);
    assert.ok(kids.length >= 1);
  });

  test("returns an empty hint when no clusters match the active file", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "z",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 1, "/does-not-match")]), 0);
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });
});

suite("SessionProvider", () => {
  test("renders five session rows when a report is loaded", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 1, "/f")]), 0);
    const provider = new SessionProvider(store, new StatusTicker(), () => undefined);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 5);
    assert.equal(provider.getChildren(nodes[0]).length, 0);
  });

  test("renders a 'no session' placeholder before a report arrives", () => {
    const store = new ReportStore();
    const provider = new SessionProvider(store, new StatusTicker(), () => undefined);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("marks state as running when the clientFactory returns a value", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const state = nodes.find((n) => typeof n.label === "string" && n.label === "State");
    assert.ok(state);
  });

  test("SessionProvider renders an Embedding progress row while a swap is in flight", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    store.setEmbeddingProgress({
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 23797,
    });
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const progress = nodes.find(
      (n) => typeof n.label === "string" && n.label === "Embedding",
    );
    assert.ok(progress, "Embedding progress row must be present");
    assert.match(
      String(progress.description ?? ""),
      /0\s*\/\s*23[,.]?797/,
      "progress description must carry done / total",
    );
  });

  test("Embedding model row shows the pending id with a loading suffix while a swap is in flight", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    store.setPendingEmbeddingModel("nomic-embed-text");
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const embeddingRow = nodes.find(
      (n) => typeof n.label === "string" && n.label === "Embedding model",
    );
    assert.ok(embeddingRow, "Embedding model row must be rendered");
    assert.match(
      String(embeddingRow.description ?? ""),
      /nomic-embed-text.*loading/i,
      "pending model id must be visible with a loading hint",
    );
  });
});
