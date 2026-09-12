// Shared hover card renderer — [VSIX-HOVER-SHARED].
// Two layouts controlled by `showVerdict`:
//   Full (bubble, no adjacent diagnostic):
//     **{slug} Identical code** × count  /  Canonical: `path`  /  links + Dismiss
//   Compact (squiggle hover, alongside diagnostic):
//     **{slug}** × count  /  Canonical: `path`  /  links + Copy for AI
//   The compact form omits the verdict — the diagnostic already shows it.
// The verdict is the cluster's clone kind ([CLONE-KIND-LABELS]).
// Slug is the first 7 hex chars of cluster.id — stable across runs.
// Rank must never take the id slot: Deslop#149, Deslop#349.

import * as vscode from "vscode";

import { clusterSlug, INFORMATIONAL_FINDING, isClone, kindTitle, occurrenceCount, ReportCluster } from "./types/report";

export { clusterSlug };

export interface ClusterHoverOptions {
  readonly showDismiss?: boolean;
  readonly count?: number;
  /// When false, the verdict label is omitted (use alongside a diagnostic).
  readonly showVerdict?: boolean;
}

export function clusterHoverMarkdown(
  cluster: ReportCluster,
  options: ClusterHoverOptions = {},
): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  const count = options.count ?? occurrenceCount(cluster);
  const slug = clusterSlug(cluster);
  const showVerdict = options.showVerdict ?? true;

  const heading = showVerdict ? `${slug} ${kindTitle(cluster.kind)}` : slug;
  const countText = isClone(cluster) ? ` × ${count}` : "";
  md.appendMarkdown(`**${heading}**${countText}\n\n`);
  if (!isClone(cluster)) md.appendMarkdown(`${INFORMATIONAL_FINDING}\n\n`);
  const canonical = cluster.occurrences[0];
  if (canonical) {
    const relPath = vscode.workspace.asRelativePath(canonical.path, false);
    md.appendMarkdown(`Canonical: \`${relPath}\`\n\n`);
  }

  const openArgs = encodeURIComponent(JSON.stringify([cluster.id]));
  // [VSIX-PAIR-COMPARE] The cluster panel owns the per-occurrence Compare
  // action, which resolves that row against the cluster's canonical range.
  const links: string[] = [
    `[View cluster](command:deslop.openCluster?${openArgs})`,
  ];
  if (!showVerdict) {
    links.push(`[Copy for AI](command:deslop.copyClusterContextById?${openArgs})`);
  }
  if (options.showDismiss) {
    const dismissArgs = encodeURIComponent(JSON.stringify([cluster.id]));
    links.push(`[Dismiss](command:deslop.bubble.dismissCluster?${dismissArgs})`);
  }
  md.appendMarkdown(links.join(" · "));
  return md;
}
