// Tree providers for the Duplicate Clusters activity-bar container.
// Three trees per [VSIX-ACTIVITY-BAR]: Top Offenders, Focused File, Session panel.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore, LifecyclePhase } from "../reportStore";
import { indexedSeverity, SEVERITY_DOT } from "../severity";
import {
  bucketLabels,
  ReportCluster,
  ReportOccurrence,
  resolveBucket,
  Severity,
} from "../types/report";

type Node = ClusterNode | OccurrenceNode | SessionFieldNode | StatusNode;

const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_INTERVAL_MS = 120;

class ClusterNode extends vscode.TreeItem {
  constructor(
    readonly cluster: ReportCluster,
    readonly rank: number,
    severity: Severity,
  ) {
    const labels = bucketLabels(resolveBucket(cluster));
    // Tree label is a pure-visual surface — plain title only. Tooltip
    // is shared-text (copyable, AI-scrapable on hover-extract), so it
    // carries the hybrid form with bracketed Type-N.
    super(
      `#${rank} ${SEVERITY_DOT[severity]} ${labels.plainTitle}`,
      vscode.TreeItemCollapsibleState.Collapsed,
    );
    this.description = cluster.id;
    this.contextValue = "deslop.cluster";
    this.tooltip = new vscode.MarkdownString(
      `**${labels.hybridTitle}** — ${labels.actionSentence}\n\n` +
        `weight: \`${cluster.weight.toFixed(2)}\` · size: \`${cluster.size}\` · copies: \`${cluster.occurrences.length}\``,
    );
    this.command = {
      command: "deslop.openCluster",
      title: "Open cluster",
      arguments: [cluster.id],
    };
  }
}

class OccurrenceNode extends vscode.TreeItem {
  constructor(readonly occurrence: ReportOccurrence) {
    const location = occurrenceLocation(occurrence);
    super(location?.label ?? occurrence.path, vscode.TreeItemCollapsibleState.None);
    if (location) this.description = location.description;
    this.contextValue = "deslop.occurrence";
    this.command = {
      command: "deslop.openOccurrence",
      title: location?.commandTitle ?? "Open occurrence",
      arguments: [occurrence],
    };
  }
}

class SessionFieldNode extends vscode.TreeItem {
  constructor(label: string, value: string, commandId?: string) {
    super(label, vscode.TreeItemCollapsibleState.None);
    this.description = value;
    if (commandId) {
      this.command = { command: commandId, title: label };
    }
  }
}

class StatusNode extends vscode.TreeItem {
  constructor(
    message: string,
    kind: "info" | "busy" | "error",
    tooltip?: string,
  ) {
    super(message, vscode.TreeItemCollapsibleState.None);
    this.contextValue = `deslop.status.${kind}`;
    if (kind === "busy") {
      this.iconPath = new vscode.ThemeIcon("sync~spin");
    } else if (kind === "error") {
      this.iconPath = new vscode.ThemeIcon(
        "error",
        new vscode.ThemeColor("errorForeground"),
      );
      this.command = {
        command: "deslop.revealLog",
        title: "Reveal Deslop log",
      };
    }
    if (tooltip) this.tooltip = tooltip;
  }
}

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
    this.disposables.push(store.onDidChange(() => this.onLifecycleMaybeChanged()));
    this.disposables.push(ticker.onTick(() => {
      if (this.needsAnimation()) this.emitter.fire();
    }));
    this.onLifecycleMaybeChanged();
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

  dispose(): void {
    this.tickerSub?.dispose();
    for (const d of this.disposables) d.dispose();
    this.emitter.dispose();
  }
}

export class TopOffendersProvider extends LifecycleAwareProvider {
  getChildren(node?: Node): Node[] {
    if (node instanceof ClusterNode) {
      return node.cluster.occurrences.map((o) => new OccurrenceNode(o));
    }
    if (node) return [];
    const { report, lifecycle } = this.store.current;
    const status = renderLifecycle(lifecycle, this.ticker.currentFrame, "Analysing");
    if (status) return [status];
    if (!report || report.clusters.length === 0) {
      return [new StatusNode("No duplication detected", "info")];
    }
    const severities = indexedSeverity(report.clusters);
    return report.clusters.map((cluster, i) => {
      const severity = severities.get(cluster.id) ?? "faint";
      return new ClusterNode(cluster, i + 1, severity);
    });
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
      return node.cluster.occurrences.map((o) => new OccurrenceNode(o));
    }
    if (node) return [];
    const editor = vscode.window.activeTextEditor;
    if (!editor) return [new StatusNode("No active editor", "info")];
    const report = this.store.current.report;
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

interface OccurrenceLocation {
  label: string;
  description: string;
  commandTitle: string;
}

function occurrenceLocation(occurrence: ReportOccurrence): OccurrenceLocation | undefined {
  const source = readOccurrenceSource(occurrence.path);
  if (!source) return undefined;
  const position = positionForByte(source, occurrence.start_byte);
  const label = `${occurrence.path}:${position.line}:${position.column}`;
  return {
    label,
    description: `line ${position.line}, column ${position.column}`,
    commandTitle: `Open ${path.basename(occurrence.path)} at ${position.line}:${position.column}`,
  };
}

function readOccurrenceSource(occurrencePath: string): string | undefined {
  const fsPath = resolveOccurrencePath(occurrencePath);
  try {
    return fs.readFileSync(fsPath, "utf8");
  } catch {
    return undefined;
  }
}

function resolveOccurrencePath(occurrencePath: string): string {
  if (path.isAbsolute(occurrencePath)) return occurrencePath;
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return root ? path.join(root, occurrencePath) : occurrencePath;
}

function positionForByte(source: string, byte: number): { line: number; column: number } {
  const buffer = Buffer.from(source, "utf8");
  const safeByte = Math.min(Math.max(byte, 0), buffer.length);
  const prefix = buffer.slice(0, safeByte).toString("utf8");
  const line = prefix.split("\n").length;
  const lastNewline = prefix.lastIndexOf("\n");
  const columnOffset = lastNewline === -1 ? prefix.length : prefix.length - lastNewline - 1;
  return { line, column: columnOffset + 1 };
}

function sameFile(reportPath: string, editorPath: string): boolean {
  if (reportPath === editorPath) return true;
  return editorPath.endsWith(reportPath) || reportPath.endsWith(editorPath);
}
