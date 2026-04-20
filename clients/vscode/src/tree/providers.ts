// Tree providers for the Duplicate Clusters activity-bar container.
// Three trees per [VSIX-ACTIVITY-BAR]: Top Offenders, Focused File, Session panel.

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore } from "../reportStore";
import { indexedSeverity, SEVERITY_DOT } from "../severity";
import { ReportCluster, ReportOccurrence, Severity } from "../types/report";

type Node = ClusterNode | OccurrenceNode | SessionFieldNode | EmptyNode;

class ClusterNode extends vscode.TreeItem {
  constructor(
    readonly cluster: ReportCluster,
    readonly rank: number,
    severity: Severity,
  ) {
    super(
      `#${rank} ${SEVERITY_DOT[severity]} ${cluster.interpretation}`,
      vscode.TreeItemCollapsibleState.Collapsed,
    );
    this.description = cluster.id;
    this.contextValue = "codededup.cluster";
    this.tooltip = new vscode.MarkdownString(
      `**${cluster.interpretation}**\n\n` +
        `weight: \`${cluster.weight.toFixed(2)}\` · size: \`${cluster.size}\` · copies: \`${cluster.occurrences.length}\``,
    );
    this.command = {
      command: "codededup.openCluster",
      title: "Open cluster",
      arguments: [cluster.id],
    };
  }
}

class OccurrenceNode extends vscode.TreeItem {
  constructor(readonly occurrence: ReportOccurrence) {
    super(occurrence.path, vscode.TreeItemCollapsibleState.None);
    this.description = `${occurrence.start_byte}..${occurrence.end_byte}`;
    this.contextValue = "codededup.occurrence";
    this.command = {
      command: "codededup.openOccurrence",
      title: "Open occurrence",
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

class EmptyNode extends vscode.TreeItem {
  constructor(message: string) {
    super(message, vscode.TreeItemCollapsibleState.None);
    this.contextValue = "codededup.empty";
  }
}

export class TopOffendersProvider implements vscode.TreeDataProvider<Node> {
  private readonly emitter = new vscode.EventEmitter<Node | undefined | void>();
  readonly onDidChangeTreeData = this.emitter.event;

  constructor(private readonly store: ReportStore) {
    store.onDidChange(() => this.emitter.fire());
  }

  getTreeItem(node: Node): vscode.TreeItem {
    return node;
  }

  getChildren(node?: Node): Node[] {
    const report = this.store.current.report;
    if (!report) return [new EmptyNode("Analysing…")];
    if (!node) {
      if (report.clusters.length === 0) return [new EmptyNode("No duplication detected")];
      const severities = indexedSeverity(report.clusters);
      return report.clusters.map((cluster, i) => {
        const severity = severities.get(cluster.id) ?? "faint";
        return new ClusterNode(cluster, i + 1, severity);
      });
    }
    if (node instanceof ClusterNode) {
      return node.cluster.occurrences.map((o) => new OccurrenceNode(o));
    }
    return [];
  }
}

export class FocusedFileProvider implements vscode.TreeDataProvider<Node> {
  private readonly emitter = new vscode.EventEmitter<Node | undefined | void>();
  readonly onDidChangeTreeData = this.emitter.event;

  constructor(private readonly store: ReportStore) {
    store.onDidChange(() => this.emitter.fire());
    vscode.window.onDidChangeActiveTextEditor(() => this.emitter.fire());
  }

  getTreeItem(node: Node): vscode.TreeItem {
    return node;
  }

  getChildren(node?: Node): Node[] {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return [new EmptyNode("No active editor")];
    const report = this.store.current.report;
    if (!report) return [];
    const activePath = editor.document.uri.fsPath;
    const overlapping = report.clusters.filter((c) =>
      c.occurrences.some((o) => sameFile(o.path, activePath)),
    );
    if (!node) {
      if (overlapping.length === 0) return [new EmptyNode("No clusters in this file")];
      const severities = indexedSeverity(report.clusters);
      return overlapping.map((cluster) => {
        const rank = report.clusters.findIndex((c) => c.id === cluster.id) + 1;
        const severity = severities.get(cluster.id) ?? "faint";
        return new ClusterNode(cluster, rank, severity);
      });
    }
    if (node instanceof ClusterNode) {
      return node.cluster.occurrences.map((o) => new OccurrenceNode(o));
    }
    return [];
  }
}

export class SessionProvider implements vscode.TreeDataProvider<Node> {
  private readonly emitter = new vscode.EventEmitter<Node | undefined | void>();
  readonly onDidChangeTreeData = this.emitter.event;

  constructor(
    private readonly store: ReportStore,
    private readonly clientOf: () => LanguageClient | undefined,
  ) {
    store.onDidChange(() => this.emitter.fire());
  }

  getTreeItem(node: Node): vscode.TreeItem {
    return node;
  }

  getChildren(node?: Node): Node[] {
    if (node) return [];
    const report = this.store.current.report;
    if (!report) return [new EmptyNode("No session yet")];
    const model = report.embedding_provenance?.model_id ?? "off";
    const cache = `${report.cache_stats.hits} hit / ${report.cache_stats.misses} miss`;
    const state = this.clientOf() ? "running" : "stopped";
    return [
      new SessionFieldNode("Embedding model", model, "codededup.pickEmbeddingModel"),
      new SessionFieldNode("Cache", cache),
      new SessionFieldNode("Files analysed", String(report.files_analysed)),
      new SessionFieldNode("Schema version", String(report.report_schema_version)),
      new SessionFieldNode("State", state),
    ];
  }
}

function sameFile(reportPath: string, editorPath: string): boolean {
  if (reportPath === editorPath) return true;
  return editorPath.endsWith(reportPath) || reportPath.endsWith(editorPath);
}
