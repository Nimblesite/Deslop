// Tree node classes for the Duplicate Clusters activity-bar container.
// Behavioural shape and label rules are spec'd in
// docs/specs/vsix.md under [VSIX-TOP-OFFENDERS-GROUPING],
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE],
// and [VSIX-TOP-OFFENDERS-RANK-GLOBAL].

import * as vscode from "vscode";

import { clusterSlug } from "../clusterHover";
import { occurrenceDisplayLocation } from "../locations";
import { SEVERITY_DOT } from "../severity";
import {
  Bucket,
  bucketLabels,
  occurrenceCount,
  ReportCluster,
  ReportOccurrence,
  resolveBucket,
  Severity,
} from "../types/report";

export type Node =
  | ClusterNode
  | OccurrenceNode
  | FileNode
  | BucketGroupNode
  | SessionFieldNode
  | StatusNode;

// [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Category colour is metadata
// backed by text/a11y labels, never the only signal.
export const CATEGORY_STYLE: Record<Bucket, { icon: string; color: string }> = {
  identical: { icon: "circle-filled", color: "charts.red" },
  nearly_identical: { icon: "circle-large-filled", color: "charts.orange" },
  loosely_similar: { icon: "circle-outline", color: "charts.blue" },
  same_behavior: { icon: "sparkle", color: "charts.purple" },
};

export function categoryIcon(bucket: Bucket): vscode.ThemeIcon {
  const style = CATEGORY_STYLE[bucket];
  return new vscode.ThemeIcon(style.icon, new vscode.ThemeColor(style.color));
}

export function representativePath(cluster: ReportCluster): string {
  return cluster.occurrences[0]?.path ?? cluster.id;
}

export function displayPath(filePath: string): string {
  if (!filePath) return "unknown file";
  return vscode.workspace.asRelativePath(filePath, false);
}

// [VSIX-TOP-OFFENDERS-CLUSTER-ID] The bold label leads with the cluster's
// stable slug (shared with the hover bubble via `clusterSlug`) — rank #N
// is volatile (re-numbered on every snapshot) and would mislead humans
// and AI consumers if it took the id slot.
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] / [VSIX-TOP-OFFENDERS-FILE-MODE]
// File mode passes `file: undefined` so the redundant `· <file>`
// suffix is dropped under a parent FileNode; cluster mode passes the
// display path. Tooltip is built separately and stays mode-invariant.
export function clusterRowLabel(args: {
  slug: string;
  severity: Severity;
  bucket: Bucket;
  file?: string;
}): string {
  const labels = bucketLabels(args.bucket);
  const head = `${args.slug} ${SEVERITY_DOT[args.severity]} ${labels.plainTitle}`;
  return args.file ? `${head} · ${args.file}` : head;
}

export interface ClusterNodeOptions {
  showFile?: boolean;
}

export class ClusterNode extends vscode.TreeItem {
  constructor(
    readonly cluster: ReportCluster,
    readonly rank: number,
    severity: Severity,
    options: ClusterNodeOptions = {},
  ) {
    const bucket = resolveBucket(cluster);
    const labels = bucketLabels(bucket);
    const filePath = representativePath(cluster);
    const fileLabel = displayPath(filePath);
    const showFile = options.showFile ?? true;
    const slug = clusterSlug(cluster);
    const labelArgs = showFile
      ? { slug, severity, bucket, file: fileLabel }
      : { slug, severity, bucket };
    super(clusterRowLabel(labelArgs), vscode.TreeItemCollapsibleState.Collapsed);
    this.description = `rank #${rank} · ${occurrenceCount(cluster)} copies`;
    this.contextValue =
      occurrenceCount(cluster) > 1 ? "deslop.clusterComparable" : "deslop.clusterSingle";
    this.iconPath = categoryIcon(bucket);
    this.accessibilityInformation = {
      label: `${labels.plainTitle} in ${fileLabel}, cluster ${cluster.id}, rank ${rank}`,
      role: "treeitem",
    };
    // Tooltip is the AI-scrapable hover surface and stays mode-invariant
    // — always carries the full file path. [VSIX-TOP-OFFENDERS-FILE-MODE]
    this.tooltip = new vscode.MarkdownString(
      `**${labels.hybridTitle}** — ${labels.actionSentence}\n\n` +
        `file: \`${filePath}\`\n\n` +
        `rank #${rank} · weight: \`${cluster.weight.toFixed(2)}\` · size: \`${cluster.size}\` · copies: \`${occurrenceCount(cluster)}\`\n\n` +
        `cluster id: \`${cluster.id}\``,
    );
    this.command = {
      command: "deslop.openCluster",
      title: "Open cluster",
      arguments: [cluster.id],
    };
  }
}

