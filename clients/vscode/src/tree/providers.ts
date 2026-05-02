// Tree providers for the Duplicate Clusters activity-bar container.
// Three trees per [VSIX-ACTIVITY-BAR]: Top Offenders, Focused File,
// Session panel. Top Offenders dispatches between cluster mode and
// file mode per [VSIX-TOP-OFFENDERS-GROUPING].

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore, LifecyclePhase } from "../reportStore";
import { indexedSeverity } from "../severity";
import {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  Node,
  OccurrenceNode,
  SessionFieldNode,
  StatusNode,
} from "./nodes";
import {
  buildClusterMode,
  buildFileMode,
  getBucketGroupChildren,
  getFileNodeChildren,
  GroupBy,
} from "./grouping";

// Re-export node classes so existing call sites
// (commands/register.ts, commands/treeMenus.ts, tests, e2e suites)
// keep working without import-path churn.
export {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  OccurrenceNode,
  SessionFieldNode,
  StatusNode,
} from "./nodes";

const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_INTERVAL_MS = 120;

function renderLifecycle(
  lifecycle: LifecyclePhase,
  frame: number,
  idleLabel: string,
): StatusNode | null {
  if (lifecycle.kind === "ready") return null;
  if (lifecycle.kind === "failed") {
    return new StatusNode(
      `Stopped: ${lifecycle.message}`,
      "error",
      `${lifecycle.message}\n\nClick to open the Deslop log.`,
    );
  }
  const spinner = SPINNER_FRAMES[frame % SPINNER_FRAMES.length] ?? "";
  const label = lifecycle.kind === "starting" ? "Starting" : idleLabel;
  return new StatusNode(`${spinner} ${label}…`, "busy");
}

export class StatusTicker implements vscode.Disposable {
  private readonly emitter = new vscode.EventEmitter<number>();
  readonly onTick = this.emitter.event;
  private frame = 0;
  private handle: ReturnType<typeof setInterval> | undefined;
  private subscribers = 0;

  acquire(): vscode.Disposable {
    this.subscribers += 1;
    this.handle ??= setInterval(() => {
      this.frame = (this.frame + 1) % SPINNER_FRAMES.length;
      this.emitter.fire(this.frame);
    }, SPINNER_INTERVAL_MS);
    let released = false;
    return {
      dispose: () => {
        if (released) return;
        released = true;
        this.subscribers -= 1;
        if (this.subscribers <= 0 && this.handle) {
          clearInterval(this.handle);
          this.handle = undefined;
        }
      },
    };
  }

  get currentFrame(): number {
    return this.frame;
  }

  dispose(): void {
    if (this.handle) clearInterval(this.handle);
    this.emitter.dispose();
  }
}

abstract class LifecycleAwareProvider implements vscode.TreeDataProvider<Node>, vscode.Disposable {
  protected readonly emitter = new vscode.EventEmitter<Node | undefined | void>();
  readonly onDidChangeTreeData = this.emitter.event;
  private tickerSub: vscode.Disposable | undefined;
  protected readonly disposables: vscode.Disposable[] = [];

  constructor(
    protected readonly store: ReportStore,
    protected readonly ticker: StatusTicker,
  ) {
    // effect() tracks store.lifecycle (read inside onLifecycleMaybeChanged).
    // Runs immediately for the initial state, then on every lifecycle change.
    this.disposables.push({ dispose: effect(() => this.onLifecycleMaybeChanged()) });
    this.disposables.push(ticker.onTick(() => {
      if (this.needsAnimation()) this.emitter.fire();
    }));
  }

  private onLifecycleMaybeChanged(): void {
    const needs = this.needsAnimation();
    if (needs && !this.tickerSub) this.tickerSub = this.ticker.acquire();
    if (!needs && this.tickerSub) {
      this.tickerSub.dispose();
      this.tickerSub = undefined;
    }
    this.emitter.fire();
  }

  private needsAnimation(): boolean {
    const phase = this.store.current.lifecycle.kind;
    return phase === "starting" || phase === "analysing";
  }

  getTreeItem(node: Node): vscode.TreeItem {
    return node;
  }

  abstract getChildren(node?: Node): Node[];

  /** Force a tree rebuild. Used by config-driven view-state changes. */
  refresh(): void {
    this.emitter.fire();
  }

  dispose(): void {
    this.tickerSub?.dispose();
    for (const d of this.disposables) d.dispose();
    this.emitter.dispose();
  }
}

