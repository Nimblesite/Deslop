// Severity bucketing per [LSP-SEVERITY]. Colors live in design.ts.

import { ReportCluster, Severity } from "./types/report";

export { SEVERITY_COLOR, SEVERITY_DOT } from "./design";

export function rankPercentile(rank: number, total: number): number {
  if (total <= 1) return 0;
  return 1 - (rank - 1) / (total - 1);
}

export function severityForRank(rank: number, total: number): Severity {
  const pct = rankPercentile(rank, total);
  if (pct >= 0.99) return "worst";
  if (pct >= 0.9) return "top10";
  if (pct >= 0.5) return "mid";
  return "faint";
}

export function indexedSeverity(clusters: ReportCluster[]): Map<string, Severity> {
  const total = clusters.length;
  const out = new Map<string, Severity>();
  clusters.forEach((cluster, i) => out.set(cluster.id, severityForRank(i + 1, total)));
  return out;
}
