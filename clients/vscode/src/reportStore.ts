// Holds the current report snapshot + emits change events for every surface.
// Wired to the LSP's `deslop/reportChanged` notification and seeded by `deslop/reportGet`.

import * as vscode from "vscode";
import {
  ChangeSummary,
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
}

export class ReportStore implements vscode.Disposable {
  private state: ReportState = {
    report: null,
    generation: 0,
    lifecycle: { kind: "starting" },
  };
  private readonly emitter = new vscode.EventEmitter<ReportState>();
  private readonly summaryEmitter = new vscode.EventEmitter<ChangeSummary>();

  readonly onDidChange = this.emitter.event;
  readonly onDidChangeSummary = this.summaryEmitter.event;

  get current(): ReportState {
    return this.state;
  }

  setSnapshot(report: Report, generation: number): void {
    this.state = { report, generation, lifecycle: { kind: "ready" } };
    this.emitter.fire(this.state);
  }

  applyDelta(delta: ReportDelta): void {
    const current = this.state.report;
    if (!current) return;
    const byId = new Map<string, ReportCluster>();
    for (const cluster of current.clusters) byId.set(cluster.id, cluster);
    for (const id of delta.clusters_removed) byId.delete(id);
    for (const cluster of delta.clusters_updated) byId.set(cluster.id, cluster);
    for (const cluster of delta.clusters_added) byId.set(cluster.id, cluster);
    const clusters = Array.from(byId.values()).sort((a, b) => b.weight - a.weight);
    const next: Report = {
      ...current,
      clusters,
      cache_stats: delta.cache_stats,
      tool_version: delta.tool_version,
    };
    this.state = {
      report: next,
      generation: delta.to_generation,
      lifecycle: { kind: "ready" },
    };
    this.emitter.fire(this.state);
  }

  setLifecycle(lifecycle: LifecyclePhase): void {
    this.state = { ...this.state, lifecycle };
    this.emitter.fire(this.state);
  }

  notifyChange(summary: ChangeSummary): void {
    this.summaryEmitter.fire(summary);
  }

  dispose(): void {
    this.emitter.dispose();
    this.summaryEmitter.dispose();
  }
}
