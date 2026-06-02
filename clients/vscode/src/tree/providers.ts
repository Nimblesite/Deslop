// Tree providers for the Duplicate Clusters activity-bar container.
// Three trees per [VSIX-ACTIVITY-BAR]: Top Offenders, Focused File,
// Session panel. Top Offenders dispatches between cluster mode and
// file mode per [VSIX-TOP-OFFENDERS-GROUPING].

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore, LifecyclePhase } from "../reportStore";
import { indexedSeverity } from "../severity";
import { ReportCluster, RepoMetrics } from "../types/report";
import {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  FolderMetricNode,
  FolderNode,
  LanguageGroupNode,
  MetricsHeadlineNode,
  Node,
  OccurrenceNode,
  SessionFieldNode,
  StatusNode,
} from "./nodes";
import {
  buildClusterMode,
  buildFileMode,
  buildRankIndex,
  getBucketGroupChildren,
  getFileNodeChildren,
  GroupBy,
} from "./grouping";
import { buildFolderMode } from "./folder";
import { buildMetricRows } from "./metrics";
import { groupByLanguage, normalizeSplitByLanguage } from "./language";
import { normalizeSortBy, SortBy } from "./sort";

// Re-export node classes so existing call sites
// (commands/register.ts, commands/treeMenus.ts, tests, e2e suites)
// keep working without import-path churn.
export {
  BucketGroupNode,
  ClusterNode,
  FileMetricNode,
  FileNode,
  FolderMetricNode,
  FolderNode,
  LanguageGroupNode,
  MetricsHeadlineNode,
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
    // Spinner gating reads canonical: animate while no LSP report exists yet,
    // regardless of dirty-set masking. The visible projection only matters
    // for the row content, not for whether the data has loaded.
    const { lifecycle, report } = this.store.current;
    return (lifecycle.kind === "starting" || lifecycle.kind === "analysing") && !report;
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
// (cluster | file | folder, default cluster). Unknown / missing values
// fall back to "cluster" — never panic.
function readGroupBy(): GroupBy {
  const raw = vscode.workspace
    .getConfiguration("deslop")
    .get<string>("topOffenders.groupBy", "cluster");
  if (raw === "file") return "file";
  if (raw === "folder") return "folder";
  return "cluster";
}

// [VSIX-TOP-OFFENDERS-SORT] Reads `deslop.topOffenders.sortBy`.
function readSortBy(): SortBy {
  return normalizeSortBy(
    vscode.workspace.getConfiguration("deslop").get<string>("topOffenders.sortBy", "impact"),
  );
}

// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] Reads `deslop.topOffenders.splitByLanguage`.
function readSplitByLanguage(): boolean {
  return normalizeSplitByLanguage(
    vscode.workspace
      .getConfiguration("deslop")
      .get<boolean>("topOffenders.splitByLanguage", false),
  );
}

export class TopOffendersProvider extends LifecycleAwareProvider {
  getChildren(node?: Node): Node[] {
    if (node instanceof FolderNode || node instanceof LanguageGroupNode) return node.children;
    if (node instanceof FileNode) return getFileNodeChildren(node);
    if (node instanceof BucketGroupNode) return getBucketGroupChildren(node);
    if (node instanceof ClusterNode) {
      return node.cluster.occurrences.map((o, i) =>
        new OccurrenceNode(o, node.cluster, node.rank, i),
      );
    }
    if (node) return [];
    // [VSIX-STATE-DIRTY]: tree rows render from the visible projection so a
    // file the user is mid-edit drops out instantly. Lifecycle still gates
    // the spinner via the canonical signal in needsAnimation().
    const { visibleReport, lifecycle } = this.store.current;
    // Show spinner only before first report arrives, or on error. During
    // re-analysis the existing report stays visible — stale > blank.
    if (lifecycle.kind === "failed" || !visibleReport) {
      const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
      if (status) return [status];
    }
    if (!visibleReport || visibleReport.clusters.length === 0) {
      return [new StatusNode("No duplication detected", "info")];
    }
    return buildRoots(visibleReport.clusters);
  }

  // [VSIX-TOP-OFFENDERS-TOOLBAR] Required by TreeView.reveal for the
  // Expand All toolbar action. Only roots are ever revealed (their
  // parent is the implicit root), so returning undefined is sufficient.
  getParent(): Node | undefined {
    return undefined;
  }
}

// [VSIX-TOP-OFFENDERS-GROUPING] Builds the root rows: dispatches on the
// grouping mode + sort axis, then wraps in per-language groups when the
// split is on. Global rank is precomputed once so it never re-numbers.
function buildRoots(clusters: ReportCluster[]): Node[] {
  const rankIndex = buildRankIndex(clusters);
  const severities = indexedSeverity(clusters);
  const groupBy = readGroupBy();
  const sortBy = readSortBy();
  const build = (subset: ReportCluster[]): Node[] => {
    if (groupBy === "file") return buildFileMode(subset, severities, rankIndex, sortBy);
    if (groupBy === "folder") return buildFolderMode(subset, severities, rankIndex, sortBy);
    return buildClusterMode(subset, severities, rankIndex);
  };
  if (!readSplitByLanguage()) return build(clusters);
  return groupByLanguage(clusters).map(({ language, clusters: members }) =>
    new LanguageGroupNode(
      language,
      build(members),
      members.reduce((max, cluster) => Math.max(max, cluster.weight), 0),
      members.length,
    ),
  );
}

// [VSIX-METRICS-PANEL] The Duplication panel — replaces the former
// Focused File tree. A headline duplication score over the whole corpus
// plus a per-folder/per-file breakdown. Renders from the visible
// projection's repo metrics ([METRICS-REPO]); refreshes on every report
// change. Folder rows expand to their dup-bearing files.
export class MetricsProvider extends LifecycleAwareProvider {
  getChildren(node?: Node): Node[] {
    if (node instanceof FolderMetricNode) return node.children;
    if (node) return [];
    const { visibleReport, lifecycle } = this.store.current;
    if (lifecycle.kind === "failed" || !visibleReport) {
      const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
      if (status) return [status];
    }
    if (!visibleReport) return [new StatusNode("No session yet", "info")];
    const metrics = visibleReport.metrics;
    if (metrics.duplicated_loc === 0) {
      return [new StatusNode("No duplication detected", "info")];
    }
    return [metricsHeadline(metrics), ...buildMetricRows(metrics)];
  }
}

// [VSIX-METRICS-PANEL] Headline row: repo-wide percentage + plain-English
// totals, with a threshold-breach warning when the gate is crossed.
function metricsHeadline(metrics: RepoMetrics): MetricsHeadlineNode {
  const detail =
    `${metrics.analysed_loc.toLocaleString()} LOC analysed · ` +
    `${metrics.duplicated_loc.toLocaleString()} duplicated · ` +
    `${metrics.clusters_total} clusters across ${metrics.duplicated_files} files`;
  return new MetricsHeadlineNode(
    metrics.duplication_percent,
    detail,
    metrics.threshold.breached,
    `${metrics.threshold.percent.toFixed(1)}% gate`,
  );
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
    // Show spinner only before first report arrives, or on error. During
    // re-analysis the existing session data stays visible — stale > blank.
    if (lifecycle.kind === "failed" || !report) {
      const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
      if (status) return [status];
    }
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
  message: string | undefined;
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
