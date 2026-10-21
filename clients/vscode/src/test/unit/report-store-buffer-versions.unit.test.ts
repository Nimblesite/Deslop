// [LIVE-OPEN-BUFFERS] [VSIX-STATE-DIRTY] Isolate acknowledgement ordering.
import * as assert from "node:assert/strict";
import type { AnalysedBuffer, Report } from "../../types/report";
import { ReportStore } from "../../reportStore";
import { cluster, delta, emptyReport, occurrence, storeWith } from "./report-store.helpers";

const BUFFER_PATH = "/repo/Shipping.cs";
const PEER_PATH = "/repo/Peer.cs";
const CLUSTER_ID = "shipping-clone";
const CLONE_MASS = 76;
const RANGE_START = 112;
const RANGE_END = 450;
const INITIAL_GENERATION = 1;
const ANALYSED_GENERATION = 2;
const OLD_VERSION = 4;
const CURRENT_VERSION = 5;
const NO_VISIBLE_CLUSTERS = 0;
const SNAPSHOT = emptyReport({ clusters: [cluster(CLUSTER_ID, CLONE_MASS, [
  occurrence(BUFFER_PATH, RANGE_START, RANGE_END),
  occurrence(PEER_PATH, RANGE_START, RANGE_END),
])] });

function applySnapshot(store: ReportStore, snapshot: Report, buffer: AnalysedBuffer): boolean {
  return store.setSnapshot(snapshot, ANALYSED_GENERATION, buffer);
}

function applyDelta(store: ReportStore, snapshot: Report, buffer: AnalysedBuffer): boolean {
  return store.applyDelta(delta({
    from_generation: INITIAL_GENERATION, to_generation: ANALYSED_GENERATION,
    clusters_updated: snapshot.clusters, metrics: snapshot.metrics,
  }), buffer);
}

for (const apply of [applySnapshot, applyDelta]) {
  suite(`analysed buffer versions through ${apply.name}`, () => {
    test("an older editor acknowledgement cannot reveal newer edits", () => {
      const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
      store.markFileDirty(BUFFER_PATH, OLD_VERSION);
      store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
      assert.equal(apply(store, SNAPSHOT, { path: BUFFER_PATH, version: OLD_VERSION }), true);
      assert.deepEqual(store.current.report?.clusters, SNAPSHOT.clusters);
      assert.equal(store.current.visibleReport?.clusters.length, NO_VISIBLE_CLUSTERS);
      assert.equal(store.current.generation, ANALYSED_GENERATION);
    });

    test("the matching editor version and report become visible atomically", () => {
      const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
      store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
      const observations: Report[] = [];
      store.onDidChange((state) => {
        if (state.visibleReport) observations.push(state.visibleReport);
      });
      assert.equal(apply(store, SNAPSHOT, { path: BUFFER_PATH, version: CURRENT_VERSION }), true);
      assert.deepEqual(observations.map((report) => report.clusters), [SNAPSHOT.clusters]);
      assert.deepEqual(store.current.visibleReport?.clusters, SNAPSHOT.clusters);
      assert.deepEqual(store.current.visibleReport?.metrics, SNAPSHOT.metrics);
    });
  });
}

suite("analysed buffer generation guards", () => {
  test("a rejected snapshot cannot acknowledge the current editor version", () => {
    const store = storeWith(SNAPSHOT, ANALYSED_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    const buffer = { path: BUFFER_PATH, version: CURRENT_VERSION };
    assert.equal(store.setSnapshot(SNAPSHOT, INITIAL_GENERATION, buffer), false);
    assert.equal(store.current.generation, ANALYSED_GENERATION);
    assert.equal(store.current.visibleReport?.clusters.length, NO_VISIBLE_CLUSTERS);
    assert.deepEqual(store.current.report?.clusters, SNAPSHOT.clusters);
  });
});
