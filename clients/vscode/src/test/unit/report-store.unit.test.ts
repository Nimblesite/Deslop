// Unit: ReportStore snapshot + delta wiring. Runs under vscode-test so the
// transitive `vscode` EventEmitter resolves.

import * as assert from "node:assert/strict";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster, ReportDelta } from "../../types/report";

function emptyReport(overrides: Partial<Report> = {}): Report {
  return {
    report_schema_version: 1,
    tool_version: "tool-v1",
    min_nodes: 30,
    files_analysed: 0,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 0,
      duplicated_loc: 0,
      duplication_percent: 0,
      clusters_total: 0,
      duplicated_files: 0,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "",
    action_hints: [],
    embedding_provenance: null,
    clusters: [],
    ...overrides,
  };
}

function cluster(id: string, weight: number): ReportCluster {
  return {
    id,
    weight,
    size: 1,
    canonical_node_count: 0,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [],
    summary: "",
    interpretation: "",
  };
}

suite("ReportStore", () => {
  test("setSnapshot fires onDidChange with the stored state", () => {
    const store = new ReportStore();
    let fired = 0;
    store.onDidChange(() => {
      fired += 1;
    });
    store.setSnapshot(emptyReport(), 7);
    assert.equal(fired, 1);
    assert.equal(store.current.generation, 7);
  });

  test("applyDelta is a no-op when there is no seeded report", () => {
    const store = new ReportStore();
    let fired = 0;
    store.onDidChange(() => {
      fired += 1;
    });
    const delta: ReportDelta = {
      from_generation: 0,
      to_generation: 1,
      clusters_added: [],
      clusters_removed: [],
      clusters_updated: [],
      cache_stats: { hits: 0, misses: 0 },
      tool_version: "v",
    };
    store.applyDelta(delta);
    assert.equal(fired, 0);
  });

  test("applyDelta adds, updates, removes, and re-sorts by weight", () => {
    const store = new ReportStore();
    const a = cluster("a", 1);
    const b = cluster("b", 2);
    store.setSnapshot(emptyReport({ clusters: [a, b] }), 1);
    const delta: ReportDelta = {
      from_generation: 1,
      to_generation: 2,
      clusters_added: [cluster("c", 10)],
      clusters_removed: ["a"],
      clusters_updated: [cluster("b", 5)],
      cache_stats: { hits: 3, misses: 4 },
      tool_version: "v2",
    };
    store.applyDelta(delta);
    const out = store.current.report;
    assert.ok(out, "report must exist after applyDelta");
    assert.deepEqual(
      out.clusters.map((c) => c.id),
      ["c", "b"],
    );
    assert.equal(out.cache_stats.hits, 3);
    assert.equal(out.tool_version, "v2");
    assert.equal(store.current.generation, 2);
  });

  test("notifyChange fan-outs via onDidChangeSummary", () => {
    const store = new ReportStore();
    let seen: unknown = null;
    store.onDidChangeSummary((s) => {
      seen = s;
    });
    store.notifyChange({
      clusters_added: 1,
      clusters_removed: 0,
      clusters_updated: 2,
      worst_weight: 42,
    });
    assert.ok(seen);
  });

  test("dispose tears down emitters without throwing", () => {
    const store = new ReportStore();
    store.dispose();
  });

  test("setPendingEmbeddingModel exposes the pending id and fires onDidChange", () => {
    const store = new ReportStore();
    let fired = 0;
    store.onDidChange(() => {
      fired += 1;
    });
    store.setPendingEmbeddingModel("nomic-embed-text");
    assert.equal(store.current.pendingEmbeddingModel, "nomic-embed-text");
    assert.equal(fired, 1);
  });

  test("setSnapshot clears any pending embedding model once a fresh report arrives", () => {
    const store = new ReportStore();
    store.setPendingEmbeddingModel("nomic-embed-text");
    store.setSnapshot(emptyReport(), 1);
    assert.equal(store.current.pendingEmbeddingModel, null);
  });
});
