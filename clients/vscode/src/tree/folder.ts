// [VSIX-TOP-OFFENDERS-FOLDER-MODE] Folder-tree builder. Reuses
// `groupByFile` / `fileNodeWithChildren` from `./grouping` for the file
// leaves and the shared `./pathTree` for the folder structure, so a file
// leaf behaves identically to a file-mode root.

import { ReportCluster, Severity } from "../types/report";
import { FileAgg, fileNodeWithChildren, groupByFile } from "./grouping";
import { FolderNode, Node } from "./nodes";
import { baseName, displayPath } from "./paths";
import { buildPathTree, countLeaves, PathTree } from "./pathTree";
import { compareWeightedPath, SortBy, WeightedPath } from "./sort";

interface BuiltChild {
  weighted: WeightedPath;
  node: Node;
}

/** Roots are top-level folders; each expands into sub-folders and
 * FileNodes. Single-child folder chains are path-compressed. The active
 * sort axis orders every level; global rank is untouched. */
export function buildFolderMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
  rankIndex: Map<string, number>,
  sortBy: SortBy,
): Node[] {
  const files = groupByFile(clusters, rankIndex);
  const tree = buildPathTree(files, (file) => displayPath(file.path));
  return childrenOf(tree, severities, sortBy).map((child) => child.node);
}

/** Builds and sorts the folder + file children of one trie node,
 * interleaving them by the active sort axis. */
function childrenOf(
  tree: PathTree<FileAgg>,
  severities: Map<string, Severity>,
  sortBy: SortBy,
): BuiltChild[] {
  const children: BuiltChild[] = tree.folders.map((folder) =>
    folderChild(folder, severities, sortBy),
  );
  for (const file of tree.leaves) {
    children.push({
      weighted: {
        path: baseName(displayPath(file.path)),
        maxWeight: file.maxWeight,
        sumWeight: file.sumWeight,
      },
      node: fileNodeWithChildren(file, severities),
    });
  }
  const compare = compareWeightedPath(sortBy);
  children.sort((left, right) => compare(left.weighted, right.weighted));
  return children;
}

/** Builds a FolderNode, aggregating descendant weights and file count. */
function folderChild(
  folder: PathTree<FileAgg>,
  severities: Map<string, Severity>,
  sortBy: SortBy,
): BuiltChild {
  const children = childrenOf(folder, severities, sortBy);
  const maxWeight = children.reduce((max, child) => Math.max(max, child.weighted.maxWeight), 0);
  const sumWeight = children.reduce((sum, child) => sum + child.weighted.sumWeight, 0);
  const node = new FolderNode(
    folder.path,
    folder.label,
    children.map((child) => child.node),
    maxWeight,
    countLeaves(folder),
  );
  return { weighted: { path: folder.label, maxWeight, sumWeight }, node };
}
