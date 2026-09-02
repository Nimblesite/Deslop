// Unit: ReportStore snapshot + delta wiring. Runs under vscode-test so the
// transitive `vscode` EventEmitter resolves.

import * as assert from "node:assert/strict";
import { ReportStore } from "../../reportStore";
import { Report, ReportDelta } from "../../types/report";

import { cluster, delta, emptyReport, metrics, occurrence } from "./report-store.helpers";


/** The recomputed metrics every applyDelta case asserts against. */
const DELTA_METRICS = metrics({
  analysed_loc: 9367,
  duplicated_loc: 1046,
  duplication_percent: 11.2,
});
const EMBEDDING_MODEL_ID = "nomic-embed-text";

/**
 * Applies one delta and returns the resulting report. Every applyDelta
 * case respelled the same apply-then-assert-it-exists preamble; Deslop
 * scored the copies against this repo's own corpus. The `assert.ok` also
 * narrows `Report | undefined` for the caller's own assertions.
 */
function applyAndRead(store: ReportStore, overrides: Partial<ReportDelta>): Report {
  store.applyDelta(delta(overrides));
  const out = store.current.report;
  assert.ok(out, "report must exist after applyDelta");
  return out;
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

  // RA-05: the wire generation is not a freshness token — out-of-order
  // refresh completions can relabel it backward and then forward again
  // (ABA). The store owns a strictly monotonic revision instead, and it
  // refuses a generation rollback outright.
  test("revision advances on every accepted mutation and generation rollback is rejected", () => {
    const store = new ReportStore();
    assert.equal(store.current.revision, 0, "a fresh store starts at revision 0");

    assert.equal(store.setSnapshot(emptyReport({ clusters: [cluster("a", 5)] }), 3), true);
    assert.equal(store.current.revision, 1, "an accepted snapshot bumps the revision");
    assert.equal(store.current.generation, 3);

    // A stale completion labelled with an older generation: rejected whole.
    assert.equal(store.setSnapshot(emptyReport({ clusters: [cluster("stale", 9)] }), 2), false);
    assert.equal(store.current.generation, 3, "the generation never rolls backward");
    assert.equal(store.current.revision, 1, "a rejected snapshot must not bump the revision");
    assert.equal(store.current.report?.clusters[0]?.id, "a", "the content stays untouched");

    // The ABA relabel: same generation, different content — accepted, and
    // only the revision records that the world changed.
    assert.equal(store.setSnapshot(emptyReport({ clusters: [cluster("b", 4)] }), 3), true);
    assert.equal(store.current.revision, 2, "a same-generation replacement still advances the revision");
    assert.equal(store.current.generation, 3, "the generation label reads 3 again");
    assert.equal(store.current.report?.clusters[0]?.id, "b");

    const applied = store.applyDelta({
      from_generation: 3,
      to_generation: 4,
      clusters_added: [cluster("c", 2)],
      clusters_removed: [],
      clusters_updated: [],
    literal_findings_added: [],
    literal_findings_removed: [],
    literal_findings_updated: [],
      metrics: metrics(),
      cache_stats: { hits: 0, misses: 0 },
      tool_version: "tool-v2",
    });
    assert.equal(applied, true);
    assert.equal(store.current.revision, 3, "an applied delta bumps the revision");
    assert.equal(store.current.generation, 4);
  });

  // [PRINCIPLES-LIVE-IS-REACTIVE] [VSIX reactivity] An empty report may be a
  // cache seed or a mid-scan snapshot, so it must NOT settle the lifecycle to
  // "ready" — otherwise
  // the panel declares "No duplication detected" while a scan is still
  // running. Only the server's analysisState idle signal settles it.
  test("setSnapshot leaves an in-flight lifecycle alone when the report is empty", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: "analysing" });
    store.setSnapshot(emptyReport(), 7);
    assert.equal(
      store.current.lifecycle.kind,
      "analysing",
      "an empty report must not prematurely declare the scan complete",
    );
  });

  test("setSnapshot settles the lifecycle to ready when the report carries findings", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: "analysing" });
    store.setSnapshot(
      emptyReport({
        clusters: [
          cluster("c", 10, [occurrence("/repo/A.cs", 0, 10), occurrence("/repo/B.cs", 0, 10)]),
        ],
      }),
      7,
    );
    assert.equal(
      store.current.lifecycle.kind,
      "ready",
      "a report with findings is self-evidently a completed analysis",
    );
  });

  test("applyDelta is a no-op when there is no seeded report", () => {
    const store = new ReportStore();
    let fired = 0;
    store.onDidChange(() => {
      fired += 1;
    });
    store.applyDelta(
      delta({
        from_generation: 0,
        to_generation: 1,
        tool_version: "v",
      }),
    );
    assert.equal(fired, 0);
  });

  test("applyDelta adds, updates, removes, and orders by the engine's rank", () => {
    // The engine restamps the whole ranking on every generation, so the
    // delta's clusters arrive carrying their new ranks and the merge
    // orders on those — never on a weight comparison of its own, which
    // would have to guess the engine's tie-break
    // ([VSIX-TOP-OFFENDERS-RANK-GLOBAL], [PRINCIPLES-ONE-CALCULATION]).
    const store = new ReportStore();
    const a = cluster("a", 2, [], 1);
    const b = cluster("b", 1, [], 2);
    store.setSnapshot(emptyReport({ clusters: [a, b] }), 1);
    store.applyDelta(
      delta({
        clusters_added: [cluster("c", 10, [], 1)],
        clusters_removed: ["a"],
        clusters_updated: [cluster("b", 5, [], 2)],
        literal_findings_added: [],
        literal_findings_removed: [],
        literal_findings_updated: [],
        cache_stats: { hits: 3, misses: 4 },
        tool_version: "v2",
      }),
    );
    const out = store.current.report;
    assert.ok(out, "report must exist after applyDelta");
    assert.deepEqual(
      out.clusters.map((c) => c.id),
      ["c", "b"],
      "the merged list follows the ranks the engine published, worst first",
    );
    assert.deepEqual(
      out.clusters.map((c) => c.rank),
      [1, 2],
      "and each cluster keeps the rank it arrived with",
    );
    assert.equal(out.cache_stats.hits, 3);
    assert.equal(out.tool_version, "v2");
    assert.equal(store.current.generation, 2);
  });

  // #199: the DUPLICATION headline + per-file rows read straight from
  // report.metrics, and the delta path is the one almost always taken
  // after the first snapshot. applyDelta must therefore overwrite the
  // carried-over seed metrics with the delta's recomputed values, or the
  // headline freezes for the rest of the session.
  test("applyDelta replaces report.metrics with the delta's recomputed metrics (#199)", () => {
    const store = new ReportStore();
    store.setSnapshot(
      emptyReport({
        metrics: metrics({ analysed_loc: 8981, duplicated_loc: 1588, duplication_percent: 17.7 }),
      }),
      1,
    );
    const out = applyAndRead(store, { metrics: DELTA_METRICS });
    assert.equal(out.metrics.duplication_percent, 11.2, "headline percent must follow the delta");
    assert.equal(out.metrics.analysed_loc, 9367, "analysed LOC must follow the delta");
    assert.equal(out.metrics.duplicated_loc, 1046, "duplicated LOC must follow the delta");
  });

  // #196: the Duplication panel gates on metrics.duplicated_loc while Top
  // Offenders gates on clusters.length. When the seed snapshot is empty (a
  // cache seed or mid-scan snapshot) its metrics are zero, and the live
  // loop then streams clusters in via deltas. If applyDelta failed to carry
  // the delta's recomputed metrics, Top Offenders would light up while the
  // Duplication panel stayed pinned at "No duplication detected". Assert the
  // zero-seed -> delta transition moves metrics off zero AND populates
  // clusters, so the two panels can no longer disagree.
  test("applyDelta moves metrics off a zero seed when a delta brings clusters (#196)", () => {
    const store = new ReportStore();
    store.setSnapshot(emptyReport(), 1);
    assert.equal(store.current.report?.metrics.duplicated_loc, 0, "seed starts clean");
    const out = applyAndRead(store, {
      clusters_added: [
        cluster("71a9ee9", 6191, [
          occurrence("crates/osprey-codegen/src/collections.rs", 0, 10),
          occurrence("crates/osprey-codegen/src/strings.rs", 0, 10),
        ]),
      ],
      metrics: DELTA_METRICS,
    });
    assert.equal(out.metrics.duplicated_loc, 1046, "duplicated LOC must follow the delta off zero");
    assert.equal(out.metrics.duplication_percent, 11.2, "headline percent must reflect the delta");
    assert.equal(out.clusters.length, 1, "the delta's cluster must populate Top Offenders");
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
    store.setPendingEmbeddingModel(EMBEDDING_MODEL_ID);
    assert.equal(store.current.pendingEmbeddingModel, EMBEDDING_MODEL_ID);
    assert.equal(fired, 1);
  });

  test("setSnapshot clears any pending embedding model once a fresh report arrives", () => {
    const store = new ReportStore();
    store.setPendingEmbeddingModel(EMBEDDING_MODEL_ID);
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
      model_id: EMBEDDING_MODEL_ID,
      done: 0,
      total: 200,
      percent: 0,
      message: undefined,
    });
    assert.equal(fired, 1);
    assert.deepEqual(store.current.embeddingProgress, {
      phase: "starting",
      provider_id: "ollama",
      model_id: EMBEDDING_MODEL_ID,
      done: 0,
      total: 200,
      percent: 0,
      message: undefined,
    });
  });

  // A healthy long-lived session may never receive a full snapshot — deltas
  // alone carry it — and the retraction ledger used to be cleared only by one.
  // Every delta cloned the whole accumulated history, so N removals cost O(N)
  // retained ids and O(N²) copying over the session's life.
  test("the retraction ledger stays bounded across a long delta-only session", () => {
    const store = new ReportStore();
    store.setSnapshot(emptyReport({ clusters: [cluster("seed", 1)] }), 1);

    const churn = 2_000;
    for (let index = 0; index < churn; index += 1) {
      store.applyDelta({
        from_generation: index + 1,
        to_generation: index + 2,
        clusters_added: [cluster(`c-${index}`, 1)],
        // Remove the *previous* generation's cluster. Removing the one this
        // delta adds would un-retract it in the same pass — an add is the
        // server saying it found the cluster again.
        clusters_removed: index === 0 ? [] : [`c-${index - 1}`],
        clusters_updated: [],
    literal_findings_added: [],
    literal_findings_removed: [],
    literal_findings_updated: [],
        metrics: metrics(),
        cache_stats: { hits: 0, misses: 0 },
        tool_version: "v",
      });
    }

    const retracted = store.current.retractedClusters;
    assert.ok(
      retracted.size <= 256,
      `${churn} unique removals must not retain ${retracted.size} ids`,
    );
    assert.ok(
      retracted.has(`c-${churn - 2}`),
      "the most recent retraction is the one an in-flight probe could still name",
    );
    assert.equal(
      retracted.has("c-0"),
      false,
      "and the oldest is dropped first — no probe from 2000 generations ago is live",
    );
    assert.equal(
      store.current.generation,
      churn + 1,
      "every delta must still have applied",
    );
  });

  test("setEmbeddingProgress(null) clears the active progress state", () => {
    const store = new ReportStore();
    store.setEmbeddingProgress({
      phase: "complete",
      provider_id: "ollama",
      model_id: EMBEDDING_MODEL_ID,
      done: 64,
      total: 64,
      percent: 100,
      message: undefined,
    });
    store.setEmbeddingProgress(null);
    assert.equal(store.current.embeddingProgress, null);
  });
});
