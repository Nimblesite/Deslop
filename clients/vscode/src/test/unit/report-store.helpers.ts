// Shared ReportStore test builders: metrics, reports, occurrences and
// clusters. Split from report-store.unit.test.ts to honour the 500-line
// file rule; used by every report-store suite.

import { Report, ReportCluster, RepoMetrics } from "../../types/report";

export function metrics(overrides: Partial<RepoMetrics> = {}): RepoMetrics {
  return {
    analysed_loc: 0,
    duplicated_loc: 0,
    duplication_percent: 0,
    clusters_total: 0,
    duplicated_files: 0,
    threshold: { percent: 0, breached: false, source: "none" },
    per_file: [],
    ...overrides,
  };
}

export function emptyReport(overrides: Partial<Report> = {}): Report {
  return {
    tool_version: "tool-v1",
    min_nodes: 30,
    files_analysed: 0,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: metrics(),
    schema_doc: "",
    action_hints: [],
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters: [],
    ...overrides,
  };
}

export function occurrence(path: string, startByte = 0, endByte = 10) {
  return { path, start_byte: startByte, end_byte: endByte, hidden: false };
}

export function cluster(
  id: string,
  weight: number,
  occurrences: ReportCluster["occurrences"] = [],
): ReportCluster {
  return {
    id,
    weight,
    size: Math.max(1, occurrences.length),
    canonical_node_count: 0,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences,
    occurrences_total: occurrences.length,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}