// [VSIX-TOP-OFFENDERS-GROUPING] Reads `deslop.topOffenders.groupBy`
// (cluster | file, default cluster) and dispatches into the matching
// builder. Unknown / missing values fall back to "cluster" — never panic.
function readGroupBy(): GroupBy {
  const raw = vscode.workspace
    .getConfiguration("deslop")
    .get<string>("topOffenders.groupBy", "cluster");
  return raw === "file" ? "file" : "cluster";
}

export class TopOffendersProvider extends LifecycleAwareProvider {
  getChildren(node?: Node): Node[] {
    if (node instanceof FileNode) return getFileNodeChildren(node);
    if (node instanceof BucketGroupNode) return getBucketGroupChildren(node);
    if (node instanceof ClusterNode) {
      return node.cluster.occurrences.map((o, i) =>
        new OccurrenceNode(o, node.cluster, node.rank, i),
      );
    }
    if (node) return [];
    const { report, lifecycle } = this.store.current;
    const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
    if (status) return [status];
    if (!report || report.clusters.length === 0) {
      return [new StatusNode("No duplication detected", "info")];
    }
    const severities = indexedSeverity(report.clusters);
    return readGroupBy() === "file"
      ? buildFileMode(report.clusters, severities)
      : buildClusterMode(report.clusters, severities);
  }
}

export class FocusedFileProvider extends LifecycleAwareProvider {
  constructor(store: ReportStore, ticker: StatusTicker) {
    super(store, ticker);
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(() => this.emitter.fire()),
    );
  }

  getChildren(node?: Node): Node[] {
    if (node instanceof ClusterNode) {
      return node.cluster.occurrences.map((o, i) =>
        new OccurrenceNode(o, node.cluster, node.rank, i),
      );
    }
    if (node) return [];
    const { report, lifecycle } = this.store.current;
    if (lifecycle.kind === "failed") {
      const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
      if (status) return [status];
    }
    const editor = vscode.window.activeTextEditor;
    if (!editor) return [new StatusNode("No active editor", "info")];
    if (!report) return [];
    const activePath = editor.document.uri.fsPath;
    const overlapping = report.clusters.filter((c) =>
      c.occurrences.some((o) => sameFile(o.path, activePath)),
    );
    if (overlapping.length === 0) return [new StatusNode("No clusters in this file", "info")];
    const severities = indexedSeverity(report.clusters);
    return overlapping.map((cluster) => {
      const rank = report.clusters.findIndex((c) => c.id === cluster.id) + 1;
      const severity = severities.get(cluster.id) ?? "faint";
      return new ClusterNode(cluster, rank, severity);
    });
  }
}

export class SessionProvider extends LifecycleAwareProvider {
  constructor(
    store: ReportStore,
    ticker: StatusTicker,
    private readonly clientOf: () => LanguageClient | undefined,
  ) {
    super(store, ticker);
  }

  getChildren(node?: Node): Node[] {
    if (node) return [];
    const { report, lifecycle, pendingEmbeddingModel, embeddingProgress } =
      this.store.current;
    const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
    if (status) return [status];
    if (!report) return [new StatusNode("No session yet", "info")];
    const activeModel =
      report.embedding_provenance?.model_id ?? "Select model to enable AI matches";
    const model = pendingEmbeddingModel
      ? `${pendingEmbeddingModel} (loading…)`
      : activeModel;
    const cache = `${report.cache_stats.hits} hit / ${report.cache_stats.misses} miss`;
    const state = this.clientOf() ? "running" : "stopped";
    const rows: Node[] = [
      new SessionFieldNode("Embedding model", model, "deslop.pickEmbeddingModel"),
    ];
    if (embeddingProgress) {
      rows.push(new SessionFieldNode("Embedding", formatProgress(embeddingProgress)));
    }
    rows.push(
      new SessionFieldNode("Cache", cache),
      new SessionFieldNode("Files analysed", String(report.files_analysed)),
      new SessionFieldNode("Schema version", String(report.report_schema_version)),
      new SessionFieldNode("State", state),
    );
    return rows;
  }
}

function formatProgress(progress: {
  phase: string;
  done: number;
  total: number;
  model_id: string;
  message?: string | null;
}): string {
  const done = progress.done.toLocaleString();
  const total = progress.total.toLocaleString();
  const percent = progress.total > 0
    ? Math.floor((progress.done / progress.total) * 100)
    : 0;
  const phase = progress.phase.replace(/_/g, " ");
  const detail = progress.message ? ` · ${progress.message}` : "";
  return `${phase} · ${progress.model_id} · ${done} / ${total} (${percent}%)${detail}`;
}

function sameFile(reportPath: string, editorPath: string): boolean {
  if (reportPath === editorPath) return true;
  return editorPath.endsWith(reportPath) || reportPath.endsWith(editorPath);
}
