// Pure builders that turn a worst-first cluster list into the tree
// shapes spec'd in docs/specs/vsix.md under
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE], and
// [VSIX-TOP-OFFENDERS-FOLDER-MODE]. No VS Code disposables here — only
// TreeItem construction. Folder-mode building lives in `./folder`,
// which reuses `groupByFile` / `fileNodeWithChildren` from here.

import {
  Bucket,
  CATEGORIES,
  ReportCluster,
  ReportOccurrence,
  resolveBucket,
  resolveCategory,
  Severity,
} from "../types/report";
import {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  GroupNode,
  Node,
  TypeGroupNode,
} from "./nodes";
import { displayPath, representativePath } from "./paths";
import { compareWeightedPath, SortBy } from "./sort";

export type GroupBy = "cluster" | "file" | "folder" | "type";

/** Normalizes a persisted groupBy value. Unknown / missing values fall
 * back to `"cluster"` — never panic ([VSIX-TOP-OFFENDERS-GROUPING]). */
export function normalizeGroupBy(raw: string | undefined): GroupBy {
  return raw === "file" || raw === "folder" || raw === "type" ? raw : "cluster";
}

export interface RankedCluster {
  cluster: ReportCluster;
  rank: number; // global worst-first rank — stable across every mode.
}

/** A file and the ranked clusters within it, with the worst (max) and
 * aggregate (sum) cluster weights precomputed for sorting. Reused by
 * file mode and folder mode. */
export interface FileAgg {
  path: string;
  entries: RankedCluster[];
  maxWeight: number;
  sumWeight: number;
}

/** Maps cluster id → 1-based global worst-first rank, built once from
 * the report's canonical order. Grouping, sorting, and language
 * splitting all read rank from here so it is never re-numbered within a
 * subtree ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). */
export function buildRankIndex(clusters: ReportCluster[]): Map<string, number> {
  const index = new Map<string, number>();
  clusters.forEach((cluster, position) => index.set(cluster.id, position + 1));
  return index;
}

function rankClusters(
  clusters: ReportCluster[],
  rankIndex: Map<string, number>,
): RankedCluster[] {
  return clusters.map((cluster) => ({ cluster, rank: rankIndex.get(cluster.id) ?? 0 }));
}

// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] Roots are clusters. The sort axis
// orders the DISPLAY: impact keeps the report's worst-first order; path
// orders by representative file path. The global rank #N is read from
// rankIndex and stays stable regardless of display order
// ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). Sorting is presentation-only — it
// never re-fetches or re-analyses ([VSIX-VIEW-STATE-UI-ONLY]).
export function buildClusterMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
  rankIndex: Map<string, number>,
  sortBy: SortBy,
): Node[] {
  const ranked = rankClusters(clusters, rankIndex);
  if (sortBy === "path") {
    ranked.sort(
      (left, right) =>
        representativePath(left.cluster).localeCompare(representativePath(right.cluster)) ||
        right.cluster.weight - left.cluster.weight,
    );
  }
  return ranked.map(({ cluster, rank }) => {
    const severity = severities.get(cluster.id) ?? "faint";
    return new ClusterNode(cluster, rank, severity, { showFile: true });
  });
}

// [VSIX-TOP-OFFENDERS-SORT] Orders a cluster's occurrences for display
// under the active sort axis, preserving each occurrence's ORIGINAL index
// so the canonical badge (index 0) and "occurrence N of M" labels stay
// identity-stable. impact keeps the report's canonical order; path orders
// by file path then byte offset.
export function orderedOccurrences(
  cluster: ReportCluster,
  sortBy: SortBy,
): { occurrence: ReportOccurrence; index: number }[] {
  const entries = cluster.occurrences.map((occurrence, index) => ({ occurrence, index }));
  if (sortBy === "path") {
    entries.sort(
      (left, right) =>
        left.occurrence.path.localeCompare(right.occurrence.path) ||
        left.occurrence.start_byte - right.occurrence.start_byte,
    );
  }
  return entries;
}

/** Buckets ranked clusters by their representative file into {@link
 * FileAgg} rows. Reused by file mode and folder mode. */
export function groupByFile(
  clusters: ReportCluster[],
  rankIndex: Map<string, number>,
): FileAgg[] {
  const groups = new Map<string, RankedCluster[]>();
  for (const entry of rankClusters(clusters, rankIndex)) {
    const path = representativePath(entry.cluster);
    const bucket = groups.get(path);
    if (bucket) bucket.push(entry);
    else groups.set(path, [entry]);
  }
  return Array.from(groups.entries()).map(([path, entries]) => ({
    path,
    entries,
    maxWeight: entries.reduce((max, entry) => Math.max(max, entry.cluster.weight), 0),
    sumWeight: entries.reduce((sum, entry) => sum + entry.cluster.weight, 0),
  }));
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] Roots are files. The sort axis orders
// them: impact = max weight desc (sum desc, path); path = relative path
// localeCompare. Each file expands to BucketGroupNodes.
export function buildFileMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
  rankIndex: Map<string, number>,
  sortBy: SortBy,
): Node[] {
  const files = groupByFile(clusters, rankIndex);
  const compare = compareWeightedPath(sortBy);
  files.sort((left, right) =>
    compare(
      { path: displayPath(left.path), maxWeight: left.maxWeight, sumWeight: left.sumWeight },
      { path: displayPath(right.path), maxWeight: right.maxWeight, sumWeight: right.sumWeight },
    ),
  );
  return files.map((file) => fileNodeWithChildren(file, severities));
}

