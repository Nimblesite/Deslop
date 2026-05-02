typeDiagram

# [LIVE-IPC-SOCKET] JSON-RPC payload models shared by deslop-lsp and deslop-mcp.
type OllamaModelInfo {
  name: String
  bare_id: String
  digest: String
  size_bytes: Int
  is_embedding_model: Bool
}

type EmbeddingModelInfo {
  provider_id: String
  model_id: String
  model_version: Option<String>
  dimensions: Option<Int>
  recommended: Bool
  reachable: Bool
}

union FindSimilarInput {
  OpenRange {
    path: String
    start_byte: Int
    end_byte: Int
  }
  Snippet {
    snippet: String
    language: String
  }
}

type FindSimilarRequest {
  input: FindSimilarInput
  max_results: Option<Int>
}

type FindSimilarResult {
  clusters: List<ReportCluster>
  below_min_nodes: Bool
}

type FileReport {
  path: String
  clusters: List<ReportCluster>
}

type SessionConfig {
  workspace_root: String
  min_nodes: Int
  languages: List<String>
  embedding_provenance: Option<EmbeddingProvenance>
  exclusion_config_path: Option<String>
  cache_root: String
  incremental: Bool
}

type ChangeSummary {
  clusters_added: Int
  clusters_removed: Int
  clusters_updated: Int
  worst_weight: Float
}

type ReportChangedNotification {
  generation: Int
  summary: ChangeSummary
}

union EmbeddingPhase {
  Queued
  Starting
  Running
  Complete
  Failed
}

type EmbeddingProgress {
  phase: EmbeddingPhase
  provider_id: String
  model_id: String
  done: Int
  total: Int
  message: Option<String>
}

union AnalysisState {
  Idle
  Running { started_at_ms: Int }
  Errored { message: String }
}

# ---------------------------------------------------------------------------
# Domain wire types — also travel on the live wire, also the JSON the CLI
# writes to disk. Extracted from `crates/deslop-core/src/{report,delta,
# report_metrics,report_boilerplate,report_hints}.rs`.
# ---------------------------------------------------------------------------

type ActionHint {
  pattern: String
  recommendation: String
}

union ThresholdSource {
  Cli
  Config
  None
}

type ThresholdSummary {
  percent: Float
  breached: Bool
  source: ThresholdSource
}

type RepoMetrics {
  analysed_loc: Int
  duplicated_loc: Int
  duplication_percent: Float
  clusters_total: Int
  duplicated_files: Int
  threshold: ThresholdSummary
}

type ReportBoilerplateOccurrence {
  path: String
  start_byte: Int
  end_byte: Int
}

type ReportBoilerplateHint {
  kind: String
  language: String
  severity: String
  recommendation: String
  occurrences: List<ReportBoilerplateOccurrence>
}

type CacheStats {
  hits: Int
  misses: Int
}

type EmbeddingProvenance {
  provider_id: String
  model_id: String
  model_version: String
  dimensions: Int
  attempted_subtrees: Int
  indexed_subtrees: Int
  failed_subtrees: Int
}

type ReportSignals {
  structural: Float
  token_jaccard: Float
  embedding_cos: Float
  fused: Float
}

type ReportOccurrence {
  path: String
  start_byte: Int
  end_byte: Int
  hidden: Bool
}

type ReportCluster {
  id: String
  weight: Float
  size: Int
  canonical_node_count: Int
  signals: ReportSignals
  bucket: String
  occurrences: List<ReportOccurrence>
  occurrences_total: Int
  occurrences_truncated: Bool
  summary: String
  interpretation: String
}

type Report {
  report_schema_version: Int
  tool_version: String
  min_nodes: Int
  files_analysed: Int
  clusters_hidden: Int
  cache_stats: CacheStats
  metrics: RepoMetrics
  schema_doc: String
  action_hints: List<ActionHint>
  boilerplate_hints: List<ReportBoilerplateHint>
  embedding_provenance: Option<EmbeddingProvenance>
  clusters: List<ReportCluster>
}

type ReportDelta {
  from_generation: Int
  to_generation: Int
  clusters_added: List<ReportCluster>
  clusters_removed: List<String>
  clusters_updated: List<ReportCluster>
  cache_stats: CacheStats
  tool_version: String
}

# ---------------------------------------------------------------------------
# `deslop/*` LSP custom-method request param shapes ([LSP-CUSTOM-METHODS]).
# These ride directly on the JSON-RPC `params` slot and travel from the
# VSIX (and other LSP clients) to deslop-lsp.
# ---------------------------------------------------------------------------

type PathParams {
  path: String
}

type ReportDeltaParams {
  since_generation: Option<Int>
}

type RangeParams {
  path: String
  start_byte: Int
  end_byte: Int
}

type ClusterIdParams {
  id: String
}

type VirtualDocumentParams {
  uri: String
}

type SetModelParams {
  provider_id: String
  model_id: String
  endpoint: Option<String>
}

# All types referenced by name are now defined here. EXTERNAL_TYPES in
# the generator can stay populated as a safety net for legacy callers.
