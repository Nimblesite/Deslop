// Centralised VSIX state store ([VSIX-STATE]).
// Signals are the primary reactive primitive — every surface that
// shows report data derives from them via effect() or computed().
// onDidChange is a compatibility shim for places that have not yet
// migrated to effect() — it skips the initial synchronous run so
// subscriber counts in existing tests stay correct.

import * as vscode from "vscode";
import { signal, batch, effect, ReadonlySignal } from "@preact/signals-core";

import {
  ChangeSummary,
  EmbeddingProgress,
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
  report: Report | null;
  generation: number;
  lifecycle: LifecyclePhase;
  pendingEmbeddingModel: string | null;
  embeddingProgress: EmbeddingProgress | null;
}

export class ReportStore implements vscode.Disposable {
  private readonly _report = signal<Report | null>(null);
  private readonly _generation = signal<number>(0);
  private readonly _lifecycle = signal<LifecyclePhase>({ kind: "starting" });
  private readonly _pendingEmbeddingModel = signal<string | null>(null);
  private readonly _embeddingProgress = signal<EmbeddingProgress | null>(null);
  private readonly summaryEmitter = new vscode.EventEmitter<ChangeSummary>();

  /** Signal for direct use in effect() — re-renders only when the report changes. */
  readonly report: ReadonlySignal<Report | null> = this._report;
  /** Signal for direct use in effect() — re-renders only when lifecycle changes. */
  readonly lifecycle: ReadonlySignal<LifecyclePhase> = this._lifecycle;
  /** Signal for direct use in effect() — re-renders when embedding model pending changes. */
  readonly pendingEmbeddingModel: ReadonlySignal<string | null> = this._pendingEmbeddingModel;
  /** Signal for direct use in effect() — re-renders when embedding progress changes. */
  readonly embeddingProgress: ReadonlySignal<EmbeddingProgress | null> = this._embeddingProgress;

  /** One-shot summary events (not reactive state — use onDidChangeSummary for these). */
  readonly onDidChangeSummary = this.summaryEmitter.event;

  /** Snapshot of all signals. Reading inside an effect() tracks every field. */
  get current(): ReportState {
    return {
      report: this._report.value,
      generation: this._generation.value,
      lifecycle: this._lifecycle.value,
      pendingEmbeddingModel: this._pendingEmbeddingModel.value,
      embeddingProgress: this._embeddingProgress.value,
    };
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
      this._lifecycle.value = { kind: "ready" };
      this._pendingEmbeddingModel.value = null;
      this._embeddingProgress.value = null;
    });
  }

  applyDelta(delta: ReportDelta): void {
    const current = this._report.value;
    if (!current) return;
    const byId = new Map<string, ReportCluster>();
    for (const cluster of current.clusters) byId.set(cluster.id, cluster);
    for (const id of delta.clusters_removed) byId.delete(id);
    for (const cluster of delta.clusters_updated) byId.set(cluster.id, cluster);
    for (const cluster of delta.clusters_added) byId.set(cluster.id, cluster);
    const clusters = Array.from(byId.values()).sort((a, b) => b.weight - a.weight);
    batch(() => {
      this._report.value = {
        ...current,
        clusters,
        cache_stats: delta.cache_stats,
        tool_version: delta.tool_version,
      };
      this._generation.value = delta.to_generation;
      this._lifecycle.value = { kind: "ready" };
      this._pendingEmbeddingModel.value = null;
      this._embeddingProgress.value = null;
    });
  }

  markFileDirty(path: string): void {
    const current = this._report.value;
    if (!current) return;
    let changed = false;
    const clusters: ReportCluster[] = [];
    for (const cluster of current.clusters) {
      const kept = cluster.occurrences.filter((occurrence) => !sameReportFile(occurrence.path, path));
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
    if (!changed) return;
    this._report.value = {
      ...current,
      metrics: {
        ...current.metrics,
        clusters_total: clusters.length,
      },
      clusters,
    };
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

  notifyChange(summary: ChangeSummary): void {
    this.summaryEmitter.fire(summary);
  }

  dispose(): void {
    this.summaryEmitter.dispose();
  }
}

function occurrenceTotal(cluster: ReportCluster): number {
  const total =
    cluster.occurrences_total && cluster.occurrences_total > 0
      ? cluster.occurrences_total
      : cluster.size;
  return Math.max(total, cluster.occurrences.length);
}

function sameReportFile(reportPath: string, changedPath: string): boolean {
  const left = normalisePath(reportPath);
  const right = normalisePath(changedPath);
  return samePathOrSuffix(left, right) || samePathOrSuffix(right, left);
}

function samePathOrSuffix(left: string, right: string): boolean {
  return left === right || right.endsWith(`/${left}`);
}

function normalisePath(value: string): string {
  return value.replace(/\\/g, "/");
}
