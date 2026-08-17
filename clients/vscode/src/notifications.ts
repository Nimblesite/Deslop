// LSP notification wiring for the VSIX ([VSIX-STATE], [VSIX-STATE-DIRTY]).
// Report refreshes are serialised: `deslop/reportChanged` and the
// embedding-complete refresh share one queue, so completions apply in
// dispatch order and a slow early snapshot can never land on top of a later
// one. The queue is the ordering guard; `ReportStore.setSnapshot`'s
// generation-rollback rejection stays as the store's own last line of
// defence against a stale completion.

import { LanguageClient } from "vscode-languageclient/node";

import { log, logError } from "./logging";
import { ReportStore } from "./reportStore";
import {
  AnalysisState,
  EmbeddingProgress,
  Report,
  ReportChangedNotification,
  ReportDelta,
} from "./types/report";

/**
 * Handle over the serialised refresh queue. `settled()` resolves once every
 * refresh dispatched so far has been applied (or logged as failed) — the
 * deterministic await point for tests driving notification handlers.
 */
export interface RefreshSchedule {
  settled(): Promise<void>;
}

interface RefreshQueue extends RefreshSchedule {
  enqueue(label: string, refresh: () => Promise<void>): void;
}

// One refresh at a time, in dispatch order. Failures are logged and never
// break the chain, so a single bad response cannot wedge later refreshes.
function serialRefreshQueue(): RefreshQueue {
  let tail: Promise<void> = Promise.resolve();
  return {
    enqueue(label: string, refresh: () => Promise<void>): void {
      tail = tail.then(refresh).catch((err: unknown) => logError(err, label));
    },
    settled: () => tail,
  };
}

/** Wires the deslop LSP notifications into the store. Returns the refresh schedule so callers (tests) can await queued refreshes deterministically. */
export function wireNotifications(c: LanguageClient, store: ReportStore): RefreshSchedule {
  const refreshes = serialRefreshQueue();
  c.onNotification("deslop/reportChanged", (payload: ReportChangedNotification) => {
    refreshes.enqueue("refresh report after change", () => refreshAfterChange(c, store, payload));
  });
  c.onNotification("deslop/analysisState", (state: AnalysisState) => {
    applyAnalysisState(store, state);
  });
  c.onNotification("deslop/embeddingProgress", (progress: EmbeddingProgress) => {
    if (progress.phase === "complete") {
      store.setEmbeddingProgress(null);
      refreshes.enqueue("refresh report after embedding", () => refreshAfterEmbedding(c, store));
    } else {
      store.setEmbeddingProgress(progress);
    }
  });
  return { settled: () => refreshes.settled() };
}

function applyAnalysisState(store: ReportStore, state: AnalysisState): void {
  log("analysis state", { state });
  if (state.state === "running") store.setLifecycle({ kind: "analysing" });
  else if (state.state === "idle") store.setLifecycle({ kind: "ready" });
  else if (state.state === "errored") {
    store.setLifecycle({ kind: "failed", message: state.message });
  }
}

/** Converges the store to the live engine after a `deslop/reportChanged`: delta first, full snapshot as the fallback. Exported for the test harness. */
export async function refreshAfterChange(
  c: LanguageClient,
  store: ReportStore,
  payload: ReportChangedNotification,
): Promise<void> {
  // Pull the delta spanning the store's own baseline (#230). Without a
  // `since_generation` the server defaults to `current - 1`, which only spans
  // one generation — so a client that missed a `reportChanged` (lagged
  // broadcast or async gap) would merge a delta that never retracts the
  // clusters dropped in the skipped generations, leaving them as phantoms.
  const delta = await c.sendRequest<ReportDelta | null>("deslop/reportDelta", {
    since_generation: store.current.generation,
  });
  // applyDelta rejects (returns false) when no report is seeded yet or the
  // delta's baseline does not match the store's generation. Either way fall
  // back to the full snapshot so the store converges to the live engine.
  if (delta && store.applyDelta(delta)) return;
  const snapshot = await c.sendRequest<Report>("deslop/reportGet");
  // The store refuses a generation rollback: a newer snapshot has already
  // landed, so this completion is stale and the store is already converged.
  if (!store.setSnapshot(snapshot, payload.generation)) {
    log("discarded stale report snapshot", { generation: payload.generation });
  }
}

async function refreshAfterEmbedding(c: LanguageClient, store: ReportStore): Promise<void> {
  const snapshot = await c.sendRequest<Report>("deslop/reportGet");
  store.setSnapshot(snapshot, store.current.generation + 1);
}
