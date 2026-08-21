// Tree node classes for the Duplicate Clusters activity-bar container.
// Behavioural shape and label rules are spec'd in
// docs/specs/vsix.md under [VSIX-TOP-OFFENDERS-GROUPING],
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE],
// and [VSIX-TOP-OFFENDERS-RANK-GLOBAL].

import * as vscode from "vscode";

import { clusterSlug } from "../clusterHover";
import { occurrenceDisplayLocation } from "../locations";
import { resolveWorkspacePath } from "../pathUtils";
import { formatPercent, formatScore } from "../types/format";
import { SEVERITY_DOT } from "../severity";
import {
  Bucket,
  bucketLabels,
  FileMetric,
  occurrenceCount,
  ReportCluster,
  ReportOccurrence,
  resolveBucket,
  Severity,
} from "../types/report";
import { languageDisplayName } from "./language";
import { baseName, displayPath, representativePath } from "./paths";
import type { ThresholdStatus } from "./threshold";

// Re-exported so existing import sites (`../tree/nodes`) keep resolving
// after the helpers moved to the cycle-free `./paths` leaf module.
export { displayPath, representativePath } from "./paths";

const TREE_ITEM_ROLE = "treeitem";

export type Node =
  | ClusterNode
  | OccurrenceNode
  | FileNode
  | FolderNode
  | LanguageGroupNode
  | BucketGroupNode
  | MetricsHeadlineNode
  | FolderMetricNode
  | FileMetricNode
  | SessionFieldNode
  | StatusNode;

// [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Category colour is metadata
// backed by text/a11y labels, never the only signal.
export const CATEGORY_STYLE: Record<Bucket, { icon: string; color: string }> = {
  identical: { icon: "circle-filled", color: "charts.red" },
  nearly_identical: { icon: "circle-large-filled", color: "charts.orange" },
  structural_only: { icon: "circle-slash", color: "charts.foreground" },
  loosely_similar: { icon: "circle-outline", color: "charts.blue" },
  same_behavior: { icon: "sparkle", color: "charts.purple" },
};

