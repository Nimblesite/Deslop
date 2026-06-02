// [VSIX-METRICS-PANEL] Turns the shared per-folder rollup (`./rollup`)
// into the Duplication panel's tree nodes. The percentage math lives in
// `./rollup` so the sidebar panel and the report webview stay in sync.

import { RepoMetrics } from "../types/report";
import { FileMetricNode, FolderMetricNode, Node } from "./nodes";
import { displayPath } from "./paths";
import { buildFolderRollup, RollupChild } from "./rollup";

/** Builds the per-folder / per-file rows beneath the headline. */
export function buildMetricRows(metrics: RepoMetrics): Node[] {
  return buildFolderRollup(metrics.per_file, (file) => displayPath(file.path)).map(toNode);
}

function toNode(child: RollupChild): Node {
  if (child.kind === "file") return new FileMetricNode(child.file);
  const { folder } = child;
  return new FolderMetricNode(
    folder.path,
    folder.label,
    folder.children.map(toNode),
    folder.percent,
    folder.analysedLoc,
    folder.duplicatedLoc,
  );
}
