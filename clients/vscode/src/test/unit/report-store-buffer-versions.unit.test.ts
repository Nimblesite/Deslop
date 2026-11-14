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

/** The edit under test, as a report names it once analysed. */
const ANALYSED_NOW: AnalysedBuffer[] = [{ path: BUFFER_PATH, version: CURRENT_VERSION }];

function naming(snapshot: Report, analysed: AnalysedBuffer[]): Report {
  return { ...snapshot, analysed_buffers: analysed };
}

function analysedDelta(snapshot: Report, analysed?: AnalysedBuffer[]) {
  return delta({
    from_generation: INITIAL_GENERATION, to_generation: ANALYSED_GENERATION,
    clusters_updated: snapshot.clusters, metrics: snapshot.metrics, analysed_buffers: analysed,
  });
}

function applySnapshot(store: ReportStore, snapshot: Report, buffer: AnalysedBuffer): boolean {
  return store.setSnapshot(naming(snapshot, [buffer]), ANALYSED_GENERATION);
}

function applyDelta(store: ReportStore, snapshot: Report, buffer: AnalysedBuffer): boolean {
  return store.applyDelta(analysedDelta(snapshot, [buffer]));
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
    assert.equal(store.setSnapshot(naming(SNAPSHOT, ANALYSED_NOW), INITIAL_GENERATION), false);
    assert.equal(store.isFileDirty(BUFFER_PATH), true, "a stale report releases nothing");
    assert.equal(store.current.generation, ANALYSED_GENERATION);
    assert.equal(store.current.visibleReport?.clusters.length, NO_VISIBLE_CLUSTERS);
    assert.deepEqual(store.current.report?.clusters, SNAPSHOT.clusters);
  });
});

// The live-bubble race: a refresh started by an earlier notification can
// fetch the report that already analysed a later edit, before that edit's own
// notification arrives. The report names the versions it analysed, so it
// releases the edit as it lands, whichever refresh fetched it.
suite("a report names the buffer versions it analysed", () => {
  test("releases the named version atomically, whichever refresh fetched it", () => {
    const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    const observations: boolean[] = [];
    store.onDidChange((state) => {
      if (state.generation === ANALYSED_GENERATION) observations.push(store.isFileDirty(BUFFER_PATH));
    });
    assert.equal(store.applyDelta(analysedDelta(SNAPSHOT, ANALYSED_NOW)), true);
    assert.deepEqual(observations, [false], "no observer sees the analysed report while the buffer is still dirty");
    assert.equal(store.isFileDirty(BUFFER_PATH), false);
    assert.deepEqual(store.current.visibleReport?.clusters, SNAPSHOT.clusters);
  });

  test("a report that names no version for the buffer leaves it dirty until one does", () => {
    const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    assert.equal(store.applyDelta(analysedDelta(SNAPSHOT)), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), true, "generation 2 did not analyse this edit");
    const peerOnly = [{ path: PEER_PATH, version: CURRENT_VERSION }];
    assert.equal(store.setSnapshot(naming(SNAPSHOT, peerOnly), ANALYSED_GENERATION), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), true, "another file's version releases nothing here");
    assert.equal(store.setSnapshot(naming(SNAPSHOT, ANALYSED_NOW), ANALYSED_GENERATION + 1), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), false);
  });

  test("a new server session releases only what its own reports name", () => {
    const store = storeWith(SNAPSHOT, INITIAL_GENERATION);
    store.markFileDirty(BUFFER_PATH, CURRENT_VERSION);
    store.resetForNewSession();
    assert.equal(store.setSnapshot(naming(SNAPSHOT, []), ANALYSED_GENERATION), true);
    assert.equal(store.isFileDirty(BUFFER_PATH), true, "the new session has not analysed this edit");
  });
});
