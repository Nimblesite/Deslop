// [VSIX-METRICS-PANEL] Turns the engine-computed `metrics.folders` /
// `metrics.per_file` wire rows (`./rollup` nests them; the engine's
// single `percent` function computed them) into the Duplication panel's
// tree nodes. No percentage is ever calculated in the VSIX.

import { RepoMetrics } from "../types/report";
import { FileMetricNode, FolderMetricNode, Node } from "./nodes";
import { buildFolderRollup, RollupChild } from "./rollup";

/** Builds the per-folder / per-file rows beneath the headline. */
export function buildMetricRows(metrics: RepoMetrics): Node[] {
  return buildFolderRollup(metrics).map(toNode);
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