export function categoryIcon(bucket: Bucket): vscode.ThemeIcon {
  const style = CATEGORY_STYLE[bucket];
  return new vscode.ThemeIcon(style.icon, new vscode.ThemeColor(style.color));
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
  /** The engine's global worst-first rank for this cluster
   * ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). Read off the wire, never
   * re-numbered from this row's position in the tree. */
  readonly rank: number;

  constructor(
    readonly cluster: ReportCluster,
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
    const rank = cluster.rank;
    this.rank = rank;
    this.description = `rank #${rank} · ${occurrenceCount(cluster)} copies`;
    this.contextValue =
      occurrenceCount(cluster) > 1 ? "deslop.clusterComparable" : "deslop.clusterSingle";
    this.iconPath = categoryIcon(bucket);
    this.accessibilityInformation = {
      label: `${labels.plainTitle} in ${fileLabel}, cluster ${cluster.id}, rank ${rank}`,
      role: TREE_ITEM_ROLE,
    };
    // Tooltip is the AI-scrapable hover surface and stays mode-invariant
    // — always carries the full file path. [VSIX-TOP-OFFENDERS-FILE-MODE]
    this.tooltip = new vscode.MarkdownString(
      `**${labels.hybridTitle}** — ${labels.actionSentence}\n\n` +
        `file: \`${filePath}\`\n\n` +
        `rank #${rank} · weight: \`${formatScore(cluster.weight)}\` · size: \`${cluster.size}\` · copies: \`${occurrenceCount(cluster)}\`\n\n` +
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

// [VSIX-TOP-OFFENDERS-FILE-MODE] Top-level row in file mode. The caller
// passes the weight of the file's worst cluster — the engine's figure,
// read off the lowest-ranked member — so "impact at a glance" matches
// the sort key without any weight being recomputed here.
export class FileNode extends vscode.TreeItem {
  constructor(
    readonly filePath: string,
    readonly clusters: ReportCluster[],
    worstWeight: number,
  ) {
    const label = displayPath(filePath);
    const clusterCount = clusters.length;
    const noun = clusterCount === 1 ? "cluster" : "clusters";
    super(`${label} · ${clusterCount} ${noun}`, vscode.TreeItemCollapsibleState.Collapsed);
    this.description = `worst weight ${formatScore(worstWeight)}`;
    this.contextValue = "deslop.fileGroup";
    this.iconPath = new vscode.ThemeIcon("file");
    this.tooltip = new vscode.MarkdownString(
      `\`${filePath}\`\n\n` +
        `${clusterCount} duplicate ${noun} · worst weight \`${formatScore(worstWeight)}\``,
    );
    this.accessibilityInformation = {
      label: `${label}, ${clusterCount} duplicate ${noun}`,
      role: TREE_ITEM_ROLE,
    };
  }
}

// Shared group-row machinery for the two grouping axes: file-mode
// bucket sections and type-mode bucket roots render through this one
// base ([FACET-GROUP-BY-TYPE] "one implementation, two group axes").
// Display-only: clusters carry the navigation command; the group row
// carries the shared label and live count.
export abstract class GroupNode extends vscode.TreeItem {
  protected constructor(
    title: string,
    readonly clusters: ReportCluster[],
    contextValue: string,
    /** Whether child cluster rows show their file suffix — true when
     * the group is a root (no file ancestor implies the file). */
    readonly showFileInChildren: boolean,
    icon?: vscode.ThemeIcon,
  ) {
    super(`${title} (${clusters.length})`, vscode.TreeItemCollapsibleState.Expanded);
    this.contextValue = contextValue;
    if (icon) this.iconPath = icon;
    this.accessibilityInformation = {
      label: `${title}, ${clusters.length} clusters`,
      role: TREE_ITEM_ROLE,
    };
  }
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] Bucket section under a FileNode, and
// [FACET-GROUP-BY-TYPE] bucket root in type grouping mode (#258) — one
// node for both axes, labelled by the shared bucket plain title.
// `showFileInChildren` is true only for type-mode roots, where no file
// ancestor implies the file.
export class BucketGroupNode extends GroupNode {
  constructor(
    readonly bucket: Bucket,
    clusters: ReportCluster[],
    showFileInChildren = false,
  ) {
    super(
      bucketLabels(bucket).plainTitle,
      clusters,
      "deslop.bucketGroup",
      showFileInChildren,
      categoryIcon(bucket),
    );
  }
}

// [VSIX-TOP-OFFENDERS-FOLDER-MODE] Folder row in folder mode. Children
// are pre-built (sub-folders and FileNodes) so the provider returns
// `node.children` directly. `label` is the compressed segment chain.
export class FolderNode extends vscode.TreeItem {
  constructor(
    readonly folderPath: string,
    label: string,
    readonly children: Node[],
    worstWeight: number,
    fileCount: number,
  ) {
    super(label, vscode.TreeItemCollapsibleState.Collapsed);
    const noun = fileCount === 1 ? "file" : "files";
    this.description = `worst weight ${formatScore(worstWeight)} · ${fileCount} ${noun}`;
    this.contextValue = "deslop.folderGroup";
    this.iconPath = vscode.ThemeIcon.Folder;
    this.tooltip = new vscode.MarkdownString(
      `\`${folderPath}\`\n\n` +
        `${fileCount} ${noun} with duplication · worst weight \`${formatScore(worstWeight)}\``,
    );
    this.accessibilityInformation = {
      label: `${label}, ${fileCount} duplicated ${noun}`,
      role: TREE_ITEM_ROLE,
    };
  }
}

// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] Outer per-language group. Wraps a
// full cluster/file/folder subtree for one language; rank stays global.
export class LanguageGroupNode extends vscode.TreeItem {
  constructor(
    readonly language: string,
    readonly children: Node[],
    worstWeight: number,
    clusterCount: number,
  ) {
    super(languageDisplayName(language), vscode.TreeItemCollapsibleState.Expanded);
    const noun = clusterCount === 1 ? "cluster" : "clusters";
    this.description = `worst weight ${formatScore(worstWeight)} · ${clusterCount} ${noun}`;
    this.contextValue = "deslop.languageGroup";
    this.iconPath = new vscode.ThemeIcon("symbol-namespace");
    this.accessibilityInformation = {
      label: `${languageDisplayName(language)}, ${clusterCount} ${noun}`,
      role: TREE_ITEM_ROLE,
    };
  }
}

// The one percentage formatter lives in the vscode-free `types` layer so
// the webviews share it ([METRICS-REPO]). Re-exported here because every
// existing tree-side import reads it from this module.
export { formatPercent };

// [VSIX-METRICS-PANEL] Headline row of the Duplication panel: the
// repo-wide duplication percentage plus the configured duplication gate
// (always shown when a gate exists, breached or not). Activating it opens
// the report webview ([VSIX-METRICS-REPORT]).
export class MetricsHeadlineNode extends vscode.TreeItem {
  constructor(percent: number, detail: string, status: ThresholdStatus) {
    super(`${formatPercent(percent)} duplicated`, vscode.TreeItemCollapsibleState.None);
    this.description = status.configured ? `${detail} · ${status.label}` : detail;
    this.contextValue = "deslop.metricsHeadline";
    this.iconPath = new vscode.ThemeIcon(
      status.breached ? "warning" : "graph",
      status.breached ? new vscode.ThemeColor("errorForeground") : undefined,
    );
    this.tooltip = new vscode.MarkdownString(
      `**${formatPercent(percent)} of analysed lines are duplicated.**\n\n${detail}\n\n` +
        (status.configured ? `${status.label}\n\n` : "") +
        "Open the full duplication report for the per-folder and per-file breakdown.",
    );
    this.command = {
      command: "deslop.openDuplicationReport",
      title: "Open duplication report",
    };
  }
}

// [VSIX-METRICS-PANEL] Folder row in the Duplication panel. `percent`
// is the exact rollup over every file beneath it. Children are the
// dup-bearing sub-folders and files.
export class FolderMetricNode extends vscode.TreeItem {
  constructor(
    readonly folderPath: string,
    label: string,
    readonly children: Node[],
    percent: number,
    analysedLoc: number,
    duplicatedLoc: number,
  ) {
    super(label, vscode.TreeItemCollapsibleState.Collapsed);
    this.description = `${formatPercent(percent)} duplicated`;
    this.contextValue = "deslop.folderMetric";
    this.iconPath = vscode.ThemeIcon.Folder;
    this.tooltip = new vscode.MarkdownString(
      `\`${folderPath}\`\n\n${formatPercent(percent)} duplicated · ${duplicatedLoc} / ${analysedLoc} LOC`,
    );
    this.accessibilityInformation = {
      label: `${label}, ${formatPercent(percent)} duplicated`,
      role: TREE_ITEM_ROLE,
    };
  }
}

// [VSIX-METRICS-PANEL] File row in the Duplication panel. Activating it
// opens the file.
export class FileMetricNode extends vscode.TreeItem {
  constructor(readonly metric: FileMetric) {
    super(baseName(displayPath(metric.path)), vscode.TreeItemCollapsibleState.None);
    this.description = `${formatPercent(metric.duplication_percent)} · ${metric.duplicated_loc}/${metric.analysed_loc} LOC`;
    this.contextValue = "deslop.fileMetric";
    this.iconPath = new vscode.ThemeIcon("file");
    // `metric.path` is rendered relative to the scan root by the engine, so
    // it must be resolved against the workspace before it names a file on
    // disk — otherwise the row opens a phantom path at the filesystem root
    // ([Deslop#328]).
    const fileUri = vscode.Uri.file(resolveWorkspacePath(metric.path));
    this.resourceUri = fileUri;
    this.tooltip = new vscode.MarkdownString(
      `\`${metric.path}\`\n\n` +
        `${formatPercent(metric.duplication_percent)} duplicated · ${metric.duplicated_loc} / ${metric.analysed_loc} LOC`,
    );
    this.command = {
      command: "vscode.open",
      title: "Open file",
      arguments: [fileUri],
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
