//! Wire-level names, pointers and numeric knobs shared by several
//! `deslop-mcp` CLI suites. One spelling per wire token, so a contract
//! change fails every suite that cites it instead of just the one that
//! happens to own the string.

/// Tool name: the mass-ranked duplicates page.
pub const DUPLICATES_TOOL: &str = "duplicates";
/// Tool name: session state + embedding-model actions.
pub const SESSION_TOOL: &str = "session";
/// Tool name: the pair-evidence deep dive.
pub const COMPARE_PAIR_TOOL: &str = "compare-pair";
/// Tool name: the single-cluster deep dive.
pub const CLUSTER_BY_ID_TOOL: &str = "cluster-by-id";
/// Tool name: the pre-copy prevention lookup.
pub const FIND_SIMILAR_TOOL: &str = "find-similar";
/// Tool name: synchronous re-analysis with a fresh page.
pub const RESCAN_TOOL: &str = "rescan";
/// Tool name: the one-shot report-schema markdown.
pub const SCHEMA_DOC_TOOL: &str = "schema-doc";
/// JSON-RPC method: tool advertisement.
pub const TOOLS_LIST_METHOD: &str = "tools/list";
/// JSON-RPC method: tool invocation.
pub const TOOLS_CALL_METHOD: &str = "tools/call";
/// JSON field: `mass`, the cluster's size-weighted duplicate load.
pub const MASS_FIELD: &str = "mass";
/// JSON field: the mass-derived rank band.
pub const SEVERITY_FIELD: &str = "severity";
/// JSON field: page detail level (`full` / `summary`).
pub const DETAIL_FIELD: &str = "detail";
/// `detail` value: slim per-cluster rows.
pub const DETAIL_SUMMARY: &str = "summary";
/// JSON field: severity filter list.
pub const SEVERITIES_FIELD: &str = "severities";
/// JSON field: occurrence-path substring filter.
pub const PATH_CONTAINS_FIELD: &str = "path_contains";
/// JSON field: language filter list.
pub const LANGUAGES_FIELD: &str = "languages";
/// JSON field / argument: page window start.
pub const OFFSET_PARAM: &str = "offset";
/// JSON field / argument: page window length.
pub const LIMIT_PARAM: &str = "limit";
/// JSON pointer: the `clusters[]` page body.
pub const CLUSTERS_POINTER: &str = "/clusters";
/// JSON pointer: the JSON-RPC error code.
pub const ERROR_CODE_POINTER: &str = "/error/code";
/// JSON pointer: the unfiltered cluster count.
pub const TOTAL_CLUSTERS_POINTER: &str = "/total_clusters";
/// JSON field: a workspace-relative file scope.
pub const PATH_FIELD: &str = "path";
/// JSON field: snippet language, or a cluster's detected language.
pub const LANGUAGE_FIELD: &str = "language";
/// Language id the fixtures are planted in.
pub const CSHARP_LANGUAGE: &str = "csharp";
/// JSON field: the JSON-RPC error envelope.
pub const ERROR_FIELD: &str = "error";
/// JSON field: occurrence end offset, exclusive.
pub const END_BYTE_FIELD: &str = "end_byte";
/// JSON field: cluster identity / request id.
pub const ID_FIELD: &str = "id";
/// JSON field: the arguments object of a `tools/call`.
pub const ARGUMENTS_FIELD: &str = "arguments";
/// JSON field: occurrence start offset.
pub const START_BYTE_FIELD: &str = "start_byte";
/// Fixture file that participates in the planted clone family.
pub const ALPHA_FILE_NAME: &str = "Alpha.cs";
/// JSON field: the retired bucket label.
pub const BUCKET_FIELD: &str = "bucket";
/// JSON field: the inline snippet a find-similar carries.
pub const SNIPPET_FIELD: &str = "snippet";
/// JSON field: minimum `size_nodes` filter.
pub const MIN_SIZE_FIELD: &str = "min_size";
/// JSON pointer: the session's detected-language list.
pub const LANGUAGES_POINTER: &str = "/languages";
/// JSON pointer: embedding provenance inside a session snapshot.
pub const EMBEDDING_PROVENANCE_POINTER: &str = "/embedding_provenance";
/// MCP resource uri: the live JSON report.
pub const REPORT_URI: &str = "deslop://report";
/// JSON field: the `paths[]` a rescan/filesChanged names.
pub const PATHS_FIELD: &str = "paths";
/// CLI flag: the workspace root the server binds.
pub const ROOT_FLAG: &str = "--root";
/// JSON pointer: the advertised tool array.
pub const TOOLS_LIST_POINTER: &str = "/result/tools";
/// JSON pointer: a frame's id, for round-trip checks.
pub const ID_POINTER: &str = "/id";
/// JSON field: the report schema markdown blob.
pub const SCHEMA_DOC_FIELD: &str = "schema_doc";
/// JSON field: the occurrence list of a full cluster.
pub const OCCURRENCES_FIELD: &str = "occurrences";
/// `serverInfo.name` the server must advertise.
pub const MCP_PROGRAM_NAME: &str = "deslop-mcp";
/// Page limit the broad-result queries request.
pub const QUERY_PAGE_LIMIT: u64 = 50;
/// Limit for the broadest page a mutation scenario compares.
pub const BROAD_RESULT_LIMIT: u64 = 100;
/// The live LSP's default minimum node floor.
pub const DEFAULT_MIN_NODES: u64 = 30;
/// Limit / `top_n` requested where a small page is asserted.
pub const REQUESTED_TOP_OFFENDERS_COUNT: usize = 3;
/// Page limit the server defaults to when the caller omits one.
pub const SMALL_PAGE_LIMIT: u64 = 5;
