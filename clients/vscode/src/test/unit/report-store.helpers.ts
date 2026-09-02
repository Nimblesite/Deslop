// Shared ReportStore test builders: metrics, reports, occurrences and
// clusters. Split from report-store.unit.test.ts to honour the 500-line
// file rule; used by every report-store suite.

import { ReportStore } from "../../reportStore";
import { Report, ReportCluster, ReportDelta, RepoMetrics } from "../../types/report";
import { wireCluster } from "../cluster.helpers";

export function metrics(overrides: Partial<RepoMetrics> = {}): RepoMetrics {
  return {
    analysed_loc: 0,
    duplicated_loc: 0,
    duplication_percent: 0,
    clusters_total: 0,
    duplicated_files: 0,
    threshold: { percent: 0, breached: false, source: "none" },
    per_file: [],
    folders: [],
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
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters: [],
    literal_findings: [],
    literal_findings_total: 0,
    literal_findings_hidden: 0,
    literal_findings_capped: false,
    literal_max_findings: 100,
    ...overrides,
  };
}

export { occurrence } from "../cluster.helpers";

export function cluster(
  id: string,
  mass: number,
  occurrences: ReportCluster["occurrences"] = [],
  rank = 1,
): ReportCluster {
  return wireCluster({
    id,
    rank,
    mass,
    canonical_node_count: 0,
    occurrences,
    occurrences_total: occurrences.length,
  });
}

/**
 * A store already carrying one snapshot. Suites opened with the same
 * `new ReportStore()` + `setSnapshot(emptyReport({ clusters }), gen)`
 * pair; Deslop scored the copies against this repo's own corpus. Suites
 * that must observe the seeding `onDidChange` still wire the listener
 * themselves before calling `setSnapshot`.
 */
export function seededStore(clusters: ReportCluster[], generation = 1): ReportStore {
  const store = new ReportStore();
  store.setSnapshot(emptyReport({ clusters }), generation);
  return store;
}

/**
 * A generation 1 -> 2 delta carrying nothing. Overrides name only what a
 * test is actually asserting, the same shape `emptyReport` already uses,
 * so a new wire field is added in one place rather than at every literal.
 */
export function delta(overrides: Partial<ReportDelta> = {}): ReportDelta {
  return {
    from_generation: 1,
    to_generation: 2,
    clusters_added: [],
    clusters_removed: [],
    clusters_updated: [],
    literal_findings_added: [],
    literal_findings_removed: [],
    literal_findings_updated: [],
    metrics: metrics(),
    cache_stats: { hits: 0, misses: 0 },
    tool_version: "tool-v1",
    ...overrides,
  };
}
