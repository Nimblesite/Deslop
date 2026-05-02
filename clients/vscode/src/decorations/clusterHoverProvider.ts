// Hover provider — [VSIX-HOVER-PROVIDER].
// One card per hover: picks the highest-ranked cluster whose occurrence
// byte range contains the cursor. Clusters are pre-sorted by weight descending,
// so the first match is the most impactful one.

import * as vscode from "vscode";

import { clusterHoverMarkdown } from "../clusterHover";
import { ReportStore } from "../reportStore";
import { byteRangeToRange, sameFile } from "./manager";

export class ClusterHoverProvider implements vscode.HoverProvider {
  constructor(private readonly store: ReportStore) {}

  provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): vscode.Hover | null {
    const report = this.store.current.report;
    if (!report) return null;
    const activePath = document.uri.fsPath;
    for (const [index, cluster] of report.clusters.entries()) {
      for (const occurrence of cluster.occurrences) {
        if (!sameFile(occurrence.path, activePath)) continue;
        const range = byteRangeToRange(document, occurrence);
        if (range?.contains(position)) {
          return new vscode.Hover(
            clusterHoverMarkdown(cluster, { rank: index + 1, showCategory: false }),
          );
        }
      }
    }
    return null;
  }
}
