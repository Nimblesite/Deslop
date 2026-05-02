#!/usr/bin/env node
// Generates Rust IPC model code from `docs/models/live-ipc.td` using the
// typediagram CLI (https://typediagram.dev/docs/cli.html), then post-
// processes the output to satisfy the Deslop workspace lints (serde
// derives, doc comments, precise integer widths, serde tag attributes,
// import statements). The .td file is the single source of truth; the
// emitted `.rs` is gitignored and rebuilt on every `cargo build` via
// `crates/deslop-core/build.rs`.
//
// Per CLAUDE.md: "ALL MODELS TRANSFERRED ACROSS THE WIRE MUST USE
// typeDiagram. NO IFS. NO BUTS." This script is the build-side adapter
// from the typediagram CLI's bare struct output to wire-ready Rust.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..");
const TD_PATH = resolve(REPO_ROOT, "docs/models/live-ipc.td");
const OUT_RUST = resolve(
  REPO_ROOT,
  "crates/deslop-core/src/wire_generated.rs",
);
const OUT_TS = resolve(
  REPO_ROOT,
  "clients/vscode/src/types/wire-generated.ts",
);
// External TypeScript types (defined in clients/vscode/src/types/report.ts)
// re-imported by the generated TS file when referenced. Skipped
// automatically when the type is defined here (TYPE_CONFIG entry exists).
const EXTERNAL_TS_TYPES = {
  ReportCluster: "./report",
  EmbeddingProvenance: "./report",
  CacheStats: "./report",
};

