// Centralised VSIX state store ([VSIX-STATE]).
// Implements [PRINCIPLES-LIVE-IS-REACTIVE]: signals are the primary reactive
// primitive — every surface that shows report data derives from them via
// effect() or computed().
// onDidChange is a compatibility shim for places that have not yet
// migrated to effect() — it skips the initial synchronous run so
// subscriber counts in existing tests stay correct.
//
// The store exposes two views of the same data ([VSIX-STATE-DIRTY]):
//   * report          — canonical truth from the LSP. Only setSnapshot /
//                       applyDelta (driven by deslop/reportChanged) write it.
//                       Lookup by cluster id is always honoured here so
//                       commands like compareWithCanonical, openCluster,
//                       openOccurrence keep working through unsaved edits.
//   * visibleReport   — derived projection. For each file the user has
//                       edited locally (markFileDirty), occurrences in that
//                       file are elided. Clusters whose visible occurrence
//                       count drops below two are dropped from this view
//                       (#117 — a one-copy "top offender" is a contradiction).
//                       Surfaces — tree providers, decorations, bubbles,
//                       hovers, code lenses, status bar, session panel,
//                       webviews — render from this projection.

import * as vscode from "vscode";
import { signal, batch, effect, computed, ReadonlySignal } from "@preact/signals-core";

import {
  EmbeddingProgress,
  FacetFilter,
  ReportCluster,
  Report,
  ReportDelta,
} from "./types/report";

export type LifecyclePhase =
  | { kind: "starting" }
  | { kind: "analysing" }
  | { kind: "ready" }
  | { kind: "failed"; message: string };

export interface ReportState {
  /** Canonical report — what the LSP last published. Use for cluster-id lookups in commands. */
  report: Report | null;
  /** Visible projection — canonical with dirty-file occurrences elided. Use for any rendered surface. */
  visibleReport: Report | null;
  generation: number;
  lifecycle: LifecyclePhase;
  pendingEmbeddingModel: string | null;
  embeddingProgress: EmbeddingProgress | null;
  /** Workspace facet filter mirrored from configuration
   * ([FACET-TOP-OFFENDERS-FILTER]) so the status bar and webviews slice
   * in lock-step with the tree. */
  facetFilter: FacetFilter;
  /** Cluster ids a delta retracted since the last full snapshot. See
   * `ReportStore.retractedClusters`. */
  retractedClusters: ReadonlySet<string>;
}

/**
 * Retractions kept before the oldest are dropped.
 *
 * Sized for concurrency, not for history: at most one probe is in flight per
 * edit and the debounce coalesces edits, so a handful of generations' worth of
 * removals is already far more than any live response can refer to. The cap
 * exists so a long delta-only session cannot accumulate ids without bound.
 */
const MAX_RETRACTED_CLUSTERS = 256;

export class ReportStore implements vscode.Disposable {
  private readonly _report = signal<Report | null>(null);
  private readonly _dirtyFiles = signal<ReadonlySet<string>>(new Set());
  private readonly _generation = signal<number>(0);
  private readonly _lifecycle = signal<LifecyclePhase>({ kind: "starting" });
  private readonly _pendingEmbeddingModel = signal<string | null>(null);
  private readonly _embeddingProgress = signal<EmbeddingProgress | null>(null);
  private readonly _facetFilter = signal<FacetFilter>({ buckets: [], categories: [] });
  private readonly _retractedClusters = signal<ReadonlySet<string>>(new Set());

  private readonly _visibleReport: ReadonlySignal<Report | null> = computed(() =>
    projectVisible(this._report.value, this._dirtyFiles.value),
  );

  /** Canonical report signal — only the LSP writes this via setSnapshot / applyDelta. */
  readonly report: ReadonlySignal<Report | null> = this._report;
  /** Visible projection signal — surfaces render from this so unsaved edits hide stale rows. */
  readonly visibleReport: ReadonlySignal<Report | null> = this._visibleReport;

  /** Workspace facet-filter signal ([FACET-TOP-OFFENDERS-FILTER]). */
  readonly facetFilter: ReadonlySignal<FacetFilter> = this._facetFilter;
  /**
   * Cluster ids a delta explicitly retracted since the last full snapshot
   * ([VSIX-STATE-DIRTY]). Surfaces that may render a cluster the canonical
   * report does not contain — the live bubble accepts LSP probe results
   * found between rescans — need to tell two populations apart: a cluster
   * the report has *never seen* (legitimate, the probe is the only evidence
   * there is) from one the server *withdrew* (a phantom; re-rendering it
   * paints back what the delta just cleared). Absence from `report.clusters`
   * cannot distinguish them, so the retraction is recorded instead of
   * being dropped on the floor.
   */
  readonly retractedClusters: ReadonlySignal<ReadonlySet<string>> = this._retractedClusters;
  /** Signal for direct use in effect() — re-renders only when lifecycle changes. */
  readonly lifecycle: ReadonlySignal<LifecyclePhase> = this._lifecycle;
  /** Signal for direct use in effect() — re-renders when embedding model pending changes. */
  readonly pendingEmbeddingModel: ReadonlySignal<string | null> = this._pendingEmbeddingModel;
  /** Signal for direct use in effect() — re-renders when embedding progress changes. */
  readonly embeddingProgress: ReadonlySignal<EmbeddingProgress | null> = this._embeddingProgress;

