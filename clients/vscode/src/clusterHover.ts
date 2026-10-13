// Shared hover card renderer — [VSIX-HOVER-SHARED].
// One renderer serves the detailed and 0.36.0 compact presentations.
// `showVerdict` only controls whether a nearby diagnostic supplies the title.
// The verdict is the cluster's clone kind ([CLONE-KIND-LABELS]).
// Slug is the first 7 hex chars of cluster.id — stable across runs.
// Rank must never take the id slot: Deslop#149, Deslop#349.

import * as vscode from "vscode";

import { DEFAULT_BUBBLE_PRESENTATION, type BubblePresentation } from "./bubble/presentation";
import { formatMass } from "./types/format";
import { clusterSlug, INFORMATIONAL_FINDING, isClone, kindTitle, occurrenceCount, ReportCluster } from "./types/report";

export { clusterSlug };

export interface ClusterHoverOptions {
  readonly showDismiss?: boolean;
  readonly count?: number;
  /// When false, the verdict label is omitted (use alongside a diagnostic).
  readonly showVerdict?: boolean;
  readonly presentation?: BubblePresentation;
}

export function clusterHoverMarkdown(
  cluster: ReportCluster,
  options: ClusterHoverOptions = {},
): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  const presentation = options.presentation ?? DEFAULT_BUBBLE_PRESENTATION;
  appendHeading(md, cluster, options);
  if (presentation === "classic" && isClone(cluster)) {
    md.appendMarkdown(`Mass: ${formatMass(cluster.mass)}\n\n`);
  }
  appendCanonical(md, cluster, presentation);
  if (presentation === "classic") appendOccurrences(md, cluster);
  appendActions(md, cluster, options);
  return md;
}

function appendHeading(md: vscode.MarkdownString, cluster: ReportCluster, options: ClusterHoverOptions): void {
  const slug = clusterSlug(cluster);
  const heading = options.showVerdict === false ? slug : `${slug} ${kindTitle(cluster.kind)}`;
  const countText = isClone(cluster) ? ` × ${options.count ?? occurrenceCount(cluster)}` : "";
  md.appendMarkdown(`**${heading}**${countText}\n\n`);
  if (!isClone(cluster)) md.appendMarkdown(`${INFORMATIONAL_FINDING}\n\n`);
}

function appendCanonical(md: vscode.MarkdownString, cluster: ReportCluster, presentation: BubblePresentation): void {
  const canonical = cluster.occurrences[0];
  if (!canonical) return;
  const path = vscode.workspace.asRelativePath(canonical.path, false);
  if (presentation === "compact") md.appendMarkdown(`Canonical: \`${path}\`\n\n`);
  else {
    md.appendMarkdown("Canonical: ");
    md.appendText(`${path}:${canonical.start_line}`);
    md.appendMarkdown("\n\n");
  }
}

function appendActions(md: vscode.MarkdownString, cluster: ReportCluster, options: ClusterHoverOptions): void {
  const openArgs = encodeURIComponent(JSON.stringify([cluster.id]));
  const links = [`[View cluster](command:deslop.openCluster?${openArgs})`];
  if (options.showVerdict === false) {
    links.push(`[Copy for AI](command:deslop.copyClusterContextById?${openArgs})`);
  }
  if (options.showDismiss) {
    links.push(`[Dismiss](command:deslop.bubble.dismissCluster?${openArgs})`);
  }
  md.appendMarkdown(links.join(" · "));
}

function appendOccurrences(md: vscode.MarkdownString, cluster: ReportCluster): void {
  if (cluster.occurrences.length === 0) return;
  md.appendMarkdown("Occurrences:\n\n");
  for (const [index, occurrence] of cluster.occurrences.entries()) {
    const path = vscode.workspace.asRelativePath(occurrence.path, false);
    md.appendMarkdown("- ");
    md.appendText(`${path}:${occurrence.start_line}${index === 0 ? " (canonical)" : ""}`);
    if (index > 0 && isClone(cluster)) {
      // [VSIX-PAIR-COMPARE] The chosen row supplies the peer endpoint.
      const args = encodeURIComponent(JSON.stringify([cluster.id, occurrence]));
      md.appendMarkdown(` · [Compare](command:deslop.compareWithCanonical?${args})`);
    }
    md.appendMarkdown("  \n");
  }
  md.appendMarkdown("\n");
}
