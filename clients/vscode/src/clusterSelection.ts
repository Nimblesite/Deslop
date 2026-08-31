// Stable cluster selection for detail panels (#173). A cluster's wire `id` is a
// content hash of its smallest member (deslop-core cluster.rs), so it churns
// whenever that member's normalised AST shifts under re-analysis. Pinning a
// panel to the raw id therefore loses the selection across a generation — the
// webview resolves `find(c => c.id === id)` to nothing and renders blank. We
// anchor on the canonical occurrence instead — a stable editor location that
// survives id churn — and fall back to the id only when no anchor match remains.

import { Report, ReportCluster } from "./types/report";

/** Stable handle for a pinned cluster selection across re-analysis (#173). */
export interface ClusterAnchor {
  /** Last known wire id — fallback when the canonical occurrence is gone. */
  readonly id: string;
  /** Canonical occurrence path. */
  readonly path: string;
  /** Canonical occurrence start byte. */
  readonly startByte: number;
  /** Canonical occurrence end byte. */
  readonly endByte: number;
}

/** Snapshot plus selection a cluster detail panel must receive (#173). */
export interface ClusterPanelFeed {
  /** Report to push — the visible projection, with the selected cluster guaranteed present. */
  readonly report: Report;
  /** Current id of the selected cluster, or null when it no longer exists. */
  readonly selectedId: string | null;
}

/** Captures a stable anchor from a cluster's canonical occurrence. */
export function clusterAnchor(cluster: ReportCluster): ClusterAnchor {
  const canonical = cluster.occurrences[0];
  return {
    id: cluster.id,
    path: canonical?.path ?? "",
    startByte: canonical?.start_byte ?? -1,
    endByte: canonical?.end_byte ?? -1,
  };
}

/** Builds an anchor for a freshly-clicked id against the canonical report. */
export function anchorForClusterId(report: Report | null, clusterId: string): ClusterAnchor {
  const cluster = report?.clusters.find((candidate) => candidate.id === clusterId);
  return cluster
    ? clusterAnchor(cluster)
    : { id: clusterId, path: "", startByte: -1, endByte: -1 };
}

/** Resolves the live cluster for a pinned anchor, surviving id churn (#173). */
export function resolveAnchoredCluster(
  report: Report | null,
  anchor: ClusterAnchor,
): ReportCluster | undefined {
  if (!report) return undefined;
  const byAnchor = report.clusters.find((cluster) =>
    cluster.occurrences.some(
      (occurrence) =>
        occurrence.path === anchor.path &&
        occurrence.start_byte === anchor.startByte &&
        occurrence.end_byte === anchor.endByte,
    ),
  );
  return byAnchor ?? report.clusters.find((cluster) => cluster.id === anchor.id);
}

/**
 * Builds the snapshot and selection a cluster detail panel must receive so the
 * opened cluster is ALWAYS resolvable — surviving content-hash id churn
 * ([VSIX-STATE]) and dirty-file elision ([VSIX-STATE-DIRTY], #117). The pinned
 * anchor is resolved against the canonical report, then guaranteed present in
 * the visible projection that surfaces render from (#173).
 */
export function clusterPanelFeed(
  canonical: Report,
  visible: Report,
  anchor: ClusterAnchor,
): ClusterPanelFeed {
  const resolved = resolveAnchoredCluster(canonical, anchor);
  if (!resolved) return { report: visible, selectedId: null };
  if (visible.clusters.some((cluster) => cluster.id === resolved.id)) {
    return { report: visible, selectedId: resolved.id };
  }
  const clusters = [resolved, ...visible.clusters].sort((left, right) => right.mass - left.mass);
  return { report: { ...visible, clusters }, selectedId: resolved.id };
}
