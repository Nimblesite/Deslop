// Unit: LSP session restart resets the report store ([VSIX-STATE]).
// vscode-languageclient auto-restarts a crashed server, and generations
// count per server session. Without a reset, a store at generation 100
// rejects the restarted server's generation-1 snapshot through the
// rollback guard and pins the dead session's findings on every surface.
// Runs under vscode-test so the transitive `vscode` module resolves.

import * as assert from "node:assert/strict";
import { LanguageClient, State } from "vscode-languageclient/node";
import { ReportStore } from "../../reportStore";
import { refreshAfterChange, wireSessionReset } from "../../notifications";
import { Report, ReportCluster } from "../../types/report";
import { emptyReport } from "./report.helpers";

type StateChange = { oldState: State; newState: State };

/** A fake client exposing only the state-change feed the wiring consumes. */
function fakeClient(): { client: LanguageClient; fire: (change: StateChange) => void } {
  let handler: ((change: StateChange) => void) | undefined;
  const client = {
    onDidChangeState: (cb: (change: StateChange) => void) => {
      handler = cb;
      return { dispose: () => (handler = undefined) };
    },
  } as unknown as LanguageClient;
  return {
    client,
    fire: (change) => handler?.(change),
  };
}

function cluster(id: string, path: string): ReportCluster {
  return {
    id,
    weight: 10,
    size: 2,
    canonical_node_count: 40,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    bucket: "identical",
    category: "logic",
    occurrences: [
      { path, start_byte: 0, end_byte: 10, hidden: false },
      { path, start_byte: 20, end_byte: 30, hidden: false },
    ],
    summary: `cluster ${id}`,
    interpretation: "Identical code.",
  } as unknown as ReportCluster;
}

function report(clusterId: string, path: string): Report {
  return emptyReport({
    tool_version: "tool-v1",
    files_analysed: 1,
    metrics: {
      analysed_loc: 100,
      duplicated_loc: 10,
      duplication_percent: 10,
      clusters_total: 1,
      duplicated_files: 1,
      threshold: { percent: 0, breached: false, source: "none" },
      per_file: [],
    },
    clusters: [cluster(clusterId, path)],
  }) as unknown as Report;
}

const running: StateChange = { oldState: State.Starting, newState: State.Running };
const stopped: StateChange = { oldState: State.Running, newState: State.Stopped };
const starting: StateChange = { oldState: State.Stopped, newState: State.Starting };

suite("session reset on LSP restart", () => {
  test("a restarted session's generation-1 snapshot replaces the old generation-100 report", () => {
    const store = new ReportStore();
    const { client, fire } = fakeClient();
    wireSessionReset(client, store);

    fire(running); // initial session
    assert.equal(store.setSnapshot(report("old-server", "old.cs"), 100), true);
    assert.equal(store.current.generation, 100);
    assert.equal(store.current.report?.clusters[0]?.id, "old-server");
    const revisionBefore = store.current.revision;

    // Pre-fix behaviour pinned by contradiction: without the reset the
    // rollback guard rejects the new session's snapshot outright.
    fire(stopped);
    fire(starting);
    fire(running); // auto-restart → second session

    assert.equal(store.current.report, null, "old report must be cleared on restart");
    assert.equal(store.current.generation, 0, "generation must restart with the session");
    assert.equal(store.current.retractedClusters.size, 0, "retraction ledger must clear");
    assert.equal(store.current.lifecycle.kind, "analysing", "restart re-enters analysing");
    assert.equal(store.current.pendingEmbeddingModel, null);
    assert.equal(store.current.embeddingProgress, null);
    assert.equal(
      store.current.revision,
      revisionBefore + 1,
      "revision must advance (never rewind) so in-flight probes go stale",
    );

    assert.equal(
      store.setSnapshot(report("new-server", "new.cs"), 1),
      true,
      "the new session's generation-1 snapshot must be accepted",
    );
    const restored = store.current;
    assert.equal(restored.generation, 1);
    assert.equal(restored.report?.clusters[0]?.id, "new-server");
    assert.equal(restored.lifecycle.kind, "ready");
  });

  test("the initial Running transition does not clobber the activation-seeded store", () => {
    const store = new ReportStore();
    const { client, fire } = fakeClient();
    wireSessionReset(client, store);

    assert.equal(store.setSnapshot(report("seeded", "seed.cs"), 3), true);
    fire(running); // first session — no reset

    assert.equal(store.current.report?.clusters[0]?.id, "seeded");
    assert.equal(store.current.generation, 3);
  });

  test("Stopped and Starting transitions alone never reset the store", () => {
    const store = new ReportStore();
    const { client, fire } = fakeClient();
    wireSessionReset(client, store);

    fire(running);
    assert.equal(store.setSnapshot(report("held", "held.cs"), 7), true);
    fire(stopped);
    fire(starting);

    assert.equal(
      store.current.report?.clusters[0]?.id,
      "held",
      "a stopped server still shows the last report until the new session is Running",
    );
    assert.equal(store.current.generation, 7);
  });

  test("an in-flight old-session refresh resolving after the reset is inert", async () => {
    const store = new ReportStore();
    // A deferred client: the refresh's snapshot request stays pending until
    // the test settles it — after the restart reset — mimicking a dead
    // session answering late.
    let settleSnapshot: ((snapshot: Report) => void) | undefined;
    const client = {
      sendRequest: (name: string) => {
        if (name === "deslop/reportDelta") return Promise.resolve(null);
        return new Promise<Report>((resolvePromise) => {
          settleSnapshot = resolvePromise;
        });
      },
    } as unknown as LanguageClient;

    const inflight = refreshAfterChange(client, store, {
      generation: 100,
      summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0, worst_weight: 0 },
    });
    // Flush microtasks (no timers) until the delta await settles and the
    // snapshot request is dispatched — in the OLD session.
    for (let flushes = 0; flushes < 10 && !settleSnapshot; flushes += 1) await Promise.resolve();
    assert.ok(settleSnapshot, "the refresh must have dispatched its snapshot request");
    // The server restarts while the old session's answer is still pending.
    store.resetForNewSession();
    settleSnapshot?.(report("old-server", "old.cs"));
    await inflight;

    assert.equal(
      store.current.report,
      null,
      "the dead session's generation-100 snapshot must not land after the reset",
    );
    assert.equal(store.current.generation, 0, "the store stays at the new session's baseline");

    assert.equal(
      store.setSnapshot(report("new-server", "new.cs"), 1),
      true,
      "the new session's generation-1 snapshot must still be accepted",
    );
    const converged = store.current;
    assert.equal(converged.generation, 1);
    assert.equal(converged.report?.clusters[0]?.id, "new-server");
  });

  test("disposing the wiring stops restart transitions from resetting", () => {
    const store = new ReportStore();
    const { client, fire } = fakeClient();
    const wiring = wireSessionReset(client, store);

    fire(running);
    assert.equal(store.setSnapshot(report("kept", "kept.cs"), 5), true);
    wiring.dispose();
    fire(stopped);
    fire(running);

    assert.equal(store.current.report?.clusters[0]?.id, "kept");
    assert.equal(store.current.generation, 5);
  });
});