/** Builds a FileNode for a {@link FileAgg} and stashes the ranked
 * entries + severities so the provider can lazily build its bucket
 * groups without re-ranking. Shared by file mode and folder mode. */
export function fileNodeWithChildren(
  file: FileAgg,
  severities: Map<string, Severity>,
): FileNode {
  const node = new FileNode(
    file.path,
    file.entries.map((entry) => entry.cluster),
    file.maxWeight,
  );
  fileNodeRanked.set(node, file.entries);
  fileNodeSeverities.set(node, severities);
  return node;
}

// Per-FileNode side tables keyed off the node identity. Avoids leaking
// internal types onto the public TreeItem interface and keeps the
// provider's getChildren impl trivial.
const fileNodeRanked = new WeakMap<FileNode, RankedCluster[]>();
const fileNodeSeverities = new WeakMap<FileNode, Map<string, Severity>>();

// Children of a FileNode: one BucketGroupNode per bucket present,
// sorted by max cluster weight desc, each group's clusters pre-ordered
// weight desc.
export function getFileNodeChildren(file: FileNode): Node[] {
  const ranked = fileNodeRanked.get(file);
  if (!ranked) return [];
  const byBucket = new Map<Bucket, RankedCluster[]>();
  for (const entry of ranked) {
    const bucket = resolveBucket(entry.cluster);
    const list = byBucket.get(bucket);
    if (list) list.push(entry);
    else byBucket.set(bucket, [entry]);
  }
  const groups = Array.from(byBucket.entries()).map(([bucket, list]) => ({
    bucket,
    list: list.slice().sort((left, right) => right.cluster.weight - left.cluster.weight),
    maxWeight: list.reduce((max, entry) => Math.max(max, entry.cluster.weight), 0),
  }));
  groups.sort((left, right) => right.maxWeight - left.maxWeight);
  const severities = fileNodeSeverities.get(file) ?? new Map<string, Severity>();
  return groups.map(({ bucket, list }) =>
    registerGroup(new BucketGroupNode(bucket, list.map((entry) => entry.cluster)), list, severities),
  );
}

// Per-GroupNode side tables — one machinery for BOTH group axes
// (file-mode bucket sections and type-mode category roots,
// [FACET-GROUP-BY-TYPE]). Lists are stored in final display order; the
// creation sites own the ordering.
const groupRanked = new WeakMap<GroupNode, RankedCluster[]>();
const groupSeverities = new WeakMap<GroupNode, Map<string, Severity>>();

/** Stashes a group's pre-ordered entries + severities and returns it. */
function registerGroup(
  node: GroupNode,
  list: RankedCluster[],
  severities: Map<string, Severity>,
): GroupNode {
  groupRanked.set(node, list);
  groupSeverities.set(node, severities);
  return node;
}

// Children of any GroupNode: ClusterNodes in the group's stored display
// order, with the file suffix driven by the group's axis.
export function getGroupNodeChildren(group: GroupNode): Node[] {
  const ranked = groupRanked.get(group);
  if (!ranked) return [];
  const severities = groupSeverities.get(group) ?? new Map<string, Severity>();
  return ranked.map(({ cluster, rank }) => {
    const severity = severities.get(cluster.id) ?? "faint";
    return new ClusterNode(cluster, rank, severity, { showFile: group.showFileInChildren });
  });
}

// [FACET-GROUP-BY-TYPE] Roots are one group per category present, in
// registry order, empty groups omitted. Under the impact axis clusters
// stay worst-first inside each group; the path axis orders them by
// representative path, exactly like cluster mode. Rank #N stays global.
export function buildTypeMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
  rankIndex: Map<string, number>,
  sortBy: SortBy,
): Node[] {
  const ranked = rankClusters(clusters, rankIndex);
  if (sortBy === "path") {
    ranked.sort(
      (left, right) =>
        representativePath(left.cluster).localeCompare(representativePath(right.cluster)) ||
        right.cluster.weight - left.cluster.weight,
    );
  }
  return CATEGORIES.map((category) => ({
    category,
    list: ranked.filter((entry) => resolveCategory(entry.cluster) === category),
  }))
    .filter(({ list }) => list.length > 0)
    .map(({ category, list }) =>
      registerGroup(new TypeGroupNode(category, list.map((entry) => entry.cluster)), list, severities),
    );
}
