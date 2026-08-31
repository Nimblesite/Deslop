// Shared hover card renderer — [VSIX-HOVER-SHARED].
// Two layouts controlled by `showVerdict`:
//   Full (bubble, no adjacent diagnostic):
//     **{slug} Duplicate code** × count  /  Canonical: `path`  /  links + Dismiss
//   Compact (squiggle hover, alongside diagnostic):
//     **{slug}** × count  /  Canonical: `path`  /  links + Copy for AI
//   The compact form omits the verdict — the diagnostic already shows it.
// Slug is the first 7 hex chars of cluster.id — stable across runs.
// Rank must never take the id slot: Deslop#149, Deslop#349.

import * as vscode from "vscode";

import { clusterSlug, occurrenceCount, ReportCluster } from "./types/report";

export { clusterSlug };

// [REPORTING-CONTEXT] There is no clone-kind classification to quote on
// a cluster surface; the verdict is the spec'd short label.
/** The only title a cluster surface may carry. */
export const DUPLICATION_VERDICT = "Duplicate code";

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

  md.appendMarkdown(
    showVerdict ? `**${slug} ${DUPLICATION_VERDICT}** × ${count}\n\n` : `**${slug}** × ${count}\n\n`,
  );

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
