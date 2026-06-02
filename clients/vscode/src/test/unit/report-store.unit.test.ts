// Unit: ReportStore snapshot + delta wiring. Runs under vscode-test so the
// transitive `vscode` EventEmitter resolves.

import * as assert from "node:assert/strict";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster, ReportDelta } from "../../types/report";

function emptyReport(overrides: Partial<Report> = {}): Report {
  return {
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
      per_file: [],
    },
    schema_doc: "",
    action_hints: [],
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters: [],
    ...overrides,
  };
}

function occurrence(path: string, startByte = 0, endByte = 10) {
  return { path, start_byte: startByte, end_byte: endByte, hidden: false };
}

function cluster(
  id: string,
  weight: number,
  occurrences: ReportCluster["occurrences"] = [],
): ReportCluster {
  return {
    id,
    weight,
    size: Math.max(1, occurrences.length),
    canonical_node_count: 0,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences,
    occurrences_total: occurrences.length,
    occurrences_truncated: false,
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

  test("visibleReport elides dirty-file occurrences and singleton clusters; canonical report keeps everything (#78, #117, #130)", () => {
    const store = new ReportStore();
    let fired = 0;
    store.onDidChange(() => {
      fired += 1;
    });
    store.setSnapshot(
      emptyReport({
        clusters: [
          cluster("only-dirty", 30, [occurrence("/repo/Dirty.cs", 10, 20)]),
          cluster("mixed-singleton", 25, [
            occurrence("/repo/Dirty.cs", 30, 40),
            occurrence("/repo/Clean.cs", 50, 60),
          ]),
          cluster("mixed-peers", 20, [
            occurrence("/repo/Dirty.cs", 70, 80),
            occurrence("/repo/CleanA.cs", 90, 100),
            occurrence("/repo/CleanB.cs", 110, 120),
          ]),
          cluster("untouched", 10, [
            occurrence("/repo/OtherA.cs", 130, 140),
            occurrence("/repo/OtherB.cs", 150, 160),
          ]),
        ],
      }),
      7,
    );

    store.markFileDirty("/repo/Dirty.cs");

    // Canonical report: untouched. Every cluster the LSP reported is still
    // resolvable so commands can look them up by id ([VSIX-STATE-DIRTY]).
    const canonical = store.current.report;
    assert.ok(canonical, "canonical report must remain available after markFileDirty");
    assert.equal(store.current.generation, 7, "markFileDirty must not fake a fresh LSP generation");
    assert.deepEqual(
      canonical.clusters.map((c) => c.id),
      ["only-dirty", "mixed-singleton", "mixed-peers", "untouched"],
      "canonical report keeps every cluster the LSP published",
    );
    assert.equal(
      canonical.clusters.find((c) => c.id === "mixed-peers")?.occurrences.length,
      3,
      "canonical occurrences are not mutated by client-side dirty tracking",
    );

    // Visible projection: dirty-file occurrences elided; clusters with fewer
    // than two remaining peers dropped (#117).
    const visible = store.current.visibleReport;
    assert.ok(visible, "visible projection must be derived once a report is loaded");
    assert.deepEqual(
      visible.clusters.map((c) => c.id),
      ["mixed-peers", "untouched"],
      "visible projection drops singleton-after-dirty clusters and keeps rank order",
    );
    assert.deepEqual(
      visible.clusters[0]?.occurrences.map((o) => o.path),
      ["/repo/CleanA.cs", "/repo/CleanB.cs"],
      "visible cluster keeps clean peer occurrences outside the edited file",
    );
    assert.equal(visible.clusters[0]?.size, 2, "visible count is reduced after pruning stale offsets");
    assert.equal(visible.clusters[0]?.occurrences_total, 2, "wire total is reduced with visible count");
    assert.ok(
      visible.clusters.every((c) => c.occurrences.length >= 2),
      "visible projection must not leave a one-copy top offender",
    );
    assert.equal(visible.metrics.clusters_total, 2, "visible metrics reflect the visible cluster count");
    assert.equal(fired, 2, "setSnapshot and markFileDirty both notify subscribers");
  });

  test("clearFileDirty re-exposes occurrences in the visible projection (#130)", () => {
    const store = new ReportStore();
    store.setSnapshot(
      emptyReport({
        clusters: [
          cluster("c", 10, [
            occurrence("/repo/Alpha.cs", 0, 10),
            occurrence("/repo/Beta.cs", 0, 10),
          ]),
        ],
      }),
      1,
    );
    store.markFileDirty("/repo/Alpha.cs");
    assert.equal(
      store.current.visibleReport?.clusters.length,
      0,
      "after dirty: visible projection elides the now-singleton cluster",
    );
    store.clearFileDirty("/repo/Alpha.cs");
    assert.equal(
      store.current.visibleReport?.clusters.length,
      1,
      "clearFileDirty restores the visible projection to the canonical view",
    );
    assert.equal(
      store.current.visibleReport?.clusters[0]?.occurrences.length,
      2,
      "both occurrences are visible again once the file is no longer dirty",
    );
  });

  // Regression: #130 (VSIX-STATE-DIRTY). Editor-side dirty tracking must not
  // mutate the canonical report. Commands that resolve a cluster by id
  // (compareWithCanonical, openCluster, openOccurrence, ...) read
  // store.current.report and break the moment a 2-occurrence cluster loses one
  // peer to an unsaved edit. The canonical report is owned by the LSP — only
  // deslop/reportChanged retracts a cluster.
  test("markFileDirty leaves the canonical report intact so cluster ids stay resolvable (#130)", () => {
    const store = new ReportStore();
    const onlyCluster = cluster("only-cluster", 30, [
      occurrence("/repo/Alpha.cs", 10, 20),
      occurrence("/repo/Beta.cs", 30, 40),
    ]);
    store.setSnapshot(emptyReport({ clusters: [onlyCluster] }), 1);

    store.markFileDirty("/repo/Alpha.cs");

    const canonical = store.current.report;
    assert.ok(canonical, "canonical report must remain available after markFileDirty");
    const found = canonical.clusters.find((x) => x.id === "only-cluster");
    assert.ok(
      found,
      "cluster id must stay resolvable in canonical report after markFileDirty so compareWithCanonical / openCluster / openOccurrence keep working",
    );
    assert.equal(
      found.occurrences.length,
      2,
      "canonical occurrences must not be mutated by client-side dirty tracking — only deslop/reportChanged writes the canonical report",
    );
    assert.equal(
      store.current.generation,
      1,
      "markFileDirty must not fake a fresh LSP generation",
    );
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

  test("setEmbeddingProgress exposes the latest event and fires onDidChange", () => {
    const store = new ReportStore();
    let fired = 0;
    store.onDidChange(() => {
      fired += 1;
    });
    store.setEmbeddingProgress({
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 200,
      message: undefined,
    });
    assert.equal(fired, 1);
    assert.deepEqual(store.current.embeddingProgress, {
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 200,
      message: undefined,
    });
  });

  test("setEmbeddingProgress(null) clears the active progress state", () => {
    const store = new ReportStore();
    store.setEmbeddingProgress({
      phase: "complete",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 64,
      total: 64,
      message: undefined,
    });
    store.setEmbeddingProgress(null);
    assert.equal(store.current.embeddingProgress, null);
  });
});
