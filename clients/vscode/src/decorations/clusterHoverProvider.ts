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

  provideHover(document: vscode.TextDocument, position: vscode.Position): vscode.Hover | null {
    const activePath = document.uri.fsPath;
    // [VSIX-STATE-DIRTY]: hovers are a surface — read the visible projection.
    for (const cluster of this.store.current.visibleReport?.clusters ?? []) {
      const occurrence = cluster.occurrences.find((item) =>
        sameFile(item.path, activePath) &&
        byteRangeToRange(document, item)?.contains(position),
      );
      if (occurrence) return new vscode.Hover(
        clusterHoverMarkdown(cluster, { showVerdict: false }),
      );
    }
    return null;
  }
}
