// Pure builders that turn a worst-first cluster list into the tree
// shapes spec'd in docs/specs/vsix.md under
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE], and
// [VSIX-TOP-OFFENDERS-FOLDER-MODE]. No VS Code disposables here — only
// TreeItem construction. Folder-mode building lives in `./folder`,
// which reuses `groupByFile` / `fileNodeWithChildren` from here.

import { ReportCluster, Severity, resolveBucket, Bucket } from "../types/report";
import {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  Node,
} from "./nodes";
import { displayPath, representativePath } from "./paths";
import { compareWeightedPath, SortBy } from "./sort";

export type GroupBy = "cluster" | "file" | "folder";

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

// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] Roots are clusters in worst-first
// order. Children are occurrences (handled by TopOffendersProvider when
// a ClusterNode is expanded). Cluster mode ignores the sort axis.
export function buildClusterMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
  rankIndex: Map<string, number>,
): Node[] {
  return rankClusters(clusters, rankIndex).map(({ cluster, rank }) => {
    const severity = severities.get(cluster.id) ?? "faint";
    return new ClusterNode(cluster, rank, severity, { showFile: true });
  });
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
// sorted by max cluster weight desc.
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
    list,
    maxWeight: list.reduce((max, entry) => Math.max(max, entry.cluster.weight), 0),
  }));
  groups.sort((left, right) => right.maxWeight - left.maxWeight);
  return groups.map(({ bucket, list }) => {
    const node = new BucketGroupNode(bucket, list.map((entry) => entry.cluster));
    bucketGroupRanked.set(node, list);
    bucketGroupSeverities.set(node, fileNodeSeverities.get(file) ?? new Map<string, Severity>());
    return node;
  });
}

const bucketGroupRanked = new WeakMap<BucketGroupNode, RankedCluster[]>();
const bucketGroupSeverities = new WeakMap<
  BucketGroupNode,
  Map<string, Severity>
>();

// Children of a BucketGroupNode: ClusterNodes sorted by weight desc,
// with the file suffix dropped from the label.
export function getBucketGroupChildren(group: BucketGroupNode): Node[] {
  const ranked = bucketGroupRanked.get(group);
  if (!ranked) return [];
  const severities = bucketGroupSeverities.get(group) ?? new Map<string, Severity>();
  return ranked
    .slice()
    .sort((left, right) => right.cluster.weight - left.cluster.weight)
    .map(({ cluster, rank }) => {
      const severity = severities.get(cluster.id) ?? "faint";
      return new ClusterNode(cluster, rank, severity, { showFile: false });
    });
}
