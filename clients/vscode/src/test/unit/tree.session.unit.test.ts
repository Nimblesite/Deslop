// Unit: SessionProvider. Drives getChildren() against a seeded store.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, report, sessionPanel, treeStore } from "./tree.helpers";

const STRING_TYPE_NAME = "string";
const EMBEDDING_MODEL_LABEL = "Embedding model";
const SESSION_ROW_COUNT = 4;

/** No LSP client resolved — the panel reports a stopped session. */
const NO_CLIENT = (): LanguageClient | undefined => undefined;
/** A resolved client: the panel only checks presence, never its methods. */
const RUNNING_CLIENT = (): LanguageClient => ({}) as never;

/** The one session row carrying `label`, or undefined when absent. */
function rowNamed(nodes: vscode.TreeItem[], label: string): vscode.TreeItem | undefined {
  return nodes.find((node) => typeof node.label === STRING_TYPE_NAME && node.label === label);
}

suite("SessionProvider", () => {
  test("renders four session rows when a report is loaded", () => {
    const provider = sessionPanel(treeStore([cluster("a", 1, "/f")]), NO_CLIENT);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, SESSION_ROW_COUNT);
    assert.equal(provider.getChildren(nodes[0]).length, 0);
  });

  test("omits internal report format fields from the human session panel (#118)", () => {
    const nodes = sessionPanel(treeStore([cluster("a", 1, "/f")]), NO_CLIENT).getChildren();
    const labels = nodes.map(labelText);
    assert.deepEqual(labels, [EMBEDDING_MODEL_LABEL, "Cache", "Files analysed", "State"]);
    assert.equal(nodes.length, SESSION_ROW_COUNT);
    assert.ok(labels.includes(EMBEDDING_MODEL_LABEL));
    assert.ok(labels.includes("Files analysed"));
    assert.ok(labels.includes("State"));
    assert.ok(!labels.some((label) => /schema/i.test(label)));
  });

  test("renders a 'no session' placeholder before a report arrives", () => {
    const nodes = sessionPanel(new ReportStore(), NO_CLIENT).getChildren();
    assert.equal(nodes.length, 1);
  });

  test("marks state as running when the clientFactory returns a value", () => {
    const nodes = sessionPanel(treeStore(), RUNNING_CLIENT).getChildren();
    assert.ok(rowNamed(nodes, "State"));
  });

  test("renders an Embedding progress row while a swap is in flight", () => {
    // [VSIX-SESSION-PROGRESS]
    const store = treeStore();
    store.setEmbeddingProgress({
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 23797,
      percent: 0,
      message: undefined,
    });
    const progress = rowNamed(sessionPanel(store, RUNNING_CLIENT).getChildren(), "Embedding");
    assert.ok(progress, "Embedding progress row must be present");
    assert.match(
      String(progress.description ?? ""),
      /0\s*\/\s*23[,.]?797/,
      "progress description must carry done / total",
    );
  });

  test("Embedding model row shows the pending id with a loading suffix while a swap is in flight", () => {
    // [VSIX-SESSION-PROGRESS]
    const store = treeStore();
    store.setPendingEmbeddingModel("nomic-embed-text");
    const nodes = sessionPanel(store, RUNNING_CLIENT).getChildren();
    const embeddingRow = rowNamed(nodes, EMBEDDING_MODEL_LABEL);
    assert.ok(embeddingRow, "Embedding model row must be rendered");
    assert.match(
      String(embeddingRow.description ?? ""),
      /nomic-embed-text.*loading/i,
      "pending model id must be visible with a loading hint",
    );
  });

  test("Embedding model row prompts for selection when live embeddings are off", () => {
    // [LIVE-EMBEDDING-CONSENT]
    const snapshot = report([]);
    snapshot.embedding_provenance = undefined;
    const store = new ReportStore();
    store.setSnapshot(snapshot, 0);
    const nodes = sessionPanel(store, RUNNING_CLIENT).getChildren();
    const embeddingRow = rowNamed(nodes, EMBEDDING_MODEL_LABEL);
    assert.ok(embeddingRow, "Embedding model row must be rendered");
    assert.match(
      String(embeddingRow.description ?? ""),
      /select model/i,
      "session panel must make model selection discoverable",
    );
  });

  test("failed lifecycle renders a Stopped error status node with a revealLog command", () => {
    // Exercises scanStatus's failed branch (StatusNode kind=error).
    const store = new ReportStore();
    store.setLifecycle({ kind: "failed", message: "binary missing" });
    const nodes = sessionPanel(store, NO_CLIENT).getChildren();
    const errorNode = nodes.find(
      (n) => typeof n.contextValue === STRING_TYPE_NAME && n.contextValue === "deslop.status.error",
    );
    assert.ok(errorNode, `expected an error StatusNode, got ${JSON.stringify(nodes.map(labelText))}`);
    assert.match(labelText(errorNode), /Stopped: binary missing/);
    assert.equal(errorNode.command?.command, "deslop.revealLog");
  });

  test("retains session data during re-analysis — stale > blank ([VSIX-REACTIVITY-TREE])", () => {
    const store = treeStore([cluster("a", 1, "/f")]);
    store.setLifecycle({ kind: "analysing" });
    const nodes = sessionPanel(store, NO_CLIENT).getChildren();
    assert.equal(nodes.length, SESSION_ROW_COUNT, "session rows must remain visible during re-analysis");
    const labels = nodes.map((n) => (typeof n.label === STRING_TYPE_NAME ? n.label : ""));
    assert.ok(labels.includes(EMBEDDING_MODEL_LABEL), "Embedding model row must stay visible");
    assert.ok(labels.includes("State"), "State row must stay visible");
  });
});