// Per-type generation hints. Drives the post-processor: every entry maps
// a type name (struct, enum, or alias) to the derives, serde attrs,
// field-type overrides, and crate-level `use` lines required to make
// the bare typediagram output compile against the existing wire shape.
const TYPE_CONFIG = {
  OllamaModelInfo: {
    docs: "One row from the Ollama `/api/tags` enumeration. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { size_bytes: "u64" },
    fieldDocs: {
      name: "Full model tag as installed (`nomic-embed-text:latest`).",
      bare_id: "Tag-stripped model id.",
      digest: "Truncated content digest (12 hex chars).",
      size_bytes: "Packaged model size in bytes.",
      is_embedding_model: "True when a probe returned a non-empty vector.",
    },
  },
  EmbeddingModelInfo: {
    docs: "One row of the `embedding/listModels` response. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { dimensions: "Option<usize>" },
    fieldDocs: {
      provider_id: "Provider registry key (`ollama`, `stub`).",
      model_id: "Human-readable model id.",
      model_version: "Optional opaque version string.",
      dimensions: "Optional dimensionality, when known.",
      recommended: "True when recommended for code embeddings.",
      reachable: "True when the provider was reachable at listing time.",
    },
  },
  FindSimilarInput: {
    docs: "Discriminated input to `duplicates/findSimilar`. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    serdeAttrs: ['tag = "kind"', 'rename_all = "snake_case"'],
    fieldOverrides: {
      path: "PathBuf",
      start_byte: "usize",
      end_byte: "usize",
    },
    variantDocs: {
      OpenRange: "Look up clusters overlapping a byte range in an open file.",
      Snippet: "Parse a snippet against a registered language and look up.",
    },
    fieldDocs: {
      path: "Workspace-relative or absolute path.",
      start_byte: "Inclusive byte offset of the range start.",
      end_byte: "Exclusive byte offset of the range end.",
      snippet: "Source-text snippet to fingerprint.",
      language: "Registered language id (`csharp`, `rust`, `python`).",
    },
  },
  FindSimilarRequest: {
    docs: "Outer envelope for `duplicates/findSimilar` requests. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { max_results: "Option<usize>" },
    fieldDocs: {
      input: "Discriminated input variant.",
      max_results: "Optional cap on returned clusters; `None` means no cap.",
    },
  },
  FindSimilarResult: {
    docs: "Result of `duplicates/findSimilar`. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      clusters: "Top-N clusters covering the input, worst-first.",
      below_min_nodes:
        "True when every subtree fell below the session's `min_nodes` floor.",
    },
  },
  FileReport: {
    docs: "File-scoped subset of a report; returned by `report/forFile`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { path: "PathBuf" },
    fieldDocs: {
      path: "Path the report covers, workspace-relative when possible.",
      clusters: "Clusters whose occurrences touch `path`, byte-range sorted.",
    },
  },
  SessionConfig: {
    docs: "Snapshot of the session's resolved configuration.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      workspace_root: "PathBuf",
      min_nodes: "u32",
      exclusion_config_path: "Option<PathBuf>",
      cache_root: "PathBuf",
    },
    fieldDocs: {
      workspace_root: "Workspace root pinned at session creation.",
      min_nodes: "Subtree-size floor used throughout the session.",
      languages: "Languages with registered parsers in the session.",
      embedding_provenance: "Currently-active embedding provenance, if any.",
      exclusion_config_path: "Optional explicit exclusion-config path.",
      cache_root: "Cache root (`<workspace>/.deslop-cache`).",
      incremental: "Whether the session was created with the incremental cache on.",
    },
  },
  ChangeSummary: {
    docs: "Compact summary of a `ReportDelta` for push notifications.",
    derives: ["Debug", "Clone", "Default", "Serialize", "Deserialize"],
    fieldOverrides: {
      clusters_added: "usize",
      clusters_removed: "usize",
      clusters_updated: "usize",
    },
    fieldDocs: {
      clusters_added: "Number of clusters newly present in the latest generation.",
      clusters_removed: "Number of clusters removed in the latest generation.",
      clusters_updated: "Number of clusters whose payload changed.",
      worst_weight: "Worst (highest) weight in the latest generation, `0.0` when empty.",
    },
  },
  ReportChangedNotification: {
    docs: "Wire payload for the `report/changed` notification.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { generation: "u64" },
    fieldDocs: {
      generation: "New generation that produced the change.",
      summary: "Compact summary suitable for status indicators.",
    },
  },
  EmbeddingPhase: {
    docs: "Phase of the embedding pass surfaced via `deslop/embeddingProgress`.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize", "PartialEq", "Eq"],
    serdeAttrs: ['rename_all = "snake_case"'],
    variantDocs: {
      Queued: "User selected a model and the low-priority pass is queued.",
      Starting: "Pass has just begun. `done` is `0`, `total` is populated.",
      Running: "Pass is actively working through provider batches.",
      Complete: "Pass finished successfully. `done == total`.",
      Failed: "Pass aborted with `message`. `done` reflects work before the failure.",
    },
  },
  EmbeddingProgress: {
    docs: "Wire payload for the `deslop/embeddingProgress` notification.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { done: "u64", total: "u64" },
    fieldDocs: {
      phase: "Lifecycle phase.",
      provider_id: "Provider id the swap targets (`ollama`, `stub`).",
      model_id: "Model id the swap targets.",
      done: "Subtrees embedded so far.",
      total: "Total subtrees in the current corpus.",
      message: "Diagnostic message populated only when `phase == Failed`.",
    },
  },
  AnalysisState: {
    docs: "Wire payload for the `analysis/state` notification.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    serdeAttrs: ['tag = "state"', 'rename_all = "snake_case"'],
    fieldOverrides: { started_at_ms: "u64" },
    variantDocs: {
      Idle: "Scheduler is idle — no pass in flight.",
      Running: "Scheduler is processing a pass started at `started_at_ms`.",
      Errored: "Scheduler is parked on an error; `message` carries the diagnostic.",
    },
    fieldDocs: {
      started_at_ms: "Millisecond timestamp the pass started.",
      message: "Human-readable diagnostic.",
    },
  },
  ActionHint: {
    docs: "Short agent-oriented playbook entry surfaced at the top of every report.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      pattern: "Matches one of the taxonomy rows (`type-1-2`, `type-3`, ...).",
      recommendation: "One-line recommendation written for an agent reader.",
    },
  },
  ThresholdSource: {
    docs: "Origin of the active fail-over threshold.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize", "PartialEq", "Eq"],
    serdeAttrs: ['rename_all = "lowercase"'],
    variantDocs: {
      Cli: "Threshold passed via `--fail-over`.",
      Config: "Threshold loaded from `[threshold] max_duplication_percent`.",
      None: "No threshold configured (or explicitly cleared via `--no-fail-over`).",
    },
  },
  ThresholdSummary: {
    docs: "Threshold verdict carried on `RepoMetrics`.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize"],
    fieldDocs: {
      percent: "Active threshold as a percentage; `0.0` when source is `none`.",
      breached: "True when measured duplication exceeded `percent`.",
      source: "Provenance of the threshold value.",
    },
  },
  RepoMetrics: {
    docs: "Repo-wide duplication metrics, embedded at `Report.metrics`.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize"],
    fieldOverrides: {
      analysed_loc: "u64",
      duplicated_loc: "u64",
      clusters_total: "usize",
      duplicated_files: "usize",
    },
    fieldDocs: {
      analysed_loc: "Physical lines across every analysed file.",
      duplicated_loc: "Lines covered by `>= 2` non-hidden clone occurrences.",
      duplication_percent: "Clamped `100.0 * duplicated_loc / analysed_loc`.",
      clusters_total: "Count of clusters contributing to `duplicated_loc`.",
      duplicated_files: "Count of files with at least one non-hidden clone occurrence.",
      threshold: "Resolved fail-over threshold.",
    },
  },
  ReportBoilerplateOccurrence: {
    docs: "One suppressed import/prologue range surfaced in the report.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      path: "PathBuf",
      start_byte: "usize",
      end_byte: "usize",
    },
    fieldDocs: {
      path: "Source path, relative to the scan root when possible.",
      start_byte: "Inclusive start byte of the suppressed range.",
      end_byte: "Exclusive end byte of the suppressed range.",
    },
  },
  ReportBoilerplateHint: {
    docs: "Low-severity hygiene hint for import/prologue boilerplate.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      kind: "Hint category. Currently `imports`.",
      language: "Language id the hint applies to.",
      severity: "Always `info` for boilerplate hints.",
      recommendation: "Gentle remediation guidance.",
      occurrences: "Suppressed byte ranges that justify the hint.",
    },
  },
  ReportDelta: {
    docs: "Diff between two report generations.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      from_generation: "u64",
      to_generation: "u64",
    },
    fieldDocs: {
      from_generation: "Generation of the earlier report.",
      to_generation: "Generation of the later report.",
      clusters_added: "Clusters present in `to` but not in `from`, worst-first.",
      clusters_removed: "Cluster ids present in `from` but not in `to`.",
      clusters_updated: "Clusters present in both whose payload changed.",
      cache_stats: "Cache telemetry for the generation-producing run.",
      tool_version: "Producer version stamped on the later snapshot.",
    },
  },
  CacheStats: {
    docs: "Per-run incremental-cache telemetry.",
    derives: ["Debug", "Clone", "Copy", "Default", "Serialize", "Deserialize"],
    fieldOverrides: { hits: "usize", misses: "usize" },
    fieldDocs: {
      hits: "Files resolved from the on-disk fingerprint cache.",
      misses: "Files parsed from scratch.",
    },
  },
  EmbeddingProvenance: {
    docs: "Provenance of the `(provider, model, version)` triple used by the embedding pass.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      dimensions: "usize",
      attempted_subtrees: "usize",
      indexed_subtrees: "usize",
      failed_subtrees: "usize",
    },
    fieldSerdeAttrs: {
      attempted_subtrees: ["default"],
      indexed_subtrees: ["default"],
      failed_subtrees: ["default"],
    },
    tsOptional: ["attempted_subtrees", "indexed_subtrees", "failed_subtrees"],
    fieldDocs: {
      provider_id: "Registry key of the provider (`ollama`).",
      model_id: "Human-readable model identifier.",
      model_version: "Opaque model version / digest reported by the provider.",
      dimensions: "Embedding dimensionality the provider returned.",
      attempted_subtrees: "Number of subtree embeddings requested or served from cache.",
      indexed_subtrees: "Number of unique successful subtree embeddings fed into ANN.",
      failed_subtrees: "Number of subtree embeddings rejected by the provider.",
    },
  },
  ReportSignals: {
    docs: "Per-cluster signal breakdown so consumers can tell why the cluster was flagged.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize"],
    fieldDocs: {
      structural: "Mean structural signal across cluster pairs.",
      token_jaccard: "Mean token Jaccard estimate across cluster pairs.",
      embedding_cos: "Mean embedding cosine similarity across cluster pairs.",
      fused: "Unit-bounded fused confidence from the three components.",
    },
  },
  ReportOccurrence: {
    docs: "A single clone occurrence — a specific `(file, byte_range)`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      path: "PathBuf",
      start_byte: "usize",
      end_byte: "usize",
    },
    fieldDocs: {
      path: "Source path, relative to the scan root when possible.",
      start_byte: "Inclusive byte offset of the clone within the file.",
      end_byte: "Exclusive byte offset of the end of the clone.",
      hidden: "True when the file matches a `report_hide` pattern.",
    },
  },
  ReportCluster: {
    docs: "One cluster as it appears in the rendered report.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      size: "usize",
      canonical_node_count: "usize",
      occurrences_total: "usize",
    },
    fieldSerdeAttrs: {
      bucket: ["default"],
      occurrences_total: ["default"],
      occurrences_truncated: ["default"],
    },
    tsOptional: ["bucket", "occurrences_total", "occurrences_truncated"],
    fieldDocs: {
      id: "Stable short id for cross-referencing.",
      weight: "Ranking weight (higher = worse).",
      size: "Count of cloned occurrences in the cluster.",
      canonical_node_count: "AST node count of one canonical member.",
      signals: "Per-cluster signal breakdown (structural / Jaccard / embedding / fused).",
      bucket: "Canonical bucket label (`identical`, `nearly_identical`, `loosely_similar`, `same_behavior`).",
      occurrences: "Cluster members; live wire caps this list.",
      occurrences_total: "Total occurrences before wire truncation.",
      occurrences_truncated: "True when `occurrences` was truncated for the wire.",
      summary: "Agent-oriented synthesis (blanked on the live wire).",
      interpretation: "Derived one-line interpretation (blanked on the live wire).",
    },
  },
  PathParams: {
    docs: "Wire payload for `deslop/reportForFile` and `deslop/clusterById` (file-scoped lookups).",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { path: "PathBuf" },
    fieldDocs: {
      path: "Workspace-relative or absolute path scoping the request.",
    },
  },
  ReportDeltaParams: {
    docs: "Wire payload for `deslop/reportDelta`.",
    derives: ["Debug", "Clone", "Default", "Serialize", "Deserialize"],
    fieldOverrides: { since_generation: "Option<u64>" },
    fieldDocs: {
      since_generation:
        "Generation the client already has. Missing means \"previous generation.\"",
    },
  },
  RangeParams: {
    docs: "Wire payload for `deslop/reportForRange`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      path: "PathBuf",
      start_byte: "usize",
      end_byte: "usize",
    },
    fieldDocs: {
      path: "Path scoping the range.",
      start_byte: "Inclusive start byte.",
      end_byte: "Exclusive end byte.",
    },
  },
  ClusterIdParams: {
    docs: "Wire payload for `deslop/clusterById`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      id: "Stable cluster id.",
    },
  },
  VirtualDocumentParams: {
    docs: "Wire payload for `deslop/virtualDocument` ([LSP-EDITOR-SURFACES]).",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      uri: "`deslop://{schema|report|cluster/<id>}` URI.",
    },
  },
  SetModelParams: {
    docs: "Wire payload for `deslop/embeddingSetModel`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      provider_id: "Provider registry key.",
      model_id: "Model identifier.",
      endpoint: "Optional endpoint override.",
    },
  },
  LiveErrorWire: {
    docs: "Serialisable wire shape for `deslop_core::live::LiveError`. Carried as `data` on JSON-RPC error frames so transports never lose the structured fault context.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      code: "Short machine-readable identifier (e.g. `\"unparseable_input\"`).",
      message: "Human-readable rendering, equivalent to `format!(\"{err}\")`.",
    },
  },
  OccurrenceSummary: {
    docs: "Single representative occurrence on a `ClusterSummary`. Bytes are the native unit on `ReportOccurrence`; agents convert to lines on demand.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { start_byte: "usize", end_byte: "usize" },
    fieldDocs: {
      path: "Workspace-relative path of the occurrence.",
      start_byte: "Inclusive byte offset of the clone within the file.",
      end_byte: "Exclusive byte offset of the end of the clone.",
    },
  },
  ClusterSummary: {
    docs: "Slim, agent-sized projection of a `ReportCluster`. Drops `members` + full `occurrences` arrays — those live behind `cluster-by-id`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { size_nodes: "usize", occurrence_count: "usize" },
    fieldDocs: {
      id: "Stable 16-char id; pass to `cluster-by-id` for the full record.",
      bucket: "Canonical bucket label ([CLONE-BUCKETS]).",
      score: "Worst-first ranking score; mirrors `ReportCluster.weight`.",
      size_nodes: "Representative subtree node count (`canonical_node_count`).",
      occurrence_count: "Total occurrences across the cluster, taken from `occurrences_total` so wire-truncated counts surface honestly.",
      language: "Detected source language for the first occurrence (`csharp`, `rust`, `python`, ...) or `\"unknown\"`.",
      first_occurrence: "Representative occurrence so the agent can navigate without fetching the full cluster.",
    },
  },
  ReportPageInfo: {
    docs: "Pagination cursor echoed on every `ReportPage`.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize"],
    fieldOverrides: { offset: "usize", limit: "usize", returned: "usize" },
    fieldDocs: {
      offset: "Zero-based cluster index this page started at.",
      limit: "Maximum number of clusters requested.",
      returned: "Actual number of clusters in this page (`<= limit`).",
    },
  },
  ReportPageFilters: {
    docs: "Filter knobs accepted by `report-query`. All combine with logical AND; absent fields match everything.",
    derives: ["Debug", "Clone", "Default", "Serialize", "Deserialize"],
    fieldOverrides: { min_size: "Option<usize>" },
    fieldSerdeAttrs: {
      language: ['skip_serializing_if = "Option::is_none"'],
      bucket: ['skip_serializing_if = "Option::is_none"'],
      path_contains: ['skip_serializing_if = "Option::is_none"'],
      min_score: ['skip_serializing_if = "Option::is_none"'],
      min_size: ['skip_serializing_if = "Option::is_none"'],
    },
    fieldDocs: {
      language: "Match clusters whose detected language equals this id.",
      bucket: "Match clusters whose canonical bucket equals this id.",
      path_contains: "Match clusters where any occurrence path contains this case-sensitive substring.",
      min_score: "Match clusters whose `weight` is `>= min_score`.",
      min_size: "Match clusters whose `canonical_node_count` is `>= min_size`.",
    },
  },
  ReportPage: {
    docs: "Paginated `report-get` / `report-query` response. Carries headline metrics plus a slim slice of `ClusterSummary` rows.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      report_schema_version: "u32",
      generation: "u64",
      files_analysed: "usize",
      min_nodes: "u32",
      clusters_hidden: "usize",
      total_clusters: "usize",
    },
    fieldSerdeAttrs: {
      filters: ['skip_serializing_if = "Option::is_none"'],
    },
    fieldDocs: {
      report_schema_version: "Stable schema version so agent consumers can parse defensively.",
      tool_version: "Binary / library version that produced the report.",
      schema_doc: "Markdown schema explanation.",
      generation: "Generation counter at the time the page was rendered.",
      files_analysed: "Number of files analysed in the source report.",
      min_nodes: "Minimum subtree node count used for clustering.",
      clusters_hidden: "Clusters hidden because every member matched a `report_hide` pattern.",
      embedding_provenance: "Provider/model/version that produced the embedding signals, if any.",
      cache_stats: "Incremental-cache hit / miss counters.",
      metrics: "Repo-wide duplication totals.",
      action_hints: "Short agent-oriented playbook.",
      total_clusters: "Count of clusters in the matched (filtered, pre-paginated) set.",
      page: "Pagination cursor for this page.",
      clusters: "Page slice of cluster summaries.",
      filters: "Echoed active filters; absent when no filter was applied.",
    },
  },
  TopOffendersPayload: {
    docs: "Wire payload for `top-offenders` and `rescan` MCP tools.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { total_clusters: "usize", n: "usize" },
    fieldDocs: {
      total_clusters: "Total clusters in the report (pre-truncation).",
      n: "Cap requested by the agent.",
      clusters: "Top `n` clusters, worst-first.",
    },
  },
  RangeReport: {
    docs: "Wire payload for `report-for-range`. Echoes the request range so the agent can correlate without rebuilding the call.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { path: "PathBuf", start_byte: "usize", end_byte: "usize" },
    fieldDocs: {
      path: "Path scoping the range.",
      start_byte: "Inclusive start byte echoed from the request.",
      end_byte: "Exclusive end byte echoed from the request.",
      clusters: "Clusters overlapping the range.",
    },
  },
  EmbeddingModelList: {
    docs: "Wire payload for `list-embedding-models`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      models: "Models currently installed on the host.",
    },
  },
  SetEmbeddingModelResponse: {
    docs: "Wire payload for `set-embedding-model`. Mirrors `EmbeddingSpec` plus dimensions.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { dimensions: "usize" },
    fieldDocs: {
      provider_id: "Provider registry key (`ollama`, `stub`).",
      model_id: "Human-readable model identifier.",
      model_version: "Opaque version string reported by the provider.",
      dimensions: "Embedding dimensionality.",
    },
  },
  McpSessionConfig: {
    docs: "Wire payload for the MCP `session-config` tool. Distinct from the LSP `SessionConfig` (richer, MCP-specific keys).",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { root: "PathBuf", min_nodes: "u32", generation: "u64" },
    fieldDocs: {
      root: "Workspace root pinned at session creation.",
      min_nodes: "Subtree-size floor used throughout the session.",
      languages: "Languages with registered parsers (alphabetical).",
      incremental: "Whether the incremental fingerprint cache is enabled.",
      embedding_provenance: "Currently-active embedding provenance, if any.",
      cache_stats: "Cache-hit totals since session start.",
      generation: "Current generation counter.",
    },
  },
  Report: {
    docs: "A complete analysis report.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      report_schema_version: "u32",
      min_nodes: "u32",
      files_analysed: "usize",
      clusters_hidden: "usize",
    },
    fieldSerdeAttrs: {
      cache_stats: ["default"],
      metrics: ["default"],
      boilerplate_hints: ["default"],
    },
    tsOptional: ["boilerplate_hints"],
    fieldDocs: {
      report_schema_version: "Stable schema version so agent consumers can parse defensively.",
      tool_version: "Binary / library version that produced the report.",
      min_nodes: "Minimum subtree node count used for clustering.",
      files_analysed: "Number of files analysed.",
      clusters_hidden: "Clusters hidden because every member matched a `report_hide` pattern.",
      cache_stats: "Incremental-cache hit / miss counters for this run.",
      metrics: "Repo-wide duplication totals.",
      schema_doc: "Markdown schema explanation.",
      action_hints: "Short agent-oriented playbook.",
      boilerplate_hints: "Optional import/prologue hygiene hints.",
      embedding_provenance: "Provider/model/version that produced the embedding signals, if any.",
      clusters: "Ordered clusters, worst offenders first.",
    },
  },
};

