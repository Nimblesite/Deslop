typeDiagram

# [LIVE-IPC-TCP] On-disk discovery record for the TCP loopback IPC
# transport. Written to `.deslop/cache/deslop.port` by deslop-lsp when
# TCP carries the IPC protocol (default on Windows, opt-in elsewhere via
# `--ipc-transport tcp`); read by deslop-mcp to locate and authenticate
# against the live server ([MCP-IPC-DISCOVERY]).
type IpcEndpointFile {
  port: Int
  token: String
}

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
  total_occurrences: Int
}

type FileReport {
  path: String
  clusters: List<ReportCluster>
  total_occurrences: Int
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
  literal_findings_added: Int
  literal_findings_removed: Int
  literal_findings_updated: Int
  worst_mass: Int
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
  percent: Float
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

type FileMetric {
  path: String
  analysed_loc: Int
  duplicated_loc: Int
  duplication_percent: Float
}

type DiffMetrics {
  added_loc: Int
  duplicated_added_loc: Int
  duplication_percent: Float
  threshold: ThresholdSummary
}

type RepoMetrics {
  analysed_loc: Int
  duplicated_loc: Int
  duplication_percent: Float
  clusters_total: Int
  duplicated_files: Int
  threshold: ThresholdSummary
  per_file: List<FileMetric>
  folders: List<FileMetric>
  diff: Option<DiffMetrics>
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
  succeeded_subtrees: Int
  indexed_subtrees: Int
  failed_subtrees: Int
}

type ReportOccurrence {
  path: String
  start_byte: Int
  end_byte: Int
  start_line: Int
  end_line: Int
  hidden: Bool
  in_diff: Option<Bool>
}

type ReportCluster {
  id: String
  rank: Int
  rank_band: String
  mass: Int
  canonical_node_count: Int
  occurrences: List<ReportOccurrence>
  occurrences_total: Int
  occurrence_count: Int
  occurrences_truncated: Bool
  intersects_diff: Option<Bool>
  is_newly_introduced: Option<Bool>
}

# [FUSED-PAIR-SIGNALS] Pair evidence exists only for the two exact endpoints
# named by an explicit comparison. It is never copied to ReportCluster.
type PairEndpoint {
  path: String
  start_byte: Int
  end_byte: Int
}

type PairComparisonParams {
  left: PairEndpoint
  right: PairEndpoint
}

type PairEvidence {
  structural: Float
  token_jaccard: Float
  embedding_cos: Float
  agreement: Float
  rename_consistency: Float
  literal_fraction: Float
  fused_score: Float
  content_required: Bool
  content_ok: Bool
  admitted: Bool
  classification: Option<PairClassification>
  explanation: String
}

union PairClassification {
  Identical
  NearlyIdentical
  StructuralOnly
  LooselySimilar
  SameBehavior
}

type PairComparison {
  left: PairEndpoint
  right: PairEndpoint
  evidence: PairEvidence
}

# [LITERAL-WIRE] Literal findings are a separate report family. They never
# carry clone-pair evidence and are never embedded in ReportCluster.
union LiteralFindingKind {
  MagicLiteral
  ShadowedConstant
  ConstantDuplicate
  ConstantDrift
  ConstantAlias
}

type CanonicalTarget {
  path: String
  line: Int
  constant_name: Option<String>
}

type LiteralOccurrence {
  path: String
  start_byte: Int
  end_byte: Int
  start_line: Int
  end_line: Int
  hidden: Bool
  constant_value: Option<String>
  container: Option<String>
  unused_confidence: Option<Int>
}

type LiteralFinding {
  id: String
  kind: LiteralFindingKind
  mass: Int
  occurrences: List<LiteralOccurrence>
  constant_name: Option<String>
  literal_value: Option<String>
  canonical_target: Option<CanonicalTarget>
}

type Report {
  tool_version: String
  min_nodes: Int
  files_analysed: Int
  clusters_hidden: Int
  cache_stats: CacheStats
  metrics: RepoMetrics
  schema_doc: String
  boilerplate_hints: List<ReportBoilerplateHint>
  embedding_provenance: Option<EmbeddingProvenance>
  clusters: List<ReportCluster>
  clusters_outside_diff: Option<Int>
  literal_findings: List<LiteralFinding>
  literal_findings_total: Int
  literal_findings_hidden: Int
  literal_findings_capped: Bool
  literal_max_findings: Int
}

type ReportDelta {
  from_generation: Int
  to_generation: Int
  clusters_added: List<ReportCluster>
  clusters_removed: List<String>
  clusters_updated: List<ReportCluster>
  literal_findings_added: List<LiteralFinding>
  literal_findings_removed: List<String>
  literal_findings_updated: List<LiteralFinding>
  metrics: RepoMetrics
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

# ---------------------------------------------------------------------------
# JSON-RPC 2.0 envelope ([MCP-JSONRPC]). Rides the MCP stdio transport.
# Generator emits these with `skipTs: true` because the VSIX uses VS Code's
# built-in LSP machinery and never sees raw JSON-RPC frames; emitting TS
# placeholder shapes for `params`/`result`/`data` (which carry arbitrary
# JSON) would actively misrepresent the runtime payload.
# ---------------------------------------------------------------------------

# Request or notification identifier. JSON-RPC permits strings or integers
# (and `null` via `Option<RequestId>` in containing structs). Modelled as
# an untagged enum where each variant is the bare wire value — see the
# `tupleVariants` + `serdeAttrs: ['untagged']` config that collapses the
# struct-form variants typeDiagram emits into tuple variants serde can
# round-trip transparently.
union RequestId {
  Number { value: Int }
  String { value: String }
}

# Canonical JSON-RPC / MCP error codes ([MCP-JSONRPC]). Numeric
# discriminants pinned via TYPE_CONFIG.variantDiscriminants because the
# wire contract depends on the exact integer value (-32_700 .. -32_001).
union ErrorCode {
  ParseError
  InvalidRequest
  MethodNotFound
  InvalidParams
  InternalError
  UnparseableInput
  UnsupportedLanguage
  PathOutsideRoot
  BackendError
}

# JSON-RPC 2.0 error object. `data` carries an arbitrary structured
# payload (tree-sitter error ranges, list of supported languages, etc.)
# — declared here as `String` so the generator can override to
# `Option<serde_json::Value>` on the Rust side; `skip_serializing_if`
# omits the key entirely when None per the JSON-RPC spec.
type JsonRpcError {
  code: Int
  message: String
  data: Option<String>
}

# JSON-RPC 2.0 incoming frame (request or notification). `params` is
# arbitrary JSON; declared as `String` for the .td placeholder and
# overridden to `Option<serde_json::Value>` in TYPE_CONFIG. A frame
# without `id` is a notification (no reply).
type JsonRpcRequest {
  jsonrpc: String
  method: String
  params: Option<String>
  id: Option<RequestId>
}

# JSON-RPC 2.0 successful response frame. `result` is arbitrary JSON
# (declared as `String` placeholder, overridden in TYPE_CONFIG).
type JsonRpcResponse {
  jsonrpc: String
  id: RequestId
  result: String
}

# JSON-RPC 2.0 error response frame. Distinct from JsonRpcResponse
# because the wire shape carries `error` instead of `result` (no
# `result` key emitted on error). `id` is `Option<RequestId>` because
# the request id may be unknown if the request was unparseable.
type JsonRpcErrorResponse {
  jsonrpc: String
  id: Option<RequestId>
  error: JsonRpcError
}

# ---------------------------------------------------------------------------
# `LiveError` wire shape ([LIVE-PACKAGING]). Carried as `data` on JSON-RPC
# error frames so transports never lose the structured fault context.
# ---------------------------------------------------------------------------

type LiveErrorWire {
  code: String
  message: String
}

# ---------------------------------------------------------------------------
# MCP `duplicates` page payload ([MCP-TOOL-DUPLICATES]). Summary and full
# rows are untagged wire alternatives selected by `detail`; neither carries
# pair evidence. `compare-pair` uses PairComparisonParams/PairComparison.
# ---------------------------------------------------------------------------

type OccurrenceSummary {
  path: String
  start_byte: Int
  end_byte: Int
  start_line: Int
  end_line: Int
}

type ClusterSummary {
  id: String
  rank: Int
  rank_band: String
  mass: Int
  size_nodes: Int
  occurrence_count: Int
  language: String
  first_occurrence: Option<OccurrenceSummary>
}

type ReportPageInfo {
  offset: Int
  limit: Int
  returned: Int
}

type DuplicatesFilters {
  languages: Option<List<String>>
  path_contains: Option<String>
  severities: Option<List<String>>
  min_size: Option<Int>
}

union DuplicateCluster {
  Summary { value: ClusterSummary }
  Full { value: ReportCluster }
}

type DuplicatesPage {
  tool_version: String
  generation: Int
  files_analysed: Int
  min_nodes: Int
  clusters_hidden: Int
  embedding_provenance: Option<EmbeddingProvenance>
  cache_stats: CacheStats
  metrics: RepoMetrics
  total_clusters: Int
  total_occurrences: Int
  page: ReportPageInfo
  clusters: List<DuplicateCluster>
  filters: DuplicatesFilters
}

# ---------------------------------------------------------------------------
# MCP `schema-doc` payload. The schema markdown is large and deliberately
# lives behind this one-shot tool instead of every report page.
# ---------------------------------------------------------------------------

type SchemaDocPayload {
  schema_doc: String
}

# ---------------------------------------------------------------------------
# MCP `rescan` payload ([MCP-TOOLS]). The refreshed report uses the exact
# `DuplicatesPage` shape; no parallel report-query payload survives.
# ---------------------------------------------------------------------------

type RescanPayload {
  generation: Int
  summary: ChangeSummary
  page: DuplicatesPage
}

# ---------------------------------------------------------------------------
# MCP `list-embedding-models` payload.
# ---------------------------------------------------------------------------

type EmbeddingModelList {
  models: List<EmbeddingModelInfo>
}

# ---------------------------------------------------------------------------
# MCP `set-embedding-model` response. Mirrors `EmbeddingSpec` plus the
# `dimensions` field so the agent does not need a follow-up call.
# ---------------------------------------------------------------------------

type SetEmbeddingModelResponse {
  provider_id: String
  model_id: String
  model_version: String
  dimensions: Int
}

# ---------------------------------------------------------------------------
# MCP `session` tool payload. Distinct from the LSP `SessionConfig`
# above (richer, MCP-specific keys: `root` (not `workspace_root`),
# `cache_stats`, `generation`).
# ---------------------------------------------------------------------------

type McpSessionConfig {
  root: String
  min_nodes: Int
  languages: List<String>
  incremental: Bool
  embedding_provenance: Option<EmbeddingProvenance>
  cache_stats: CacheStats
  generation: Int
}

# ---------------------------------------------------------------------------
# MCP `merge-plan` payload ([AUTOFIX-MERGE], [AUTOFIX-MERGE-MCP]). The
# mechanical call-site merge for one cluster: the anti-unified helper, the
# derived parameters (each with its per-site argument list + optional default
# value), the mechanical/AI verdict, and the WorkspaceEdit the host applies.
# The request reuses `ClusterIdParams`. Read-only — computing a plan never
# writes files.
# ---------------------------------------------------------------------------

# Whether the cluster can be merged with no AI in the loop. `AiOrHuman`
# carries the routing reason ([AUTOFIX-MERGE-GATE] / [AUTOFIX-MERGE-SAFETY]).
union MergeVerdict {
  Mechanical
  AiOrHuman { reason: String }
}

# One parameter of the synthesised helper. `per_site_arguments` holds the
# concrete expression each occurrence passes, in cluster-occurrence order
# (the substitutions recovered by anti-unification). `is_thunk` is true when
# the difference is passed as a deferred closure ([AUTOFIX-MERGE-SAFETY] D);
# `default_value` is set only for a constant-across-sites position
# ([AUTOFIX-MERGE-NAMES]).
type MergeParameter {
  name: String
  type_name: String
  is_thunk: Bool
  is_required: Bool
  default_value: Option<String>
  per_site_arguments: List<String>
}

# `helper_body` is the anti-unifier template (the new function body).
# `workspace_edit` is the LSP WorkspaceEdit serialised as arbitrary JSON —
# declared `String` here so the generator overrides it to
# `Option<serde_json::Value>` on the Rust side (same pattern as
# `JsonRpcError.data`); `None` when the verdict is `AiOrHuman`.
type MergePlan {
  cluster_id: String
  language: String
  verdict: MergeVerdict
  helper_name: String
  helper_body: String
  parameters: List<MergeParameter>
  workspace_edit: Option<String>
}

# All types referenced by name are now defined here. EXTERNAL_TYPES in
# the generator can stay populated as a safety net for legacy callers.
