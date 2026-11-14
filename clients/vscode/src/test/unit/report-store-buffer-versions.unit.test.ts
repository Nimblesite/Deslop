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

function analysedDelta(snapshot: Report) {
  return delta({
    from_generation: INITIAL_GENERATION, to_generation: ANALYSED_GENERATION,
    clusters_updated: snapshot.clusters, metrics: snapshot.metrics,
  });
}

function applySnapshot(store: ReportStore, snapshot: Report, buffer: AnalysedBuffer): boolean {
  store.noteAnalysedBuffer(ANALYSED_GENERATION, buffer);
  return store.setSnapshot(snapshot, ANALYSED_GENERATION);
}

function applyDelta(store: ReportStore, snapshot: Report, buffer: AnalysedBuffer): boolean {
  store.noteAnalysedBuffer(ANALYSED_GENERATION, buffer);
  return store.applyDelta(analysedDelta(snapshot));
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
    store.noteAnalysedBuffer(ANALYSED_GENERATION + 1, { path: BUFFER_PATH, version: CURRENT_VERSION });
    assert.equal(store.setSnapshot(SNAPSHOT, INITIAL_GENERATION), false);
    assert.equal(store.current.generation, ANALYSED_GENERATION);
    assert.equal(store.current.visibleReport?.clusters.length, NO_VISIBLE_CLUSTERS);
    assert.deepEqual(store.current.report?.clusters, SNAPSHOT.clusters);
  });
});

// The live-bubble race: a refresh started by an earlier notification fetches
// the delta that already carries the edit's generation, before the edit's own
// refresh runs. The buffer that notification named is acknowledged by that
// report as it lands, so the new offsets are never shown as unacknowledged.
suite("an earlier refresh that lands the analysed generation", () => {
  test("acknowledges the buffer the later notification named, atomically", () => {
    const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    store.noteAnalysedBuffer(ANALYSED_GENERATION, { path: BUFFER_PATH, version: CURRENT_VERSION });
    const observations: boolean[] = [];
    store.onDidChange((state) => {
      if (state.generation === ANALYSED_GENERATION) observations.push(store.isFileDirty(BUFFER_PATH));
    });
    assert.equal(store.applyDelta(analysedDelta(SNAPSHOT)), true);
    assert.deepEqual(observations, [false], "no observer sees the analysed report while the buffer is still dirty");
    assert.equal(store.isFileDirty(BUFFER_PATH), false);
    assert.deepEqual(store.current.visibleReport?.clusters, SNAPSHOT.clusters);
  });

  test("a report short of the analysed generation leaves the buffer dirty until one reaches it", () => {
    const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    store.noteAnalysedBuffer(ANALYSED_GENERATION + 1, { path: BUFFER_PATH, version: CURRENT_VERSION });
    assert.equal(store.applyDelta(analysedDelta(SNAPSHOT)), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), true, "generation 2 did not analyse the version generation 3 did");
    assert.equal(store.setSnapshot(SNAPSHOT, ANALYSED_GENERATION + 1), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), false);
  });

  test("a new server session forgets buffers the old one analysed", () => {
    const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    store.noteAnalysedBuffer(ANALYSED_GENERATION, { path: BUFFER_PATH, version: CURRENT_VERSION });
    store.resetForNewSession();
    assert.equal(store.setSnapshot(SNAPSHOT, ANALYSED_GENERATION), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), true, "the old session's analysis is not this one's");
  });
});
