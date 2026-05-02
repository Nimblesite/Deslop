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

alias ReportCluster = ReportClusterJson
