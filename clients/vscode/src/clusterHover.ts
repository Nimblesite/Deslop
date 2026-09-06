// Shared hover card renderer — [VSIX-HOVER-SHARED].
// Two layouts controlled by `showCategory`:
//   Full (bubble, no adjacent diagnostic):
//     **{slug} Category** × count  /  Canonical: `path`  /  links + Dismiss
//   Compact (squiggle hover, alongside diagnostic):
//     **{slug}** × count  /  Canonical: `path`  /  links + Copy for AI
//   The compact form omits the category label — the diagnostic already shows it.
// Slug is the first 7 hex chars of cluster.id — stable across runs.
// Rank must never take the id slot: Deslop#149, Deslop#349.

import * as vscode from "vscode";

import { bucketLabels, clusterSlug, occurrenceCount, ReportCluster, resolveBucket } from "./types/report";

export { clusterSlug };

export interface ClusterHoverOptions {
  readonly showDismiss?: boolean;
  readonly count?: number;
  /// When false, the category label is omitted (use alongside a diagnostic).
  readonly showCategory?: boolean;
}

export function clusterHoverMarkdown(
  cluster: ReportCluster,
  options: ClusterHoverOptions = {},
): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  const count = options.count ?? occurrenceCount(cluster);
  const slug = clusterSlug(cluster);
  const showCategory = options.showCategory ?? true;

  if (showCategory) {
    const labels = bucketLabels(resolveBucket(cluster));
    md.appendMarkdown(`**${slug} ${labels.plainTitle}** × ${count}\n\n`);
  } else {
    md.appendMarkdown(`**${slug}** × ${count}\n\n`);
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
  if (!showCategory) {
    links.push(`[Copy for AI](command:deslop.copyClusterContextById?${openArgs})`);
  }
  if (options.showDismiss) {
    const dismissArgs = encodeURIComponent(JSON.stringify([cluster.id]));
    links.push(`[Dismiss](command:deslop.bubble.dismissCluster?${dismissArgs})`);
  }
  md.appendMarkdown(links.join(" · "));
  return md;
}