// Maps an external type name (referenced from the .td but not defined
// in it) to the `use` path the post-processor must inject. Imports are
// emitted only when the generated code actually references the type so
// `unused_imports` warnings stay quiet. Types with a TYPE_CONFIG entry
// are skipped automatically (they are defined here, not imported).
const EXTERNAL_TYPES = {
  ReportCluster: "crate::report::ReportCluster",
  EmbeddingProvenance: "crate::report::EmbeddingProvenance",
  CacheStats: "crate::report::CacheStats",
};

const HEADER_PRELUDE = `//! Generated wire-format models for the Deslop live IPC surface.
//!
//! Source: \`docs/models/live-ipc.td\` (typeDiagram).
//! Generator: \`scripts/typediagram-gen.mjs\`.
//!
//! DO NOT EDIT BY HAND. Re-run \`make typediagram-gen\` (or any cargo
//! build) to regenerate. This file is gitignored.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
`;

function runTypediagram(target) {
  const stdout = execFileSync("typediagram", ["--to", target, TD_PATH], {
    encoding: "utf8",
  });
  return stdout;
}

// Splits the bare typediagram output into top-level items. typediagram
// emits one blank line between items; we anchor on the leading `pub `.
function splitItems(rust) {
  const lines = rust.split("\n");
  const items = [];
  let current = [];
  for (const line of lines) {
    if (line.startsWith("pub struct ") || line.startsWith("pub enum ") ||
        line.startsWith("pub type ")) {
      if (current.length > 0) {
        items.push(current.join("\n"));
      }
      current = [line];
    } else if (current.length > 0) {
      current.push(line);
    }
  }
  if (current.length > 0) {
    items.push(current.join("\n"));
  }
  return items.map((item) => item.replace(/\s+$/u, ""));
}

