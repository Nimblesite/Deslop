// Unit: deslop/reportChanged refresh wiring — delta application, snapshot
// fallback, refresh serialisation, and missed-generation convergence
// (#230). Split from extension-internals.unit.test.ts to honour the
// 500-line file rule; assertions unchanged.

import * as assert from "node:assert/strict";
import type { LanguageClient } from "vscode-languageclient/node";
import { refreshAfterChange, wireNotifications } from "../../notifications";
import { ReportStore } from "../../reportStore";
import { cluster, report } from "./tree.helpers";
import { emptyReport, repoMetrics } from "./report.helpers";

const REPORT_DELTA_METHOD = "deslop/reportDelta";
const LIVE_GENERATION = 3;

suite("reportChanged refresh wiring", () => {
  test("wireNotifications reportChanged applies a delta", async () => {
    let changedCb: ((p: unknown) => void) | undefined;
    const requests: string[] = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => void) => {
        if (name === "deslop/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        if (name === REPORT_DELTA_METHOD) {
          return Promise.resolve({
            from_generation: 0,
            to_generation: 1,
            clusters_added: [],
            clusters_removed: [],
            clusters_updated: [],
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0,
            metrics: repoMetrics(),
            cache_stats: { hits: 0, misses: 0 },
            tool_version: "v",
          });
        }
        return Promise.resolve({});
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    store.setSnapshot(
      emptyReport({
        tool_version: "v0",
        metrics: repoMetrics(),
      }),
      0,
    );
    const schedule = wireNotifications(client, store);
    changedCb?.({ generation: 1, summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0,
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0, worst_mass: 0 } });
    await schedule.settled();
    assert.ok(requests.includes(REPORT_DELTA_METHOD));
    assert.equal(store.current.generation, 1, "the queued delta must be applied by settled()");
  });

  test("wireNotifications reportChanged falls back to reportGet when delta is null", async () => {
    let changedCb: ((p: unknown) => void) | undefined;
    const requests: string[] = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => void) => {
        if (name === "deslop/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        if (name === REPORT_DELTA_METHOD) return Promise.resolve(null);
        return Promise.resolve(emptyReport({
          tool_version: "x",
          metrics: repoMetrics(),
        }));
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    const schedule = wireNotifications(client, store);
    changedCb?.({ generation: 5, summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0,
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0, worst_mass: 0 } });
    await schedule.settled();
    assert.ok(requests.includes("deslop/reportGet"));
    assert.equal(store.current.generation, 5, "the fallback snapshot must be stored by settled()");
  });

  // RA-05: refreshes used to run concurrently, so an early notification's
  // slow reportGet could complete *after* a later one's and clobber the
  // store with older content labelled with an older generation (the first
  // half of the generation ABA). The queue serialises them: a refresh does
  // not even dispatch until every earlier one has fully applied.
  test("reportChanged refreshes are serialised so a slow early snapshot cannot clobber a later one", async () => {
    let changedCb: ((p: unknown) => void) | undefined;
    const pendingGets: Array<(snapshot: unknown) => void> = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => void) => {
        if (name === "deslop/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        if (name === REPORT_DELTA_METHOD) return Promise.resolve(null);
        return new Promise((resolve) => {
          pendingGets.push(resolve);
        });
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    const schedule = wireNotifications(client, store);

    const notify = (generation: number) =>
      changedCb?.({
        generation,
        summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0,
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0, worst_mass: 0 },
      });
    const drainUntil = async (condition: () => boolean) => {
      for (let i = 0; i < 50 && !condition(); i++) await Promise.resolve();
    };

    notify(2);
    notify(LIVE_GENERATION);
    await drainUntil(() => pendingGets.length >= 1);
    assert.equal(
      pendingGets.length,
      1,
      "the second refresh must stay queued while the first snapshot is in flight",
    );

    const snapshotFor = (id: string) =>
      report([cluster(id, 10, `/repo/${id}.cs`)]);
    pendingGets[0]?.(snapshotFor("older"));
    await drainUntil(() => pendingGets.length >= 2);
    assert.equal(store.current.generation, 2, "the first refresh applies before the second dispatches");
    assert.equal(store.current.report?.clusters[0]?.id, "older");

    pendingGets[1]?.(snapshotFor("newer"));
    await schedule.settled();
    assert.equal(store.current.generation, LIVE_GENERATION, "the later notification's snapshot lands last");
    assert.equal(
      store.current.report?.clusters[0]?.id,
      "newer",
      "the newer content must win — concurrent refreshes let the older snapshot land last",
    );
  });

  // Regression (#230): a missed/lagged deslop/reportChanged leaves the store at
  // an older baseline than the single-step delta the server returns by default
  // (current-1 -> current). Applying that delta on the stale base never retracts
  // the clusters dropped in the skipped generations, so a discarded cluster
  // survives as a phantom rank-#1 entry. The refresh must converge the store to
  // the live engine instead of merging a delta against a mismatched baseline.
  test("refreshAfterChange converges to the engine after a missed generation (#230)", async () => {
    // Engine history: gen 1 [phantom(100), keep(50)] -> gen 2 drops phantom
    // (MISSED by the client) -> gen 3 adds fresh(80). Live truth at gen 3 is
    // worst-first [fresh, keep]; "phantom" no longer exists in the engine.
    const keep = cluster("keep", 50, "/repo/Keep.cs");
    const fresh = cluster("fresh", 80, "/repo/Fresh.cs");
    const liveReport = report([fresh, keep]);

    const deltaSinceParams: Array<number | undefined> = [];
    const client = {
      sendRequest: (name: string, params?: { since_generation?: number }) => {
        if (name === REPORT_DELTA_METHOD) {
          deltaSinceParams.push(params?.since_generation);
          // The server answers `since -> current(3)`. With the correct baseline
          // (1) it can retract "phantom"; the buggy no-since default (current-1
          // = 2) returns a delta that cannot, because phantom left in gen 2.
          const since = params?.since_generation ?? 2;
          if (since === 1) {
            return Promise.resolve({
              from_generation: 1,
              to_generation: LIVE_GENERATION,
              clusters_added: [fresh],
              clusters_removed: ["phantom"],
              clusters_updated: [],
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0,
              cache_stats: { hits: 0, misses: 0 },
              tool_version: "v",
            });
          }
          return Promise.resolve({
            from_generation: since,
            to_generation: LIVE_GENERATION,
            clusters_added: [fresh],
            clusters_removed: [],
            clusters_updated: [],
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0,
            cache_stats: { hits: 0, misses: 0 },
            tool_version: "v",
          });
        }
        // deslop/reportGet always serves canonical live truth.
        return Promise.resolve(liveReport);
      },
    } as unknown as LanguageClient;

    const store = new ReportStore();
    store.setSnapshot(report([cluster("phantom", 100, "/repo/Phantom.cs"), keep]), 1);

    await refreshAfterChange(client, store, {
      generation: LIVE_GENERATION,
      summary: { clusters_added: 1, clusters_removed: 1, clusters_updated: 0,
    literal_findings_added: 0,
    literal_findings_removed: 0,
    literal_findings_updated: 0, worst_mass: 80 },
    });

    assert.deepEqual(
      store.current.report?.clusters.map((c) => c.id),
      ["fresh", "keep"],
      "the stale 'phantom' cluster (rank #1) must not survive a missed generation — " +
        "the store must converge to the live engine report",
    );
    assert.equal(store.current.generation, LIVE_GENERATION, "the store must advance to the live generation");
  });
});
