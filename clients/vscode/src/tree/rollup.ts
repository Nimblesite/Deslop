// [VSIX-METRICS-PANEL] Folder/file rows for the Duplication panel and
// the duplication-report webview. This module performs NO percentage
// arithmetic — every figure (file, folder, repo) is computed once by
// the engine's `percent` function ([METRICS-REPO],
// `crates/deslop-core/src/report_metrics.rs`) and carried on the wire
// as `metrics.per_file` / `metrics.folders`. This file only nests those
// wire rows into a tree; recomputing a percentage or re-summing LOC in
// the VSIX is prohibited.

import { FileMetric, RepoMetrics } from "../types/report";
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

type MetricRows = Pick<RepoMetrics, "per_file" | "folders">;

/** Nests the engine-computed `metrics.folders` / `metrics.per_file` rows
 * into worst-first display rows. Pure structure: values are read off the
 * wire verbatim. */
export function buildFolderRollup(metrics: MetricRows): RollupChild[] {
  const folderRows = new Map<string, FileMetric>();
  for (const row of metrics.folders) folderRows.set(normalizePath(row.path), row);
  return rollupChildren(buildPathTree(metrics.per_file, (file) => file.path), folderRows);
}

/** Joins a wire path's segments with `/` so folder-row lookups match the
 * trie paths built from file rows regardless of platform separator. */
function normalizePath(path: string): string {
  return path.split(/[/\\]/).filter(Boolean).join("/");
}

function rollupChildren(
  tree: PathTree<FileMetric>,
  folderRows: Map<string, FileMetric>,
): RollupChild[] {
  const children: RollupChild[] = [];
  for (const folder of tree.folders) {
    const built = rollupFolder(folder, folderRows);
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

/** A folder renders only when the engine emitted a row for it — no row
 * means no duplicated lines beneath it. Compressed single-child chains
 * read the deepest folder's row, whose figures equal the whole chain's. */
function rollupFolder(
  folder: PathTree<FileMetric>,
  folderRows: Map<string, FileMetric>,
): FolderRollup | null {
  const row = folderRows.get(normalizePath(folder.path));
  if (!row) return null;
  return {
    label: folder.label,
    path: folder.path,
    analysedLoc: row.analysed_loc,
    duplicatedLoc: row.duplicated_loc,
    percent: row.duplication_percent,
    children: rollupChildren(folder, folderRows),
  };
}
