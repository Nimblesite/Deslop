import { Report, ReportCluster, RepoMetrics } from "../../types/report";
import { emptyReport, metrics } from "./report-store.helpers";
import { stampRanks } from "../cluster.helpers";

/** Re-exported so a suite needs one helper module, not two. */
export { emptyReport };

/**
 * The zero-valued metrics block. Was a byte-identical second copy of
 * `metrics` in report-store.helpers; Deslop scored the pair against this
 * repo's own corpus. The alias keeps every existing call site reading
 * `repoMetrics(..)` while there is only one definition to keep true.
 */
export const repoMetrics = metrics;

export function reportWithClusters(
  clusters: ReportCluster[],
  reportOverrides: Partial<Omit<Report, "clusters" | "metrics">> = {},
  metricsOverrides: Partial<RepoMetrics> = {},
): Report {
  return emptyReport({
    tool_version: "v",
    files_analysed: 1,
    metrics: repoMetrics({ clusters_total: clusters.length, ...metricsOverrides }),
    // The engine stamps the ranking onto the report it publishes, so a
    // fixture report carries it too ([SEVERITY-BAND]).
    clusters: stampRanks(clusters),
    ...reportOverrides,
  });
}
