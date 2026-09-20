// Pure builders that turn a worst-first cluster list into the tree
// shapes spec'd in docs/specs/vsix.md under
// [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE], and
// [VSIX-TOP-OFFENDERS-FOLDER-MODE]. No VS Code disposables here — only
// TreeItem construction. Folder-mode building lives in `./folder`,
// which reuses `groupByFile` / `fileNodeWithChildren` from here.
// Every displayed mass and rank comes from the engine.

import {
  CLUSTER_KINDS,
  ClusterKind,
  ReportCluster,
  ReportOccurrence,
  compareClusterRank,
} from "../types/report";
import {
  ClusterNode,
  FileNode,
  GroupNode,
  KindGroupNode,
  Node,
} from "./nodes";
import { displayPath, representativePath } from "./paths";
import { languageForPath, languageDisplayName } from "../types/languages";
import { compareWeightedPath } from "./sort";

export type GroupBy = "cluster" | "file" | "folder" | "kind" | "language";

/** Normalizes a persisted groupBy value. Unknown / missing values fall
 * back to `"cluster"` — never panic ([VSIX-TOP-OFFENDERS-GROUPING]). */
export function normalizeGroupBy(raw: string | undefined): GroupBy {
  return raw === "file" || raw === "folder" || raw === "kind" || raw === "language" ? raw : "cluster";
}

/** A file and the clusters within it, plus the two impact keys its row
 * sorts on. Reused by file mode and folder mode. */
export interface FileAgg {
  path: string;
  clusters: ReportCluster[];
  /** The file's worst cluster — the engine's lowest-ranked member of
   * this group. Its `mass` is the row's headline mass. */
  worst: ReportCluster;
  /** Ordering tiebreak only; see {@link WeightedPath.massTotal}. */
  massTotal: number;
}

/** The worst cluster of a non-empty list: the one the engine ranked
 * highest ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). A selection, never a
 * recomputed maximum — the engine's worst-first order already decided
 * which cluster this is, ties included. */
export function worstCluster(clusters: ReportCluster[]): ReportCluster | undefined {
  return clusters.reduce<ReportCluster | undefined>(
    (worst, cluster) => (worst && compareClusterRank(worst, cluster) <= 0 ? worst : cluster),
    undefined,
  );
}

/** Total mass beneath a row — an ordering key, never a reported
 * figure ([VSIX-TOP-OFFENDERS-SORT]). */
function totalMass(clusters: ReportCluster[]): number {
  return clusters.reduce((sum, cluster) => sum + cluster.mass, 0);
}

// Worst-first display order is the engine's own ranking, so ordering by
// `rank` reproduces it exactly — including the tie-break the engine
// applies between equally weighted clusters.
const byRank = compareClusterRank;

// [VSIX-TOP-OFFENDERS-CLUSTER-MODE] Cluster roots keep engine rank.
export function buildClusterMode(clusters: ReportCluster[]): Node[] {
  return clusters.map((cluster) => new ClusterNode(cluster, { showFile: true }));
}

// [VSIX-TOP-OFFENDERS-SORT] Keep canonical occurrence order and identity.
export function orderedOccurrences(
  cluster: ReportCluster,
): { occurrence: ReportOccurrence; index: number }[] {
  return cluster.occurrences.map((occurrence, index) => ({ occurrence, index }));
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
    return worst ? [{ path, clusters: members, worst, massTotal: totalMass(members) }] : [];
  });
}

// [VSIX-TOP-OFFENDERS-FILE-MODE] File roots sort by weight descending.
export function buildFileMode(clusters: ReportCluster[]): Node[] {
  const files = groupByFile(clusters);
  const compare = compareWeightedPath();
  files.sort((left, right) =>
    compare(
      { path: displayPath(left.path), mass: left.worst.mass, massTotal: left.massTotal },
      { path: displayPath(right.path), mass: right.worst.mass, massTotal: right.massTotal },
    ),
  );
  return files.map(fileNodeWithChildren);
}

/** Builds a FileNode for a {@link FileAgg} and stashes its clusters so
 * the provider can lazily build the kind groups. Shared by file mode
 * and folder mode. */
export function fileNodeWithChildren(file: FileAgg): FileNode {
  const node = new FileNode(file.path, file.clusters, file.worst.mass);
  fileNodeClusters.set(node, file.clusters);
  return node;
}

// Per-FileNode side table keyed off the node identity. Avoids leaking
// internal types onto the public TreeItem interface and keeps the
// provider's getChildren impl trivial.
const fileNodeClusters = new WeakMap<FileNode, ReportCluster[]>();

// [CLONE-KIND-LABELS] Omit empty categories; order groups by their worst engine rank.
function kindSections(
  clusters: ReportCluster[],
  order: (list: ReportCluster[]) => ReportCluster[],
): { kind: ClusterKind; list: ReportCluster[] }[] {
  return CLUSTER_KINDS.map((kind) => ({
    kind,
    list: order(clusters.filter((cluster) => cluster.kind === kind)),
  })).filter(({ list }) => list.length > 0)
    .sort((left, right) => compareGroups(left.list, right.list));
}

// File categories share the same weight order as category roots.
export function getFileNodeChildren(file: FileNode): Node[] {
  const clusters = fileNodeClusters.get(file);
  if (!clusters) return [];
  return kindSections(clusters, (list) => list.slice().sort(byRank)).map(({ kind, list }) =>
    registerGroup(new KindGroupNode(kind, list), list),
  );
}

// Per-GroupNode side table — one machinery for BOTH group axes
// (file-mode kind sections and kind-mode roots).
// Lists are stored in final display order; the creation sites own the
// ordering.
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
    (cluster) => new ClusterNode(cluster, { showFile: group.showFileInChildren }),
  );
}

// [FACET-GROUP-BY-KIND] Flat category roots, highest weight first.
export function buildKindMode(clusters: ReportCluster[]): Node[] {
  return kindSections(clusters, (list) => list).map(({ kind, list }) =>
    registerGroup(new KindGroupNode(kind, list, true), list),
  );
}

// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] Language is a grouping choice, not a second axis.
export function buildLanguageMode(clusters: ReportCluster[]): Node[] {
  const groups = new Map<string, ReportCluster[]>();
  for (const cluster of clusters) {
    const language = languageForPath(representativePath(cluster));
    const members = groups.get(language) ?? [];
    members.push(cluster);
    groups.set(language, members);
  }
  return [...groups.entries()]
    .map(([language, members]) => ({ language, members: members.slice().sort(byRank) }))
    .sort((left, right) => compareGroups(left.members, right.members))
    .map(({ language, members }) => registerGroup(new GroupNode(languageDisplayName(language), members, "deslop.languageGroup", true), members));
}

function compareGroups(left: ReportCluster[], right: ReportCluster[]): number {
  const leftWorst = worstCluster(left);
  const rightWorst = worstCluster(right);
  return leftWorst && rightWorst ? byRank(leftWorst, rightWorst) : left.length - right.length;
}
