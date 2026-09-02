// Mirrors deslop-core::report.
// Every wire shape has a typeDiagram .td entry and is re-exported from
// `./wire-generated`. UI-only augmentation (`displayLocation`) lives as
// an intersection so the wire shape stays the source of truth.

import type {
  Report as WireReport,
  ReportCluster as WireReportCluster,
  ReportOccurrence as WireReportOccurrence,
} from "./wire-generated";

export type {
  CacheStats,
  EmbeddingProvenance,
} from "./wire-generated";

// Wire `ReportOccurrence` plus the VSIX-only display projection the
// extension host stamps onto each occurrence before posting reports
// into webviews. The display field is not part of the canonical wire
// schema and never crosses the LSP boundary, so we layer it onto the
// generated wire type instead of polluting the .td source.
export type ReportOccurrence = WireReportOccurrence & {
  displayLocation?: OccurrenceDisplayLocation;
};

// Override the wire `ReportCluster.occurrences` and `Report.clusters`
// fields so the augmented `displayLocation` propagates end-to-end into
// every UI surface that reads the report.
export type ReportCluster = Omit<WireReportCluster, "occurrences"> & {
  occurrences: ReportOccurrence[];
};

export type Report = Omit<WireReport, "clusters"> & {
  clusters: ReportCluster[];
};

export interface OccurrenceDisplayLocation {
  line: number;
  column: number;
  label: string;
  description: string;
  commandTitle: string;
}

const SLUG_LENGTH = 7;

/** Stable display slug for a cluster (first 7 hex chars of id). Shared
 * across every cluster surface — tree, hover, bubble, report panel — so
 * humans and AI agents see the same identity instead of the volatile
 * #N rank index. [VSIX-CLUSTER-ID-CONSISTENCY] */
export function clusterSlug(cluster: ReportCluster): string {
  return cluster.id.slice(0, SLUG_LENGTH);
}

/** The cluster's occurrence count, exactly as the engine computed it
 * (`deslop_core::report::occurrence_count`). There is one counting
 * formula and it lives in Rust: the live wire truncates the carried
 * occurrence list, so a count re-derived here would silently disagree
 * with the report on every large cluster. */
export function occurrenceCount(cluster: ReportCluster): number {
  return cluster.occurrence_count;
}

// Wire-format models generated from `docs/models/live-ipc.td` by
// `scripts/typediagram/generate.mjs`. Re-exported here so the historical
// `clients/vscode/src/types/report` import path keeps resolving for
// every consumer. The generated source is gitignored; `make
// typediagram-gen` (chained into `make vsix-build`) regenerates it.
export type {
  AnalysisState,
  ChangeSummary,
  EmbeddingModelInfo,
  EmbeddingPhase,
  EmbeddingProgress,
  FileMetric,
  PairClassification,
  PairComparison,
  PairComparisonParams,
  PairEndpoint,
  PairEvidence,
  ReportChangedNotification,
  ReportDelta,
  RepoMetrics,
  SessionConfig,
  ThresholdSource,
  ThresholdSummary,
} from "./wire-generated";

// Historical TS spelling preserved via aliasing — Rust calls the wire
// types `ReportBoilerplate*` to mirror their report-namespaced module,
// the VSIX has always called them `Boilerplate*`. Single .td source
// keeps both conventions resolving to the same generated shape.
export type {
  ReportBoilerplateHint as BoilerplateHint,
  ReportBoilerplateOccurrence as BoilerplateHintOccurrence,
} from "./wire-generated";

// Severity bucketing per [LSP-SEVERITY-BAND]. The band classifies the
// cluster's rank percentile, which is a calculation, so it is computed
// once in `report_weight::rank_band` and carried on the wire.
export type Severity = "worst" | "top10" | "mid" | "faint";

/** Every severity level in rank order. Filter surfaces enumerate this
 * instead of hand-listing levels ([FACET-REPORT-WEBVIEW]). */
export const SEVERITIES: readonly Severity[] = ["worst", "top10", "mid", "faint"] as const;

const SEVERITY_LABELS: Record<Severity, string> = {
  worst: "Worst 1%",
  top10: "Top 10%",
  mid: "Mid 40%",
  faint: "Faint",
};

/** Human label for a severity level, shared by every filter surface. */
export function severityLabel(severity: Severity): string {
  return SEVERITY_LABELS[severity];
}

/** The cluster's severity band as the engine stamped it
 * ([SEVERITY-BAND]). A report written before the field existed carries
 * an empty string and reads as the tail band. */
export function clusterBand(cluster: ReportCluster): Severity {
  return SEVERITIES.find((band) => band === cluster.rank_band) ?? "faint";
}

/** The duplicated mass — the worst-first ranking metric. One formula
 * lives in Rust ([RANK-MASS-SUM]); clients carry the value. */
export function clusterMass(cluster: ReportCluster): number {
  return cluster.mass;
}

// [FACET-TOP-OFFENDERS-FILTER] Facets filter on the mass severity band
// only. Clone-kind axes (bucket, category) are retired: the report
// carries no similarity classification, and the persisted per-bucket
// filter settings are invalid configuration ([SEVERITY-CONFIG]).

/** A sanitized facet filter: only registry-known severity bands
 * survive. */
export interface FacetFilter {
  severities: Severity[];
}

// [FACET-TOP-OFFENDERS-FILTER] Drops unknown values from the persisted
// filter array (the typo fallback — a bad value must never yield an
// empty tree). An empty list means the filter is inactive.
export function sanitizeFacetFilter(filterSeverities: readonly string[]): FacetFilter {
  return {
    severities: filterSeverities.filter((value): value is Severity =>
      (SEVERITIES as readonly string[]).includes(value),
    ),
  };
}

// [FACET-TOP-OFFENDERS-FILTER] The one facet-filter slice shared by the
// Top Offenders tree, the report webview, and the status-bar count so
// the three surfaces always agree. An empty value list means "show all".
// Presentation-only: never mutates the report.
export function applyFacetFilter(
  clusters: ReportCluster[],
  filter: FacetFilter,
): ReportCluster[] {
  const { severities } = filter;
  if (severities.length === 0) return clusters;
  return clusters.filter((cluster) => severities.includes(clusterBand(cluster)));
}
