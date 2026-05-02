// Unit: SessionProvider. Drives getChildren() against a seeded store.

import * as assert from "node:assert/strict";
import { SessionProvider, StatusTicker } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, report } from "./tree.helpers";

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

  test("renders an Embedding progress row while a swap is in flight", () => {
    // [VSIX-SESSION-PROGRESS]
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    store.setEmbeddingProgress({
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 23797,
      message: null,
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
    // [VSIX-SESSION-PROGRESS]
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

  test("Embedding model row prompts for selection when live embeddings are off", () => {
    // [LIVE-EMBEDDING-CONSENT]
    const store = new ReportStore();
    const snapshot = report([]);
    snapshot.embedding_provenance = null;
    store.setSnapshot(snapshot, 0);
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const embeddingRow = nodes.find(
      (n) => typeof n.label === "string" && n.label === "Embedding model",
    );
    assert.ok(embeddingRow, "Embedding model row must be rendered");
    assert.match(
      String(embeddingRow.description ?? ""),
      /select model/i,
      "session panel must make model selection discoverable",
    );
  });

  test("failed lifecycle renders a Stopped error status node with a revealLog command", () => {
    // Exercises renderLifecycle's failed branch (StatusNode kind=error).
    const store = new ReportStore();
    store.setLifecycle({ kind: "failed", message: "binary missing" });
    const provider = new SessionProvider(store, new StatusTicker(), () => undefined);
    const nodes = provider.getChildren();
    const errorNode = nodes.find(
      (n) => typeof n.contextValue === "string" && n.contextValue === "deslop.status.error",
    );
    assert.ok(errorNode, `expected an error StatusNode, got ${JSON.stringify(nodes.map(labelText))}`);
    assert.match(labelText(errorNode), /Stopped: binary missing/);
    assert.equal(errorNode.command?.command, "deslop.revealLog");
  });

  test("retains session data during re-analysis — stale > blank ([VSIX-REACTIVITY-TREE])", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 1, "/f")]), 0);
    store.setLifecycle({ kind: "analysing" });
    const provider = new SessionProvider(store, new StatusTicker(), () => undefined);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 5, "session rows must remain visible during re-analysis");
    const labels = nodes.map((n) => (typeof n.label === "string" ? n.label : ""));
    assert.ok(labels.includes("Embedding model"), "Embedding model row must stay visible");
    assert.ok(labels.includes("State"), "State row must stay visible");
  });
});
