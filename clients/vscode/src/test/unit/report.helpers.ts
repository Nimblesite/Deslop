import { Report, ReportCluster, RepoMetrics } from "../../types/report";

export function repoMetrics(overrides: Partial<RepoMetrics> = {}): RepoMetrics {
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

export function reportWithClusters(
  clusters: ReportCluster[],
  reportOverrides: Partial<Omit<Report, "clusters" | "metrics">> = {},
  metricsOverrides: Partial<RepoMetrics> = {},
): Report {
  return {
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 1,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: repoMetrics({ clusters_total: clusters.length, ...metricsOverrides }),
    schema_doc: "",
    action_hints: [],
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters,
    ...reportOverrides,
  };
}
