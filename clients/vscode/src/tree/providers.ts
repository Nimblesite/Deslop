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
  orderedOccurrences,
} from "./grouping";
import { buildFolderMode } from "./folder";
import { buildMetricRows } from "./metrics";
import { groupByLanguage, normalizeSplitByLanguage } from "./language";
import { normalizeSortBy, SortBy } from "./sort";
import { thresholdStatus } from "./threshold";

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

// [VSIX reactivity] The busy/error status row for any non-"ready"
// lifecycle. `hasReport` picks the scan-kind label: the initial cold
// scan has no report yet ("Scanning workspace…"), while a re-analysis
// after results exist is an incremental pass ("Analysing changes…").
// Returns null only when the server has confirmed it is idle ("ready"),
// which is the SOLE state allowed to render the terminal "No duplication
// detected" message — the panel must never declare the codebase clean
// while a scan is still in flight.
function scanStatus(
  lifecycle: LifecyclePhase,
  hasReport: boolean,
  frame: number,
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
  const label =
    lifecycle.kind === "starting"
      ? "Starting"
      : hasReport
        ? "Analysing changes"
        : "Scanning workspace";
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
  private bulkExpansion: "expand" | "collapse" | undefined;
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
    // A data change rebuilds rows at their natural collapsed/expanded defaults,
    // so any one-shot Expand-All / Collapse-All override is released here.
    this.bulkExpansion = undefined;
    this.emitter.fire();
  }

  private needsAnimation(): boolean {
    // Animate whenever the server reports work in flight, so BOTH the
    // initial cold-scan spinner and the incremental "Analysing changes…"
    // badge tick. The animation settles the moment analysisState reports
    // idle ("ready") — even if clusters are already on screen.
    const { lifecycle } = this.store.current;
    return lifecycle.kind === "starting" || lifecycle.kind === "analysing";
  }

  getTreeItem(node: Node): vscode.TreeItem {
    // [VSIX-TOP-OFFENDERS-TOOLBAR] Apply a pending Expand-All / Collapse-All to
    // every expandable row. Rows are rebuilt with fresh identity on each fire, so
    // VS Code honours the collapsible state we hand back here.
    if (this.bulkExpansion && node.collapsibleState !== vscode.TreeItemCollapsibleState.None) {
      node.collapsibleState =
        this.bulkExpansion === "expand"
          ? vscode.TreeItemCollapsibleState.Expanded
          : vscode.TreeItemCollapsibleState.Collapsed;
    }
    return node;
  }

  abstract getChildren(node?: Node): Node[];

  /** Force a tree rebuild. Used by config-driven view-state changes. */
  refresh(): void {
    this.emitter.fire();
  }

  /**
   * Expand or collapse the whole tree ([VSIX-TOP-OFFENDERS-TOOLBAR]).
   * Presentation-only: it rewrites the collapsible state the provider hands back
   * and never touches the report or the LSP ([VSIX-VIEW-STATE-UI-ONLY]). The
   * override is released on the next data change.
   */
  setBulkExpansion(mode: "expand" | "collapse"): void {
    this.bulkExpansion = mode;
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
      // [VSIX-TOP-OFFENDERS-SORT] Order occurrences by the active axis while
      // keeping each one's original index so the canonical badge stays put.
      return orderedOccurrences(node.cluster, readSortBy()).map(({ occurrence, index }) =>
        new OccurrenceNode(occurrence, node.cluster, node.rank, index),
      );
    }
    if (node) return [];
    // [VSIX-STATE-DIRTY]: tree rows render from the visible projection so a
    // file the user is mid-edit drops out instantly. The canonical report
    // gates the scan-kind label (cold vs incremental).
    const { visibleReport, report, lifecycle } = this.store.current;
    const indicator = scanStatus(lifecycle, !!report, this.ticker.currentFrame);
    if (lifecycle.kind === "failed") return indicator ? [indicator] : [];
    if (!visibleReport || visibleReport.clusters.length === 0) {
      // Never declare the codebase clean until the server confirms a
      // completed scan ("ready"); while scanning, show progress instead.
      if (indicator) return [indicator];
      return [new StatusNode("No duplication detected", "info")];
    }
    const roots = buildRoots(visibleReport.clusters);
    // [req: incremental indicator] Keep clusters visible during a
    // re-analysis (stale > blank) and lead with a busy badge so the user
    // sees an update is in flight ([VSIX-REACTIVITY-TREE]).
    return indicator ? [indicator, ...roots] : roots;
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
    return buildClusterMode(subset, severities, rankIndex, sortBy);
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
    const { visibleReport, report, lifecycle } = this.store.current;
    const indicator = scanStatus(lifecycle, !!report, this.ticker.currentFrame);
    if (lifecycle.kind === "failed") return indicator ? [indicator] : [];
    if (!visibleReport) {
      return indicator ? [indicator] : [new StatusNode("No session yet", "info")];
    }
    const metrics = visibleReport.metrics;
    if (metrics.duplicated_loc === 0) {
      // Same completion gate as Top Offenders: only the server's idle
      // state may render the terminal "clean" verdict ([VSIX reactivity]).
      if (indicator) return [indicator];
      return [new StatusNode("No duplication detected", "info")];
    }
    return [metricsHeadline(metrics), ...buildMetricRows(metrics)];
  }
}

// [VSIX-METRICS-PANEL] Headline row: repo-wide percentage + plain-English
// totals, with the configured duplication gate and an over/within verdict
// appended whenever `.deslop.toml` sets one.
function metricsHeadline(metrics: RepoMetrics): MetricsHeadlineNode {
  const detail =
    `${metrics.analysed_loc.toLocaleString()} LOC analysed · ` +
    `${metrics.duplicated_loc.toLocaleString()} duplicated · ` +
    `${metrics.clusters_total} clusters across ${metrics.duplicated_files} files`;
  return new MetricsHeadlineNode(
    metrics.duplication_percent,
    detail,
    thresholdStatus(metrics.threshold),
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
    // Show spinner only before first report arrives, or on error. Once
    // session data exists it stays visible during re-analysis (stale >
    // blank); the Top Offenders panel carries the in-flight badge.
    if (lifecycle.kind === "failed" || !report) {
      const status = scanStatus(lifecycle, !!report, this.ticker.currentFrame);
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
