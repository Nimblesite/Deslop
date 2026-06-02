// Pure per-folder rollup of `metrics.per_file` ([METRICS-REPO],
// [VSIX-METRICS-PANEL]). No VS Code imports — shared by the extension's
// Duplication tree (`./metrics`) and the duplication-report webview, so
// the percentage math lives in exactly one place. Denominators include
// clean files so each folder percentage is exact; folders and files
// with no duplication are dropped, and rows are worst-first by
// percentage.

import { FileMetric } from "../types/report";
import { buildPathTree, PathTree } from "./pathTree";

export interface FolderRollup {
  label: string;
  path: string;
  analysedLoc: number;
  duplicatedLoc: number;
  percent: number;
  children: RollupChild[];
}

export type RollupChild =
  | { kind: "folder"; percent: number; folder: FolderRollup }
  | { kind: "file"; percent: number; file: FileMetric };

interface FolderSums {
  analysedLoc: number;
  duplicatedLoc: number;
}

/** Rolls `perFile` into worst-first folder rows. `pathOf` maps a file to
 * its display path (workspace-relative in the extension, raw in the
 * webview). */
export function buildFolderRollup(
  perFile: FileMetric[],
  pathOf: (file: FileMetric) => string,
): RollupChild[] {
  return rollupChildren(buildPathTree(perFile, pathOf));
}

function rollupChildren(tree: PathTree<FileMetric>): RollupChild[] {
  const children: RollupChild[] = [];
  for (const folder of tree.folders) {
    const built = rollupFolder(folder);
    if (built) children.push({ kind: "folder", percent: built.percent, folder: built });
  }
  for (const file of tree.leaves) {
    if (file.duplicated_loc > 0) {
      children.push({ kind: "file", percent: file.duplication_percent, file });
    }
  }
  children.sort((left, right) => right.percent - left.percent);
  return children;
}

function rollupFolder(folder: PathTree<FileMetric>): FolderRollup | null {
  const sums = sumFolder(folder);
  if (sums.duplicatedLoc === 0) return null;
  const percent = sums.analysedLoc === 0 ? 0 : (sums.duplicatedLoc / sums.analysedLoc) * 100;
  return {
    label: folder.label,
    path: folder.path,
    analysedLoc: sums.analysedLoc,
    duplicatedLoc: sums.duplicatedLoc,
    percent,
    children: rollupChildren(folder),
  };
}

function sumFolder(folder: PathTree<FileMetric>): FolderSums {
  const leafSums = folder.leaves.reduce<FolderSums>(
    (acc, file) => ({
      analysedLoc: acc.analysedLoc + file.analysed_loc,
      duplicatedLoc: acc.duplicatedLoc + file.duplicated_loc,
    }),
    { analysedLoc: 0, duplicatedLoc: 0 },
  );
  return folder.folders.reduce<FolderSums>((acc, child) => {
    const childSums = sumFolder(child);
    return {
      analysedLoc: acc.analysedLoc + childSums.analysedLoc,
      duplicatedLoc: acc.duplicatedLoc + childSums.duplicatedLoc,
    };
  }, leafSums);
}