function typeNameOf(item) {
  const match = item.match(/^pub (?:struct|enum|type) (\w+)/u);
  return match ? match[1] : null;
}

// Rewrites field types both in `pub field: T,` struct lines and inline
// enum variant fields like `Variant { field: T, field: T },`. Inline
// variants are split onto their own lines so per-field doc comments
// satisfy the workspace `missing_docs` lint. Also injects per-field
// `#[serde(...)]` attrs (e.g. `default`, `skip_serializing_if`) when the
// TYPE_CONFIG entry's `fieldSerdeAttrs` declares them.
function applyFieldOverrides(item, overrides, fieldDocs, fieldSerdeAttrs) {
  if (!overrides && !fieldDocs && !fieldSerdeAttrs) return item;
  const lines = item.split("\n");
  const out = [];
  for (const line of lines) {
    const structMatch = line.match(/^(\s*)pub (\w+):\s*(.+?),?\s*$/u);
    if (structMatch) {
      const [, indent, fieldName, originalType] = structMatch;
      const newType = overrideType(overrides, fieldName, originalType);
      if (fieldDocs && fieldDocs[fieldName]) {
        out.push(`${indent}/// ${fieldDocs[fieldName]}`);
      }
      const serdeAttrs = fieldSerdeAttrs && fieldSerdeAttrs[fieldName];
      if (serdeAttrs && serdeAttrs.length > 0) {
        out.push(`${indent}#[serde(${serdeAttrs.join(", ")})]`);
      }
      out.push(`${indent}pub ${fieldName}: ${newType},`);
      continue;
    }
    const inlineVariant = line.match(
      /^(\s*)(\w+)\s*\{\s*(.+?)\s*\}\s*,?\s*$/u,
    );
    if (inlineVariant) {
      const [, indent, variantName, fieldsBlob] = inlineVariant;
      const childIndent = `${indent}    `;
      const fieldEntries = splitVariantFields(fieldsBlob);
      out.push(`${indent}${variantName} {`);
      for (const entry of fieldEntries) {
        const [fieldName, originalType] = entry;
        const newType = overrideType(overrides, fieldName, originalType);
        if (fieldDocs && fieldDocs[fieldName]) {
          out.push(`${childIndent}/// ${fieldDocs[fieldName]}`);
        }
        out.push(`${childIndent}${fieldName}: ${newType},`);
      }
      out.push(`${indent}},`);
      continue;
    }
    out.push(line);
  }
  return out.join("\n");
}