  /** Snapshot of all signals. Reading inside an effect() tracks every field. */
  get current(): ReportState {
    return {
      report: this._report.value,
      visibleReport: this._visibleReport.value,
      generation: this._generation.value,
      lifecycle: this._lifecycle.value,
      pendingEmbeddingModel: this._pendingEmbeddingModel.value,
      embeddingProgress: this._embeddingProgress.value,
      facetFilter: this._facetFilter.value,
      retractedClusters: this._retractedClusters.value,
    };
  }

  /** Mirrors the persisted facet filter into the signal graph. Called
   * by extension.ts on activation and on configuration changes. */
  setFacetFilter(filter: FacetFilter): void {
    this._facetFilter.value = filter;
  }

  /**
   * Compatibility shim — fires `cb` whenever any signal changes.
   * Skips the initial synchronous run so callers see exactly one
   * notification per mutation, matching the old EventEmitter contract.
   *
   * Prefer `effect(() => { void store.current; … })` for new code.
   */
  onDidChange(cb: (state: ReportState) => void): vscode.Disposable {
    let skip = true;
    const unsub = effect(() => {
      const state = this.current;
      if (skip) {
        skip = false;
        return;
      }
      cb(state);
    });
    return { dispose: unsub };
  }

  setSnapshot(report: Report, generation: number): void {
    batch(() => {
      this._report.value = report;
      this._generation.value = generation;
      // A full snapshot re-states the whole corpus, so every earlier
      // retraction is settled by it — a cluster still absent here is
      // absent on the snapshot's own authority, not on a stale delta's.
      this._retractedClusters.value = new Set();
      // A report with findings is self-evidently a completed analysis, so
      // it settles the lifecycle to "ready". An EMPTY report is ambiguous
      // — it may be a cache seed or a mid-scan snapshot — so the "ready"
      // verdict is deferred to the server's analysisState idle signal.
      // This is what stops the panel declaring "No duplication detected"
      // while a scan is still running ([VSIX reactivity]).
      if (report.clusters.length > 0) this._lifecycle.value = { kind: "ready" };
      this._pendingEmbeddingModel.value = null;
      this._embeddingProgress.value = null;
    });
  }

  /**
   * Merges a delta onto the canonical report. Returns `false` without mutating
   * when the delta cannot be applied safely — no seeded report, or the delta's
   * `from_generation` does not match the store's current generation (#230). A
   * mismatch means the client missed a `reportChanged` (lagged broadcast or an
   * async gap), so the delta only carries the changes for a window the store
   * never reached; merging it would leave clusters dropped in the skipped
   * generations behind as phantoms. The caller falls back to a full snapshot.
   */
  /** Drops the oldest retractions once the ledger exceeds its cap. */
  private pruneRetracted(retracted: Set<string>): void {
    const excess = retracted.size - MAX_RETRACTED_CLUSTERS;
    if (excess <= 0) return;
    // Set iteration is insertion-ordered, so this drops the oldest first.
    for (const id of Array.from(retracted).slice(0, excess)) retracted.delete(id);
  }

