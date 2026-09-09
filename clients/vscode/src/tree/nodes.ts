// Tree node classes for the Duplicate Clusters activity-bar container.
// Behavioural shape and label rules are spec'd in
// docs/specs/vsix.md under [VSIX-TOP-OFFENDERS-GROUPING],
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE],
// and [VSIX-TOP-OFFENDERS-RANK-GLOBAL].

import * as vscode from "vscode";

import { clusterSlug } from "../clusterHover";
import { KIND_ICON, KIND_THEME_COLOR, SEVERITY_DOT } from "../design";
import { occurrenceDisplayLocation } from "../locations";
import { resolveWorkspacePath } from "../pathUtils";
import { formatMass, formatPercent } from "../types/format";
import {
  ClusterKind,
  clusterBand,
  FileMetric,
  kindTaxonomy,
  kindTitle,
  occurrenceCount,
  ReportCluster,
  ReportOccurrence,
  Severity,
} from "../types/report";
import { baseName, displayPath, representativePath } from "./paths";
import type { ThresholdStatus } from "./threshold";

// Re-exported so existing import sites (`../tree/nodes`) keep resolving
// after the helpers moved to the cycle-free `./paths` leaf module.
export { displayPath, representativePath } from "./paths";

const TREE_ITEM_ROLE = "treeitem";
const FILE_NODE_KIND = "file";
const CANONICAL_OCCURRENCE_INDEX = 0;

export type Node =
  | ClusterNode
  | OccurrenceNode
  | FileNode
  | FolderNode
  | KindGroupNode
  | MetricsHeadlineNode
  | FolderMetricNode
  | FileMetricNode
  | SessionFieldNode
  | StatusNode;

// [CLONE-KIND-COLOR] The row icon is the cluster's clone kind: one codicon
// and one contributed theme colour per kind, from the single paint table
// in design.ts. Rank never chooses a colour.
export function kindIcon(kind: ClusterKind): vscode.ThemeIcon {
  return new vscode.ThemeIcon(KIND_ICON[kind], new vscode.ThemeColor(KIND_THEME_COLOR[kind]));
}

