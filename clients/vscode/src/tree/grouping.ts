// Pure builders that turn a worst-first cluster list into the tree
// shapes spec'd in docs/specs/vsix.md under
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] and [VSIX-TOP-OFFENDERS-FILE-MODE].
// No VS Code disposables here — only TreeItem construction.

import { ReportCluster, Severity, resolveBucket, Bucket } from "../types/report";
import {
  BucketGroupNode,
  ClusterNode,
  FileNode,
  Node,
  representativePath,
} from "./nodes";

export type GroupBy = "cluster" | "file";

interface RankedCluster {
  cluster: ReportCluster;
  rank: number; // global worst-first rank — never re-numbered.
}

function rankClusters(clusters: ReportCluster[]): RankedCluster[] {
  return clusters.map((cluster, i) => ({ cluster, rank: i + 1 }));
}

// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] Roots are clusters in worst-first
// order. Children are occurrences (handled by TopOffendersProvider when
// a ClusterNode is expanded).
export function buildClusterMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
): Node[] {
  return rankClusters(clusters).map(({ cluster, rank }) => {
    const severity = severities.get(cluster.id) ?? "faint";
    return new ClusterNode(cluster, rank, severity, { showFile: true });
  });
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] Roots are files. Files sort by
// max-weight desc, sum-weight desc tiebreaker, then path localeCompare.
// Each file expands to BucketGroupNodes (sorted by max weight desc),
// each BucketGroupNode expands to ClusterNodes (weight desc) which
// drop the redundant `· <file>` suffix because the parent FileNode
// already shows it.
export function buildFileMode(
  clusters: ReportCluster[],
  severities: Map<string, Severity>,
): Node[] {
  const ranked = rankClusters(clusters);
  const groups = new Map<string, RankedCluster[]>();
  for (const entry of ranked) {
    const path = representativePath(entry.cluster);
    const bucket = groups.get(path);
    if (bucket) bucket.push(entry);
    else groups.set(path, [entry]);
  }
  const files = Array.from(groups.entries()).map(([path, entries]) => ({
    path,
    entries,
    maxWeight: entries.reduce((m, e) => Math.max(m, e.cluster.weight), 0),
    sumWeight: entries.reduce((s, e) => s + e.cluster.weight, 0),
  }));
  files.sort((a, b) =>
    b.maxWeight - a.maxWeight ||
    b.sumWeight - a.sumWeight ||
    a.path.localeCompare(b.path),
  );
  return files.map(({ path, entries, maxWeight }) =>
    fileNodeWithChildren(path, entries, maxWeight, severities),
  );
}

function fileNodeWithChildren(
  path: string,
  entries: RankedCluster[],
  maxWeight: number,
  severities: Map<string, Severity>,
): FileNode {
  const node = new FileNode(
    path,
    entries.map((e) => e.cluster),
    maxWeight,
  );
  // Children are computed lazily by the provider via getFileNodeChildren.
  // We attach the rank-preserving ranked entries to the node instance
  // so the provider can rebuild bucket groups without re-ranking.
  fileNodeRanked.set(node, entries);
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
    maxWeight: list.reduce((m, e) => Math.max(m, e.cluster.weight), 0),
  }));
  groups.sort((a, b) => b.maxWeight - a.maxWeight);
  return groups.map(({ bucket, list }) => {
    const node = new BucketGroupNode(bucket, list.map((e) => e.cluster));
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
    .sort((a, b) => b.cluster.weight - a.cluster.weight)
    .map(({ cluster, rank }) => {
      const severity = severities.get(cluster.id) ?? "faint";
      return new ClusterNode(cluster, rank, severity, { showFile: false });
    });
}
