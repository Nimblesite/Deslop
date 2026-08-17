// Unit: ReportStore dirty-file projection ([VSIX-STATE-DIRTY]) — the
// visible report elides occurrences in locally edited files while the
// canonical report keeps everything resolvable. Split from
// report-store.unit.test.ts to honour the 500-line file rule; assertions
// unchanged.

import * as assert from "node:assert/strict";
import { ReportStore } from "../../reportStore";
import { cluster, emptyReport, occurrence } from "./report-store.helpers";

suite("ReportStore dirty-file projection", () => {
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
  // [PRINCIPLES-LIVE-IS-REACTIVE] Dirty-file projection must not
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
});