export class OccurrenceNode extends vscode.TreeItem {
  constructor(
    readonly occurrence: ReportOccurrence,
    parentCluster?: ReportCluster,
    parentRank?: number,
    occurrenceIndex?: number,
  ) {
    const location = occurrenceDisplayLocation(occurrence);
    super(location?.label ?? occurrence.path, vscode.TreeItemCollapsibleState.None);
    if (location) this.description = location.description;
    this.contextValue =
      parentCluster !== undefined && occurrenceIndex === 0
        ? "deslop.occurrenceCanonical"
        : "deslop.occurrence";
    if (parentCluster !== undefined && occurrenceIndex !== undefined) {
      const labels = bucketLabels(resolveBucket(parentCluster));
      const total = occurrenceCount(parentCluster);
      const rankText = parentRank !== undefined ? `rank #${parentRank} · ` : "";
      this.tooltip = new vscode.MarkdownString(
        `**${rankText}${labels.plainTitle}** · occurrence ${occurrenceIndex + 1} of ${total}\n\n` +
          labels.actionSentence,
      );
    }
    this.command = {
      command: "deslop.openOccurrence",
      title: location?.commandTitle ?? "Open occurrence",
      arguments: [occurrence],
    };
  }
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] Top-level row in file mode. The
// caller passes the worst (max) and aggregate (sum) weights so a file's
// "impact at a glance" matches the sort key — without recomputing.
export class FileNode extends vscode.TreeItem {
  constructor(
    readonly filePath: string,
    readonly clusters: ReportCluster[],
    maxWeight: number,
  ) {
    const label = displayPath(filePath);
    const clusterCount = clusters.length;
    const noun = clusterCount === 1 ? "cluster" : "clusters";
    super(`${label} · ${clusterCount} ${noun}`, vscode.TreeItemCollapsibleState.Collapsed);
    this.description = `worst weight ${maxWeight.toFixed(2)}`;
    this.contextValue = "deslop.fileGroup";
    this.iconPath = new vscode.ThemeIcon("file");
    this.tooltip = new vscode.MarkdownString(
      `\`${filePath}\`\n\n` +
        `${clusterCount} duplicate ${noun} · worst weight \`${maxWeight.toFixed(2)}\``,
    );
    this.accessibilityInformation = {
      label: `${label}, ${clusterCount} duplicate ${noun}`,
      role: "treeitem",
    };
  }
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] Bucket section under a FileNode.
// Display-only: clusters carry the navigation command; the bucket
// group carries the type label and category icon.
export class BucketGroupNode extends vscode.TreeItem {
  constructor(
    readonly bucket: Bucket,
    readonly clusters: ReportCluster[],
  ) {
    const labels = bucketLabels(bucket);
    super(
      `${labels.plainTitle} (${clusters.length})`,
      vscode.TreeItemCollapsibleState.Expanded,
    );
    this.contextValue = "deslop.bucketGroup";
    this.iconPath = categoryIcon(bucket);
    this.accessibilityInformation = {
      label: `${labels.plainTitle}, ${clusters.length} clusters`,
      role: "treeitem",
    };
  }
}

export class SessionFieldNode extends vscode.TreeItem {
  constructor(label: string, value: string, commandId?: string) {
    super(label, vscode.TreeItemCollapsibleState.None);
    this.description = value;
    if (commandId) {
      this.command = { command: commandId, title: label };
    }
  }
}

export class StatusNode extends vscode.TreeItem {
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
