// [VSIX-TOP-OFFENDERS-FOLDER-MODE] Folder-tree builder. Reuses
// `groupByFile` / `fileNodeWithChildren` from `./grouping` for the file
// leaves and the shared `./pathTree` for the folder structure, so a file
// leaf behaves identically to a file-mode root.

import { ReportCluster } from "../types/report";
import { FileAgg, fileNodeWithChildren, groupByFile, worstCluster } from "./grouping";
import { FolderNode, Node } from "./nodes";
import { baseName, displayPath } from "./paths";
import { buildPathTree, countLeaves, PathTree } from "./pathTree";
import { compareWeightedPath, SortBy, WeightedPath } from "./sort";

interface BuiltChild {
  weighted: WeightedPath;
  /** The worst cluster anywhere beneath this child — the engine's
   * lowest-ranked one, carried up so a folder row shows a member's own
   * weight rather than a maximum recomputed per level. */
  worst: ReportCluster;
  node: Node;
}

/** Roots are top-level folders; each expands into sub-folders and
 * FileNodes. Single-child folder chains are path-compressed. The active
 * sort axis orders every level; global rank is untouched. */
export function buildFolderMode(clusters: ReportCluster[], sortBy: SortBy): Node[] {
  const files = groupByFile(clusters);
  const tree = buildPathTree(files, (file) => displayPath(file.path));
  return childrenOf(tree, sortBy).map((child) => child.node);
}

/** Builds and sorts the folder + file children of one trie node,
 * interleaving them by the active sort axis. */
function childrenOf(tree: PathTree<FileAgg>, sortBy: SortBy): BuiltChild[] {
  const children: BuiltChild[] = tree.folders.flatMap((folder) => folderChild(folder, sortBy));
  for (const file of tree.leaves) {
    children.push({
      weighted: {
        path: baseName(displayPath(file.path)),
        mass: file.worst.mass,
        massTotal: file.massTotal,
      },
      worst: file.worst,
      node: fileNodeWithChildren(file),
    });
  }
  const compare = compareWeightedPath(sortBy);
  children.sort((left, right) => compare(left.weighted, right.weighted));
  return children;
}

/** Builds a FolderNode, carrying up the worst descendant cluster and the
 * descendant weight total. A folder with no cluster beneath it cannot
 * exist — the trie is built from files that carry clusters — so an empty
 * one yields no row rather than an invented zero. */
function folderChild(folder: PathTree<FileAgg>, sortBy: SortBy): BuiltChild[] {
  const children = childrenOf(folder, sortBy);
  const worst = worstCluster(children.map((child) => child.worst));
  if (!worst) return [];
  const massTotal = children.reduce((sum, child) => sum + child.weighted.massTotal, 0);
  const node = new FolderNode(
    folder.path,
    folder.label,
    children.map((child) => child.node),
    worst.mass,
    countLeaves(folder),
  );
  return [{ weighted: { path: folder.label, mass: worst.mass, massTotal }, worst, node }];
}
