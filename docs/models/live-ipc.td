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

# External references resolved by the generator's IMPORTS map:
#   ReportCluster        -> crate::report::ReportCluster
#   EmbeddingProvenance  -> crate::report::EmbeddingProvenance
