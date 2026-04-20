// Mirrors codededup-core::report at REPORT_SCHEMA_VERSION = 3.
// Keep in sync with crates/codededup-core/src/report.rs and report_metrics.rs.

export interface Report {
  report_schema_version: number;
  tool_version: string;
  min_nodes: number;
  files_analysed: number;
  clusters_hidden: number;
  cache_stats: CacheStats;
  metrics: RepoMetrics;
  schema_doc: string;
  action_hints: ActionHint[];
  embedding_provenance: EmbeddingProvenance | null;
  clusters: ReportCluster[];
}

export interface CacheStats {
  hits: number;
  misses: number;
}

export interface EmbeddingProvenance {
  provider_id: string;
  model_id: string;
  model_version: string;
  dimensions: number;
}

export interface ReportCluster {
  id: string;
  weight: number;
  size: number;
  canonical_node_count: number;
  signals: ReportSignals;
  occurrences: ReportOccurrence[];
  summary: string;
  interpretation: string;
}

export interface ReportSignals {
  structural: number;
  token_jaccard: number;
  embedding_cos: number;
  fused: number;
}

export interface ReportOccurrence {
  path: string;
  start_byte: number;
  end_byte: number;
  hidden: boolean;
}

export interface ActionHint {
  pattern: string;
  recommendation: string;
}

export interface RepoMetrics {
  analysed_loc: number;
  duplicated_loc: number;
  duplication_percent: number;
  clusters_total: number;
  duplicated_files: number;
  threshold: ThresholdSummary;
}

export interface ThresholdSummary {
  percent: number;
  breached: boolean;
  source: ThresholdSource;
}

export type ThresholdSource = "none" | "cli" | "config";

// ReportDelta (codededup_core::delta) — live updates.
export interface ReportDelta {
  from_generation: number;
  to_generation: number;
  clusters_added: ReportCluster[];
  clusters_removed: string[];
  clusters_updated: ReportCluster[];
  cache_stats: CacheStats;
  tool_version: string;
}

// Push notification payloads.
export interface ChangeSummary {
  clusters_added: number;
  clusters_removed: number;
  clusters_updated: number;
  worst_weight: number;
}

export interface ReportChangedNotification {
  generation: number;
  summary: ChangeSummary;
}

export type AnalysisState = "idle" | "running" | "errored";

// embedding/listModels result.
export interface EmbeddingModelInfo {
  provider_id: string;
  model_id: string;
  model_version: string;
  dimensions: number | null;
  size_bytes: number | null;
  is_embedding_model: boolean;
}

// session/config.
export interface SessionConfig {
  min_nodes: number;
  languages: string[];
  embedding_provenance: EmbeddingProvenance | null;
  exclusion_config_path: string | null;
  cache_dir: string;
}

// Severity bucketing per [LSP-SEVERITY].
export type Severity = "worst" | "top10" | "mid" | "faint";

export function severityOf(weightPercentile: number): Severity {
  if (weightPercentile >= 0.99) return "worst";
  if (weightPercentile >= 0.9) return "top10";
  if (weightPercentile >= 0.5) return "mid";
  return "faint";
}

// verdict per [VSIX-LIVE-BUBBLE].
export type Verdict = "DUPLICATE" | "NEAR-MISS" | "SEMANTIC MATCH";

export function verdictOf(signals: ReportSignals): Verdict {
  if (signals.structural >= 1.0) return "DUPLICATE";
  if (signals.token_jaccard >= 0.9) return "NEAR-MISS";
  return "SEMANTIC MATCH";
}

export const FUSED_THRESHOLD = 0.85;