// [VSIX-TOP-OFFENDERS-CLUSTER-ID] The bold label leads with the cluster's
// stable slug (shared with the hover bubble via `clusterSlug`) — rank #N
// is volatile (re-numbered on every snapshot) and would mislead humans
// and AI consumers if it took the id slot.
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] / [VSIX-TOP-OFFENDERS-FILE-MODE]
// File mode passes `file: undefined` so the redundant `· <file>`
// suffix is dropped under a parent FileNode; cluster mode passes the
// display path. Tooltip is built separately and stays mode-invariant.
// The title is the cluster's clone kind ([CLONE-KIND-LABELS]); the dot is
// the mass rank band's glyph density ([SEVERITY-BAND]).
export function clusterRowLabel(args: {
  slug: string;
  severity: Severity;
  kind: ClusterKind;
  file?: string;
}): string {
  const head = `${args.slug} ${SEVERITY_DOT[args.severity]} ${kindTitle(args.kind)}`;
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
    options: ClusterNodeOptions = {},
  ) {
    const filePath = representativePath(cluster);
    const fileLabel = displayPath(filePath);
    const showFile = options.showFile ?? true;
    const slug = clusterSlug(cluster);
    const severity = clusterBand(cluster);
    const kind = cluster.kind;
    const title = kindTitle(kind);
    const labelArgs = showFile ? { slug, severity, kind, file: fileLabel } : { slug, severity, kind };
    super(clusterRowLabel(labelArgs), vscode.TreeItemCollapsibleState.Collapsed);
    const rank = cluster.rank;
    this.rank = rank;
    this.description = `rank #${rank} · ${occurrenceCount(cluster)} copies`;
    this.contextValue =
      occurrenceCount(cluster) > 1 ? "deslop.clusterComparable" : "deslop.clusterSingle";
    this.iconPath = kindIcon(kind);
    this.accessibilityInformation = {
      label: `${title} in ${fileLabel}, cluster ${cluster.id}, rank ${rank}`,
      role: TREE_ITEM_ROLE,
    };
    // Tooltip is the AI-scrapable hover surface and stays mode-invariant
    // — always carries the full file path. [VSIX-TOP-OFFENDERS-FILE-MODE]
    this.tooltip = new vscode.MarkdownString(
      `**${title}** (${kindTaxonomy(kind)})\n\n` +
        `file: \`${filePath}\`\n\n` +
        `rank #${rank} · mass: \`${formatMass(cluster.mass)}\` · nodes: \`${cluster.canonical_node_count}\` · copies: \`${occurrenceCount(cluster)}\`\n\n` +
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
    canonicalOccurrence: ReportOccurrence | null = parentCluster?.occurrences[CANONICAL_OCCURRENCE_INDEX] ?? null,
  ) {
    const location = occurrenceDisplayLocation(occurrence);
    super(location?.label ?? occurrence.path, vscode.TreeItemCollapsibleState.None);
    if (location) this.description = location.description;
    this.contextValue =
      parentCluster !== undefined && occurrence === canonicalOccurrence
        ? "deslop.occurrenceCanonical"
        : "deslop.occurrence";
    if (parentCluster !== undefined && occurrenceIndex !== undefined) {
      const total = occurrenceCount(parentCluster);
      const rankText = parentRank !== undefined ? `rank #${parentRank} · ` : "";
      this.tooltip = new vscode.MarkdownString(
        `**${rankText}${kindTitle(parentCluster.kind)}** · occurrence ${occurrenceIndex + 1} of ${total}`,
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
// passes the mass of the file's worst cluster — the engine's figure,
// read off the lowest-ranked member — so "impact at a glance" matches
// the sort key without any mass being recomputed here.
export class FileNode extends vscode.TreeItem {
  constructor(
    readonly filePath: string,
    readonly clusters: ReportCluster[],
    worstMass: number,
  ) {
    const label = displayPath(filePath);
    const clusterCount = clusters.length;
    const noun = clusterCount === 1 ? "cluster" : "clusters";
    super(`${label} · ${clusterCount} ${noun}`, vscode.TreeItemCollapsibleState.Collapsed);
    this.description = `worst mass ${formatMass(worstMass)}`;
    this.contextValue = "deslop.fileGroup";
    this.iconPath = new vscode.ThemeIcon(FILE_NODE_KIND);
    this.tooltip = new vscode.MarkdownString(
      `\`${filePath}\`\n\n` +
        `${clusterCount} duplicate ${noun} · worst mass \`${formatMass(worstMass)}\``,
    );
    this.accessibilityInformation = {
      label: `${label}, ${clusterCount} duplicate ${noun}`,
      role: TREE_ITEM_ROLE,
    };
  }
}

// Shared group-row machinery for the kind grouping axis: file-mode kind
// sections and kind roots render through this one base.
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

// [VSIX-TOP-OFFENDERS-FILE-MODE] Kind section under a FileNode, and kind
// root in kind grouping mode ([FACET-GROUP-BY-KIND]) — one node for both
// axes, titled and coloured by the clone kind.
// `showFileInChildren` is true only for root mode, where no file
// ancestor implies the file.
export class KindGroupNode extends GroupNode {
  constructor(
    readonly kind: ClusterKind,
    clusters: ReportCluster[],
    showFileInChildren = false,
  ) {
    super(kindTitle(kind), clusters, "deslop.kindGroup", showFileInChildren, kindIcon(kind));
    this.tooltip = new vscode.MarkdownString(`**${kindTitle(kind)}** — ${kindTaxonomy(kind)}`);
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
    worstMass: number,
    fileCount: number,
  ) {
    super(label, vscode.TreeItemCollapsibleState.Collapsed);
    const noun = fileCount === 1 ? FILE_NODE_KIND : "files";
    this.description = `worst mass ${formatMass(worstMass)} · ${fileCount} ${noun}`;
    this.contextValue = "deslop.folderGroup";
    this.iconPath = vscode.ThemeIcon.Folder;
    this.tooltip = new vscode.MarkdownString(
      `\`${folderPath}\`\n\n` +
        `${fileCount} ${noun} with duplication · worst mass \`${formatMass(worstMass)}\``,
    );
    this.accessibilityInformation = {
      label: `${label}, ${fileCount} duplicated ${noun}`,
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
    this.iconPath = new vscode.ThemeIcon(FILE_NODE_KIND);
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
