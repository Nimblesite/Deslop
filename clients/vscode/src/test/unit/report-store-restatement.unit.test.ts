// [LIVE-DELTA] A delta from generation 0 restates the corpus: the server held
// no baseline, so it lists every live cluster as added and names nothing
// removed. The store must replace its cluster list with it, never merge.
import * as assert from "node:assert/strict";
import { ReportStore } from "../../reportStore";
import { cluster, delta, emptyReport, metrics } from "./report-store.helpers";

const SEED_GENERATION = 0;
const RESTATED_GENERATION = 2;
const NEXT_GENERATION = 3;
const STALE_BASELINE = 1;
const RUN_AHEAD_GENERATION = 5;
const KEPT = "kept";
const MOVED = "moved";
const EXCLUDED = "excluded";
const LOW_MASS = 10;
const HIGH_MASS = 40;
const FIRST_RANK = 1;
const SECOND_RANK = 2;
const THIRD_RANK = 3;
const RESTATED_METRICS = metrics({ duplicated_loc: 56, analysed_loc: 81, duplication_percent: 69.1 });

function ids(store: ReportStore): string[] {
  return store.current.report?.clusters.map((item) => item.id) ?? [];
}

function seeded(generation: number): ReportStore {
  const store = new ReportStore();
  const clusters = [cluster(KEPT, HIGH_MASS, [], FIRST_RANK), cluster(MOVED, LOW_MASS, [], SECOND_RANK), cluster(EXCLUDED, LOW_MASS, [], THIRD_RANK)];
  assert.equal(store.setSnapshot(emptyReport({ clusters }), generation), true);
  return store;
}

function restatement(toGeneration: number) {
  return delta({
    from_generation: SEED_GENERATION,
    to_generation: toGeneration,
    clusters_added: [cluster(KEPT, HIGH_MASS, [], FIRST_RANK)],
    clusters_updated: [cluster(MOVED, HIGH_MASS, [], SECOND_RANK)],
    metrics: RESTATED_METRICS,
  });
}

/** A diff from `from` that withdraws `id`. */
function removal(from: number, id: string) {
  return delta({ from_generation: from, to_generation: NEXT_GENERATION, clusters_removed: [id] });
}

suite("report store restatements", () => {
  test("a restatement replaces the seeded clusters and retracts what it omits", () => {
    const store = seeded(SEED_GENERATION);
    assert.equal(store.applyDelta(restatement(RESTATED_GENERATION)), true, "the first change after the seed applies");
    assert.deepEqual(ids(store), [KEPT, MOVED], "the cluster the server dropped is gone");
    assert.equal(store.current.report?.clusters[1]?.mass, HIGH_MASS, "a restated cluster carries its new values");
    assert.equal(store.current.generation, RESTATED_GENERATION);
    assert.deepEqual(store.current.report?.metrics, RESTATED_METRICS, "the headline follows the restatement");
    assert.ok(store.current.retractedClusters.has(EXCLUDED), "the omitted cluster counts as withdrawn");
    assert.equal(store.applyDelta(removal(RESTATED_GENERATION, MOVED)), true, "a diff from the restated generation still merges");
    assert.deepEqual(ids(store), [KEPT]);
    assert.equal(store.applyDelta(removal(STALE_BASELINE, KEPT)), false, "a diff from a missed baseline is still refused");
    assert.deepEqual(ids(store), [KEPT], "a refused diff changes nothing");
  });

  test("a restatement heals a store whose generation ran ahead of the server", () => {
    const store = seeded(RUN_AHEAD_GENERATION);
    const revision = store.current.revision;
    const diff = delta({ from_generation: RESTATED_GENERATION, to_generation: NEXT_GENERATION });
    assert.equal(store.applyDelta(diff), false, "no diff can match a label the server never issued");
    assert.equal(store.setSnapshot(emptyReport(), RESTATED_GENERATION), false, "and every snapshot looks like a rollback");
    assert.equal(store.applyDelta(restatement(RESTATED_GENERATION)), true, "the server's restatement is accepted");
    assert.equal(store.current.generation, RESTATED_GENERATION, "the label is the server's again");
    assert.ok(store.current.revision > revision, "readers see a new revision");
    assert.deepEqual(ids(store), [KEPT, MOVED]);
    assert.equal(store.setSnapshot(emptyReport({ clusters: [cluster(KEPT, HIGH_MASS)] }), RESTATED_GENERATION), true, "later reports land again");
  });
});
