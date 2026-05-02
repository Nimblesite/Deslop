// Shared hover card renderer — [VSIX-HOVER-SHARED].
// Every surface that shows a cluster hover calls this.
// Layout follows docs/designs/vsix/hover_bubble/:
//   **#N Category** × count
//   Canonical: `relative/path/file.cs`
//   [Compare with canonical] · [View cluster] · [Dismiss?]

import * as vscode from "vscode";

import { bucketLabels, occurrenceCount, ReportCluster, resolveBucket } from "./types/report";

/// Options that differ per surface.
export interface ClusterHoverOptions {
  /// Global rank from the current report. Shown when provided.
  readonly rank?: number;
  /// Include a Dismiss link (bubble only, not the decoration hover).
  readonly showDismiss?: boolean;
}

/// Builds the VS Code MarkdownString hover card.
export function clusterHoverMarkdown(
  cluster: ReportCluster,
  options: ClusterHoverOptions = {},
): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  const labels = bucketLabels(resolveBucket(cluster));
  const count = occurrenceCount(cluster);
  const rankPrefix = options.rank !== undefined ? `#${options.rank} ` : "";

  md.appendMarkdown(`**${rankPrefix}${labels.plainTitle}** × ${count}\n\n`);

  const canonical = cluster.occurrences[0];
  if (canonical) {
    const relPath = vscode.workspace.asRelativePath(canonical.path, false);
    md.appendMarkdown(`Canonical: \`${relPath}\`\n\n`);
  }

  const openArgs = encodeURIComponent(JSON.stringify([cluster.id]));
  const links: string[] = [
    `[Compare with canonical](command:deslop.compareWithCanonical?${openArgs})`,
    `[View cluster](command:deslop.openCluster?${openArgs})`,
  ];
  if (options.showDismiss) {
    const dismissArgs = encodeURIComponent(JSON.stringify([cluster.id]));
    links.push(`[Dismiss](command:deslop.bubble.dismissCluster?${dismissArgs})`);
  }
  md.appendMarkdown(links.join(" · "));
  return md;
}
