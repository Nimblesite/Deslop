// Shared hover card renderer — [VSIX-HOVER-SHARED].
// Every surface that shows a cluster hover calls this.
// Two layouts:
//   Full (bubble):    **#N Category** × count  /  Canonical: `path`  /  links
//   Compact (squiggle hover): **#N** × count  /  Canonical: `path`  /  links
//   The compact form omits the category label — the diagnostic already shows it.

import * as vscode from "vscode";

import { bucketLabels, occurrenceCount, ReportCluster, resolveBucket } from "./types/report";

export interface ClusterHoverOptions {
  readonly rank?: number;
  readonly showDismiss?: boolean;
  readonly count?: number;
  /// When false, the category label is omitted — use this when the VS Code
  /// diagnostic already shows the category in the squiggle popup.
  readonly showCategory?: boolean;
}

export function clusterHoverMarkdown(
  cluster: ReportCluster,
  options: ClusterHoverOptions = {},
): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  const count = options.count ?? occurrenceCount(cluster);
  const rankPrefix = options.rank !== undefined ? `#${options.rank} ` : "";
  const showCategory = options.showCategory ?? true;

  if (showCategory) {
    const labels = bucketLabels(resolveBucket(cluster));
    md.appendMarkdown(`**${rankPrefix}${labels.plainTitle}** × ${count}\n\n`);
  } else {
    md.appendMarkdown(`**${rankPrefix}**× ${count}\n\n`);
  }

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