function overrideType(overrides, fieldName, originalType) {
  return overrides && overrides[fieldName]
    ? overrides[fieldName]
    : originalType;
}

// Splits `path: String, start_byte: i64, end_byte: i64` into entries
// while respecting angle-bracket nesting (`Option<List<T>>` etc.).
function splitVariantFields(blob) {
  const entries = [];
  let depth = 0;
  let current = "";
  for (const char of blob) {
    if (char === "<") depth += 1;
    else if (char === ">") depth -= 1;
    if (char === "," && depth === 0) {
      entries.push(current.trim());
      current = "";
      continue;
    }
    current += char;
  }
  if (current.trim().length > 0) entries.push(current.trim());
  return entries.map((entry) => {
    const colon = entry.indexOf(":");
    if (colon < 0) {
      throw new Error(`typediagram-gen: malformed variant field \`${entry}\``);
    }
    return [entry.slice(0, colon).trim(), entry.slice(colon + 1).trim()];
  });
}

function applyVariantDocs(item, variantDocs) {
  if (!variantDocs) return item;
  const lines = item.split("\n");
  const out = [];
  for (const line of lines) {
    const variantMatch = line.match(/^(\s*)(\w+)\s*(\{|,|$)/u);
    if (
      variantMatch &&
      variantMatch[1].length > 0 &&
      variantDocs[variantMatch[2]]
    ) {
      out.push(`${variantMatch[1]}/// ${variantDocs[variantMatch[2]]}`);
    }
    out.push(line);
  }
  return out.join("\n");
}

function decorateItem(item, config) {
  const before = [];
  before.push(`/// ${config.docs}`);
  if (config.derives && config.derives.length > 0) {
    before.push(`#[derive(${config.derives.join(", ")})]`);
  }
  if (config.serdeAttrs && config.serdeAttrs.length > 0) {
    before.push(`#[serde(${config.serdeAttrs.join(", ")})]`);
  }
  return [...before, item].join("\n");
}

function postprocess(rust) {
  const items = splitItems(rust);
  const decorated = [];
  const seen = new Set();
  for (const rawItem of items) {
    const name = typeNameOf(rawItem);
    if (!name) continue;
    const config = TYPE_CONFIG[name];
    if (!config) {
      throw new Error(
        `typediagram-gen: missing TYPE_CONFIG entry for \`${name}\`. ` +
          "Add an entry in scripts/typediagram-gen.mjs or remove the type from docs/models/live-ipc.td.",
      );
    }
    seen.add(name);
    let item = rawItem;
    item = applyFieldOverrides(
      item,
      config.fieldOverrides,
      config.fieldDocs,
      config.fieldSerdeAttrs,
    );
    item = applyVariantDocs(item, config.variantDocs);
    item = decorateItem(item, config);
    decorated.push(item);
  }
  for (const expected of Object.keys(TYPE_CONFIG)) {
    if (!seen.has(expected)) {
      throw new Error(
        `typediagram-gen: TYPE_CONFIG declares \`${expected}\` but ` +
          "the .td source did not produce it. Update either side.",
      );
    }
  }
  const body = decorated.join("\n\n");
  const externalImports = collectExternalImports(body);
  const header = externalImports.length > 0
    ? `${HEADER_PRELUDE}\n${externalImports.join("\n")}\n`
    : HEADER_PRELUDE;
  return `${header}\n${body}\n`;
}

// Scans the post-processed body for whole-word references to each
// external type and returns the matching `use` lines (sorted, no dups).
// Skips types that are themselves defined in this same generated file
// (TYPE_CONFIG keys) so an in-spec type never collides with a stale
// `crate::report::*` import. Keeps the import block free of
// `unused_imports` warnings without forcing the caller to declare them.
function collectExternalImports(body) {
  const imports = new Set();
  for (const [type, path] of Object.entries(EXTERNAL_TYPES)) {
    if (TYPE_CONFIG[type]) continue;
    const word = new RegExp(`\\b${type}\\b`, "u");
    if (word.test(body)) {
      imports.add(`use ${path};`);
    }
  }
  return [...imports].sort();
}

// Snake-cases an UpperCamelCase identifier (`OpenRange` -> `open_range`).
// Mirrors serde's `rename_all = "snake_case"` rule for variant names.
function toSnakeCase(name) {
  return name
    .replace(/([A-Z])([A-Z][a-z])/gu, "$1_$2")
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase();
}

// Returns the configured serde tag name (e.g. `"state"` for AnalysisState)
// when the type's serdeAttrs declares one, otherwise null.
function tagNameOf(config) {
  if (!config?.serdeAttrs) return null;
  for (const attr of config.serdeAttrs) {
    const match = attr.match(/^tag\s*=\s*"(\w+)"$/u);
    if (match) return match[1];
  }
  return null;
}

function hasSnakeCaseRename(config) {
  if (!config?.serdeAttrs) return false;
  return config.serdeAttrs.some((attr) =>
    /^rename_all\s*=\s*"snake_case"$/u.test(attr),
  );
}

// Returns a fn that applies the configured serde `rename_all` strategy
// to a variant identifier (`OpenRange` -> `open_range` for snake_case,
// `cli` for lowercase). Returns null when no rename is configured.
function variantCaseFn(config) {
  if (!config?.serdeAttrs) return null;
  for (const attr of config.serdeAttrs) {
    const match = attr.match(/^rename_all\s*=\s*"(\w+)"$/u);
    if (!match) continue;
    switch (match[1]) {
      case "snake_case": return toSnakeCase;
      case "lowercase": return (name) => name.toLowerCase();
      case "UPPERCASE": return (name) => name.toUpperCase();
      default: return null;
    }
  }
  return null;
}

// Post-processes typediagram's TypeScript output: fixes the broken
// `undefined<T>` syntax, rewrites discriminator field names + variant
// values to match what serde emits on the Rust side, collapses
// unit-only enums into wire-accurate string literal unions, and marks
// fields with serde `default` as optional `?:` so older payloads (and
// hand-written TS fixtures) that omit them keep type-checking.
function postprocessTs(ts) {
  let out = ts;
  out = out.replace(/undefined<([^>]+)>/gu, "$1 | null");
  out = rewriteUnions(out);
  out = markOptionalFields(out);
  return out;
}

// Walks each `export interface X { ... }` block and rewrites
// `field: Type;` to `field?: Type;` when the matching TYPE_CONFIG entry
// lists the field in `tsOptional`. Mirrors the historical VSIX shape
// for fields that older payloads or hand-written fixtures may omit.
function markOptionalFields(ts) {
  const lines = ts.split("\n");
  const out = [];
  let blockName = null;
  for (const line of lines) {
    const start = line.match(/^export interface (\w+)\s*\{/u);
    if (start) {
      blockName = start[1];
      out.push(line);
      continue;
    }
    if (blockName && line.trim() === "}") {
      blockName = null;
      out.push(line);
      continue;
    }
    if (blockName) {
      const config = TYPE_CONFIG[blockName];
      const optional = config?.tsOptional ?? [];
      const fieldMatch = optional.length > 0 &&
        line.match(/^(\s*)(\w+):\s*(.+);\s*$/u);
      if (fieldMatch && optional.includes(fieldMatch[2])) {
        const [, indent, fieldName, fieldType] = fieldMatch;
        out.push(`${indent}${fieldName}?: ${fieldType};`);
        continue;
      }
    }
    out.push(line);
  }
  return out.join("\n");
}

// Rewrites each `export type X = ...` discriminated-union block based on
// the matching TYPE_CONFIG entry. Walks the source line-by-line so the
// rewrite respects multi-line union declarations.
function rewriteUnions(ts) {
  const lines = ts.split("\n");
  const out = [];
  let blockName = null;
  let blockLines = [];
  for (const line of lines) {
    const startMatch = line.match(/^export type (\w+)\s*=\s*$/u);
    if (startMatch) {
      blockName = startMatch[1];
      blockLines = [line];
      continue;
    }
    if (blockName) {
      blockLines.push(line);
      if (line.trim().endsWith(";")) {
        out.push(rewriteUnionBlock(blockName, blockLines));
        blockName = null;
        blockLines = [];
      }
      continue;
    }
    out.push(line);
  }
  if (blockName) out.push(blockLines.join("\n"));
  return out.join("\n");
}

function rewriteUnionBlock(name, blockLines) {
  const config = TYPE_CONFIG[name];
  if (!config) return blockLines.join("\n");
  const tag = tagNameOf(config) ?? "kind";
  const renameFn = variantCaseFn(config);
  const variants = blockLines
    .slice(1)
    .map((line) => line.match(/\|\s*\{\s*kind:\s*"(\w+)"([^}]*)\}/u))
    .filter(Boolean);
  if (variants.length === 0) return blockLines.join("\n");
  const isUnitOnly = variants.every((m) => m[2].trim() === "");
  // Unit-only enums without a discriminator tag serialise on the wire
  // as a bare string (Rust's serde behaviour for unit variants), so the
  // matching TS shape is a literal string union.
  if (isUnitOnly && tag === "kind" && renameFn) {
    const literals = variants.map((m) => `"${renameFn(m[1])}"`).join(" | ");
    return `export type ${name} = ${literals};`;
  }
  const rewritten = [`export type ${name} =`];
  for (const match of variants) {
    const [, variant, payload] = match;
    const tagValue = renameFn ? renameFn(variant) : variant;
    rewritten.push(`  | { ${tag}: "${tagValue}"${payload}}`);
  }
  return `${rewritten.join("\n")};`;
}

const TS_HEADER = `// @generated by scripts/typediagram-gen.mjs from docs/models/live-ipc.td
// DO NOT EDIT BY HAND. Re-run \`make typediagram-gen\` to regenerate.
// Per CLAUDE.md the generated wire types are gitignored; the .td file is
// the single source of truth for shapes shared with the Rust transports.
`;

function tsImports(body) {
  const imports = [];
  const grouped = new Map();
  for (const [type, mod] of Object.entries(EXTERNAL_TS_TYPES)) {
    if (TYPE_CONFIG[type]) continue;
    const word = new RegExp(`\\b${type}\\b`, "u");
    if (word.test(body)) {
      if (!grouped.has(mod)) grouped.set(mod, new Set());
      grouped.get(mod).add(type);
    }
  }
  for (const [mod, types] of [...grouped].sort(([a], [b]) => a.localeCompare(b))) {
    imports.push(`import type { ${[...types].sort().join(", ")} } from "${mod}";`);
  }
  return imports;
}

function generateTs() {
  const raw = runTypediagram("typescript");
  const body = postprocessTs(raw);
  const imports = tsImports(body);
  const importBlock = imports.length > 0 ? `${imports.join("\n")}\n\n` : "";
  return `${TS_HEADER}\n${importBlock}${body}`;
}

function main() {
  const rust = postprocess(runTypediagram("rust"));
  mkdirSync(dirname(OUT_RUST), { recursive: true });
  writeFileSync(OUT_RUST, rust, "utf8");
  process.stdout.write(`typediagram-gen: wrote ${OUT_RUST}\n`);

  const ts = generateTs();
  mkdirSync(dirname(OUT_TS), { recursive: true });
  writeFileSync(OUT_TS, ts, "utf8");
  process.stdout.write(`typediagram-gen: wrote ${OUT_TS}\n`);
}

main();