  applyDelta(delta: ReportDelta): boolean {
    const current = this._report.value;
    if (!current) return false;
    if (delta.from_generation !== this._generation.value) return false;
    const byId = new Map<string, ReportCluster>();
    for (const cluster of current.clusters) byId.set(cluster.id, cluster);
    for (const id of delta.clusters_removed) byId.delete(id);
    for (const cluster of delta.clusters_updated) byId.set(cluster.id, cluster);
    for (const cluster of delta.clusters_added) byId.set(cluster.id, cluster);
    const clusters = Array.from(byId.values()).sort((a, b) => b.weight - a.weight);
    const retracted = new Set(this._retractedClusters.value);
    for (const id of delta.clusters_removed) retracted.add(id);
    // A later generation that re-states a cluster un-retracts it: the
    // server has found it again, so the withdrawal no longer stands.
    for (const cluster of delta.clusters_updated) retracted.delete(cluster.id);
    for (const cluster of delta.clusters_added) retracted.delete(cluster.id);
    // Bounded on purpose. A retraction only has to outlive the probes that
    // were already in flight when it happened; correctness for anything older
    // is `LiveBubble.hasMovedOn`, which compares the report generation and so
    // cannot be defeated by a pruned id. Keeping the full history instead made
    // a delta-only session — one that never receives a full snapshot to clear
    // the ledger — retain O(N) ids and copy O(N²) of them over its lifetime.
    this.pruneRetracted(retracted);
    batch(() => {
      this._retractedClusters.value = retracted;
      this._report.value = {
        ...current,
        clusters,
        // The server recomputes metrics every generation and ships them
        // on the delta (#199). Without this the DUPLICATION headline +
        // per-file rows freeze at the seed snapshot, since the delta path
        // is the one almost always taken after the first report.
        metrics: delta.metrics,
        cache_stats: delta.cache_stats,
        tool_version: delta.tool_version,
      };
      this._generation.value = delta.to_generation;
      // Same rule as setSnapshot: only a non-empty result settles the
      // lifecycle; an emptied report waits for the server's idle signal.
      if (clusters.length > 0) this._lifecycle.value = { kind: "ready" };
      this._pendingEmbeddingModel.value = null;
      this._embeddingProgress.value = null;
    });
    return true;
  }

  /**
   * Mark a file as having unsaved local edits. Updates the dirty set only —
   * the canonical report is untouched ([VSIX-STATE-DIRTY]). The visible
   * projection recomputes through `visibleReport`, eliding occurrences in
   * this file and any cluster that drops below two visible occurrences.
   */
  markFileDirty(path: string): void {
    const key = normalisePath(path);
    if (this._dirtyFiles.value.has(key)) return;
    const next = new Set(this._dirtyFiles.value);
    next.add(key);
    this._dirtyFiles.value = next;
  }

  /**
   * Drop a file from the dirty set. Wired to `onDidSaveTextDocument` and to
   * the LSP's external-change watcher: once the file's bytes match what the
   * LSP last analysed, the visible projection should mirror the canonical
   * report again ([VSIX-STATE-DIRTY]).
   */
  clearFileDirty(path: string): void {
    const key = normalisePath(path);
    if (!this._dirtyFiles.value.has(key)) return;
    const next = new Set(this._dirtyFiles.value);
    next.delete(key);
    this._dirtyFiles.value = next;
  }

  setLifecycle(lifecycle: LifecyclePhase): void {
    this._lifecycle.value = lifecycle;
  }

  setPendingEmbeddingModel(modelId: string | null): void {
    this._pendingEmbeddingModel.value = modelId;
  }

  setEmbeddingProgress(progress: EmbeddingProgress | null): void {
    this._embeddingProgress.value = progress;
  }

  dispose(): void {
  }
}

function projectVisible(canonical: Report | null, dirty: ReadonlySet<string>): Report | null {
  if (!canonical) return null;
  if (dirty.size === 0) return canonical;
  let changed = false;
  const clusters: ReportCluster[] = [];
  for (const cluster of canonical.clusters) {
    const kept = cluster.occurrences.filter((occurrence) => !occurrenceIsDirty(occurrence.path, dirty));
    const removed = cluster.occurrences.length - kept.length;
    if (removed === 0) {
      clusters.push(cluster);
      continue;
    }
    changed = true;
    if (kept.length < 2) continue;
    const oldTotal = occurrenceTotal(cluster);
    const nextTotal = Math.max(kept.length, oldTotal - removed);
    clusters.push({
      ...cluster,
      size: nextTotal,
      occurrences: kept,
      ...(cluster.occurrences_total !== undefined && { occurrences_total: nextTotal }),
    });
  }
  if (!changed) return canonical;
  return {
    ...canonical,
    metrics: {
      ...canonical.metrics,
      clusters_total: clusters.length,
    },
    clusters,
  };
}

function occurrenceIsDirty(occurrencePath: string, dirty: ReadonlySet<string>): boolean {
  const left = normalisePath(occurrencePath);
  for (const dirtyPath of dirty) {
    if (samePathOrSuffix(left, dirtyPath) || samePathOrSuffix(dirtyPath, left)) return true;
  }
  return false;
}

function occurrenceTotal(cluster: ReportCluster): number {
  const total =
    cluster.occurrences_total && cluster.occurrences_total > 0
      ? cluster.occurrences_total
      : cluster.size;
  return Math.max(total, cluster.occurrences.length);
}

function samePathOrSuffix(left: string, right: string): boolean {
  return left === right || right.endsWith(`/${left}`);
}

function normalisePath(value: string): string {
  return value.replace(/\\/g, "/");
}
