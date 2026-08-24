// Pure builders that turn a worst-first cluster list into the tree
// shapes spec'd in docs/specs/vsix.md under
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE], and
// [VSIX-TOP-OFFENDERS-FOLDER-MODE]. No VS Code disposables here — only
// TreeItem construction. Folder-mode building lives in `./folder`,
// which reuses `groupByFile` / `fileNodeWithChildren` from here.
//
// Every figure these builders show belongs to the engine. The global
// rank and the severity band are stamped on the cluster
// ([VSIX-TOP-OFFENDERS-RANK-GLOBAL], [SEVERITY-BAND]) instead of being
// re-derived from array position, and a group's headline weight is read
// off its worst member rather than recomputed as a maximum. What is left
// here is ordering and nesting — presentation mechanics over engine
// values.

import {
  Bucket,
  BUCKETS,
  ReportCluster,
  ReportOccurrence,
  clusterBand,
  resolveBucket,
} from "../types/report";
import {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  GroupNode,
  Node,
} from "./nodes";
import { displayPath, representativePath } from "./paths";
import { compareWeightedPath, SortBy } from "./sort";

export type GroupBy = "cluster" | "file" | "folder" | "type";

/** Normalizes a persisted groupBy value. Unknown / missing values fall
 * back to `"cluster"` — never panic ([VSIX-TOP-OFFENDERS-GROUPING]). */
export function normalizeGroupBy(raw: string | undefined): GroupBy {
  return raw === "file" || raw === "folder" || raw === "type" ? raw : "cluster";
}

/** A file and the clusters within it, plus the two impact keys its row
 * sorts on. Reused by file mode and folder mode. */
export interface FileAgg {
  path: string;
  clusters: ReportCluster[];
  /** The file's worst cluster — the engine's lowest-ranked member of
   * this group. Its `weight` is the row's headline weight. */
  worst: ReportCluster;
  /** Ordering tiebreak only; see {@link WeightedPath.weightTotal}. */
  weightTotal: number;
}

/** The worst cluster of a non-empty list: the one the engine ranked
 * highest ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). A selection, never a
 * recomputed maximum — the engine's worst-first order already decided
 * which cluster this is, ties included. */
export function worstCluster(clusters: ReportCluster[]): ReportCluster | undefined {
  return clusters.reduce<ReportCluster | undefined>(
    (worst, cluster) => (worst && worst.rank <= cluster.rank ? worst : cluster),
    undefined,
  );
}

/** Total weight beneath a row — an ordering key, never a reported
 * figure ([VSIX-TOP-OFFENDERS-SORT]). */
function totalWeight(clusters: ReportCluster[]): number {
  return clusters.reduce((sum, cluster) => sum + cluster.weight, 0);
}

// Worst-first display order is the engine's own ranking, so ordering by
// `rank` reproduces it exactly — including the tie-break the engine
// applies between equally weighted clusters.
function byRank(left: ReportCluster, right: ReportCluster): number {
  return left.rank - right.rank;
}

// Shared display ordering for cluster mode and type mode: impact keeps
// the report's worst-first order; path re-orders by representative file
// path with the engine's rank as the tie-break
// ([VSIX-TOP-OFFENDERS-SORT]).
function ordered(clusters: ReportCluster[], sortBy: SortBy): ReportCluster[] {
  if (sortBy !== "path") return clusters;
  return clusters
    .slice()
    .sort(
      (left, right) =>
        representativePath(left).localeCompare(representativePath(right)) || byRank(left, right),
    );
}

// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] Roots are clusters. The sort axis
// orders the DISPLAY: impact keeps the report's worst-first order; path
// orders by representative file path. The global rank #N is the engine's
// and stays stable regardless of display order
// ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). Sorting is presentation-only — it
// never re-fetches or re-analyses ([VSIX-VIEW-STATE-UI-ONLY]).
export function buildClusterMode(clusters: ReportCluster[], sortBy: SortBy): Node[] {
  return ordered(clusters, sortBy).map(
    (cluster) => new ClusterNode(cluster, clusterBand(cluster), { showFile: true }),
  );
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

/** Buckets clusters by their representative file into {@link FileAgg}
 * rows. Reused by file mode and folder mode. */
export function groupByFile(clusters: ReportCluster[]): FileAgg[] {
  const groups = new Map<string, ReportCluster[]>();
  for (const cluster of clusters) {
    const path = representativePath(cluster);
    const bucket = groups.get(path);
    if (bucket) bucket.push(cluster);
    else groups.set(path, [cluster]);
  }
  return Array.from(groups.entries()).flatMap(([path, members]) => {
    const worst = worstCluster(members);
    return worst ? [{ path, clusters: members, worst, weightTotal: totalWeight(members) }] : [];
  });
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] Roots are files. The sort axis orders
// them: impact = worst-cluster weight desc (total desc, path); path =
// relative path localeCompare. Each file expands to BucketGroupNodes.
export function buildFileMode(clusters: ReportCluster[], sortBy: SortBy): Node[] {
  const files = groupByFile(clusters);
  const compare = compareWeightedPath(sortBy);
  files.sort((left, right) =>
    compare(
      { path: displayPath(left.path), weight: left.worst.weight, weightTotal: left.weightTotal },
      { path: displayPath(right.path), weight: right.worst.weight, weightTotal: right.weightTotal },
    ),
  );
  return files.map(fileNodeWithChildren);
}

/** Builds a FileNode for a {@link FileAgg} and stashes its clusters so
 * the provider can lazily build the bucket groups. Shared by file mode
 * and folder mode. */
export function fileNodeWithChildren(file: FileAgg): FileNode {
  const node = new FileNode(file.path, file.clusters, file.worst.weight);
  fileNodeClusters.set(node, file.clusters);
  return node;
}

// Per-FileNode side table keyed off the node identity. Avoids leaking
// internal types onto the public TreeItem interface and keeps the
// provider's getChildren impl trivial.
const fileNodeClusters = new WeakMap<FileNode, ReportCluster[]>();

// Children of a FileNode: one BucketGroupNode per bucket present,
// ordered by each bucket's worst cluster, with the clusters inside each
// group in the engine's worst-first order.
export function getFileNodeChildren(file: FileNode): Node[] {
  const clusters = fileNodeClusters.get(file);
  if (!clusters) return [];
  const byBucket = new Map<Bucket, ReportCluster[]>();
  for (const cluster of clusters) {
    const bucket = resolveBucket(cluster);
    const list = byBucket.get(bucket);
    if (list) list.push(cluster);
    else byBucket.set(bucket, [cluster]);
  }
  const groups = Array.from(byBucket.entries()).flatMap(([bucket, list]) => {
    const ordering = list.slice().sort(byRank);
    const worst = worstCluster(ordering);
    return worst ? [{ bucket, list: ordering, worst }] : [];
  });
  groups.sort((left, right) => right.worst.weight - left.worst.weight);
  return groups.map(({ bucket, list }) =>
    registerGroup(new BucketGroupNode(bucket, list), list),
  );
}

// Per-GroupNode side table — one machinery for BOTH group axes
// (file-mode bucket sections and type-mode bucket roots,
// [FACET-GROUP-BY-TYPE]). Lists are stored in final display order; the
// creation sites own the ordering.
const groupClusters = new WeakMap<GroupNode, ReportCluster[]>();

/** Stashes a group's pre-ordered clusters and returns it. */
function registerGroup(node: GroupNode, list: ReportCluster[]): GroupNode {
  groupClusters.set(node, list);
  return node;
}

// Children of any GroupNode: ClusterNodes in the group's stored display
// order, with the file suffix driven by the group's axis.
export function getGroupNodeChildren(group: GroupNode): Node[] {
  const clusters = groupClusters.get(group);
  if (!clusters) return [];
  return clusters.map(
    (cluster) =>
      new ClusterNode(cluster, clusterBand(cluster), { showFile: group.showFileInChildren }),
  );
}

// [FACET-GROUP-BY-TYPE] Roots are one flat group per bucket present, in
// registry order, empty groups omitted (#258) — every Identical cluster
// surfaces together with no file/folder layer in between. Under the
// impact axis clusters stay worst-first inside each group; the path axis
// orders them by representative path, exactly like cluster mode. Rank #N
// stays global.
export function buildTypeMode(clusters: ReportCluster[], sortBy: SortBy): Node[] {
  const display = ordered(clusters, sortBy);
  return BUCKETS.map((bucket) => ({
    bucket,
    list: display.filter((cluster) => resolveBucket(cluster) === bucket),
  }))
    .filter(({ list }) => list.length > 0)
    .map(({ bucket, list }) => registerGroup(new BucketGroupNode(bucket, list, true), list));
}
