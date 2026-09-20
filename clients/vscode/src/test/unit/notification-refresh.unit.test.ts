import { FIXTURE_ROUTING } from "../cluster.helpers";
// Unit: deslop/reportChanged refresh wiring — delta application, snapshot
// fallback, refresh serialisation, and missed-generation convergence
// (#230). Split from extension-internals.unit.test.ts to honour the
// 500-line file rule; assertions unchanged.

import * as assert from "node:assert/strict";
import { refreshAfterChange, wireNotifications } from "../../notifications";
import { ReportStore } from "../../reportStore";
import type { ChangeSummary, ReportChangedNotification } from "../../types/report";
import { cluster, report, storeWith } from "./tree.helpers";
import { emptyReport, repoMetrics } from "./report.helpers";
import { notifyingClient } from "./client.helpers";

const REPORT_DELTA_METHOD = "deslop/reportDelta";
const REPORT_CHANGED_METHOD = "deslop/reportChanged";
const REPORT_GET_METHOD = "deslop/reportGet";
const LIVE_GENERATION = 3;

/** The literal-finding counters every delta and summary in this suite carries. */
const NO_LITERAL_FINDING_CHANGES = {
  literal_findings_added: 0,
  literal_findings_removed: 0,
  literal_findings_updated: 0,
};

/** A `deslop/reportDelta` reply in the shape the server sends, with every
 * field a test does not pin held at its empty value. */
function wireDelta(overrides: Record<string, unknown>): Record<string, unknown> {
  return {
    clusters_added: [],
    clusters_removed: [],
    clusters_updated: [],
    ...NO_LITERAL_FINDING_CHANGES,
    cache_stats: { hits: 0, misses: 0 },
    tool_version: "v",
    ...overrides,
  };
}

/** The `deslop/reportChanged` payload for `generation`, typed against the wire
 * contract. The summary counters default to "nothing changed"; a test pins
 * only what it is asserting. */
function changedPayload(
  generation: number,
  summary: Partial<ChangeSummary> = {},
): ReportChangedNotification {
  return {
    generation,
    summary: {
      clusters_added: 0,
      clusters_removed: 0,
      clusters_updated: 0,
      ...NO_LITERAL_FINDING_CHANGES,
      worst_mass: 0,
      ...summary,
    },
  };
}

suite("reportChanged refresh wiring", () => {
  test("wireNotifications reportChanged applies a delta", async () => {
    const { calls, client, notify } = notifyingClient((name) =>
      name === REPORT_DELTA_METHOD
        ? wireDelta({
      routing: FIXTURE_ROUTING, from_generation: 0, to_generation: 1, metrics: repoMetrics() })
        : {},
    );
    const store = storeWith(
      emptyReport({
        tool_version: "v0",
        metrics: repoMetrics(),
      }),
    );
    const schedule = wireNotifications(client, store);
    notify(REPORT_CHANGED_METHOD, changedPayload(1));
    await schedule.settled();
    assert.ok(calls.some((call) => call.method === REPORT_DELTA_METHOD));
    assert.equal(store.current.generation, 1, "the queued delta must be applied by settled()");
  });

  test("wireNotifications reportChanged falls back to reportGet when delta is null", async () => {
    const { calls, client, notify } = notifyingClient((name) =>
      name === REPORT_DELTA_METHOD
        ? null
        : emptyReport({ tool_version: "x", metrics: repoMetrics() }),
    );
    const store = new ReportStore();
    const schedule = wireNotifications(client, store);
    notify(REPORT_CHANGED_METHOD, changedPayload(5));
    await schedule.settled();
    assert.ok(calls.some((call) => call.method === REPORT_GET_METHOD));
    assert.equal(store.current.generation, 5, "the fallback snapshot must be stored by settled()");
  });

  // RA-05: refreshes used to run concurrently, so an early notification's
  // slow reportGet could complete *after* a later one's and clobber the
  // store with older content labelled with an older generation (the first
  // half of the generation ABA). The queue serialises them: a refresh does
  // not even dispatch until every earlier one has fully applied.
  test("reportChanged refreshes are serialised so a slow early snapshot cannot clobber a later one", async () => {
    const pendingGets: Array<(snapshot: unknown) => void> = [];
    const { client, notify: notifyChanged } = notifyingClient((name) =>
      name === REPORT_DELTA_METHOD
        ? null
        : new Promise((resolve) => {
            pendingGets.push(resolve);
          }),
    );
    const store = new ReportStore();
    const schedule = wireNotifications(client, store);

    const notify = (generation: number) =>
      notifyChanged(REPORT_CHANGED_METHOD, changedPayload(generation));
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

    // `calls` keeps every since_generation the client asked for, so the
    // baseline the refresh chose is observable.
    const { client } = notifyingClient((name, params) => {
      if (name !== REPORT_DELTA_METHOD) {
        // deslop/reportGet always serves canonical live truth.
        return liveReport;
      }
      // The server answers `since -> current(3)`. With the correct baseline
      // (1) it can retract "phantom"; the buggy no-since default (current-1
      // = 2) returns a delta that cannot, because phantom left in gen 2.
      const since = (params as { since_generation?: number } | undefined)?.since_generation ?? 2;
      return wireDelta({
      routing: FIXTURE_ROUTING,
        from_generation: since === 1 ? 1 : since,
        to_generation: LIVE_GENERATION,
        clusters_added: [fresh],
        clusters_removed: since === 1 ? ["phantom"] : [],
      });
    });

    const store = storeWith(report([cluster("phantom", 100, "/repo/Phantom.cs"), keep]), 1);

    await refreshAfterChange(
      client,
      store,
      changedPayload(LIVE_GENERATION, { clusters_added: 1, clusters_removed: 1, worst_mass: 80 }),
    );

    assert.deepEqual(
      store.current.report?.clusters.map((c) => c.id),
      ["fresh", "keep"],
      "the stale 'phantom' cluster (rank #1) must not survive a missed generation — " +
        "the store must converge to the live engine report",
    );
    assert.equal(store.current.generation, LIVE_GENERATION, "the store must advance to the live generation");
  });
});
