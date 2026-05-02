// Mirrors deslop-core::report at REPORT_SCHEMA_VERSION = 1.
// Keep in sync with crates/deslop-core/src/report.rs,
// crates/deslop-core/src/buckets.rs, and
// crates/deslop-core/src/report_metrics.rs.

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
  boilerplate_hints?: BoilerplateHint[];
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
  attempted_subtrees?: number;
  indexed_subtrees?: number;
  failed_subtrees?: number;
}

export interface ReportCluster {
  id: string;
  weight: number;
  size: number;
  canonical_node_count: number;
  signals: ReportSignals;
  // Canonical bucket wire label (schema v4+). One of Bucket values.
  // Optional so we can still read v3 reports; use classifyCluster() as
  // a fallback when missing.
  bucket?: Bucket;
  // On the live wire this is capped at LIVE_WIRE_OCCURRENCE_CAP; use
  // `occurrences_total` / `occurrences_truncated` to tell if the caller
  // needs to page via `deslop/clusterById`.
  occurrences: ReportOccurrence[];
  // Pre-cap occurrence count. Defaults to `size` on reports loaded
  // from a `--from-report` CLI dump that pre-dates the field.
  occurrences_total?: number;
  // True when `occurrences` was truncated for the wire. Always false
  // on `deslop/clusterById` responses.
  occurrences_truncated?: boolean;
  // Blanked on the live wire; re-derive from `bucket` via bucketLabels.
  summary: string;
  // Blanked on the live wire; re-derive from `bucket` via bucketLabels.
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
  // VSIX-only display projection. Not part of the canonical report
  // schema; the extension host derives it from start_byte before
  // posting a report into webviews.
  displayLocation?: OccurrenceDisplayLocation;
}

export interface OccurrenceDisplayLocation {
  line: number;
  column: number;
  label: string;
  description: string;
  commandTitle: string;
}

export function occurrenceCount(cluster: ReportCluster): number {
  const total =
    cluster.occurrences_total && cluster.occurrences_total > 0
      ? cluster.occurrences_total
      : cluster.size;
  return Math.max(total, cluster.occurrences.length);
}

/** Count to display in compact surfaces (hover, decoration). Uses the
 * authoritative total when present; falls back to the visible slice length
 * so we never show a count higher than the occurrences the caller can act on. */
export function visibleOccurrenceCount(cluster: ReportCluster): number {
  return cluster.occurrences_total && cluster.occurrences_total > 0
    ? cluster.occurrences_total
    : cluster.occurrences.length;
}

export interface ActionHint {
  pattern: string;
  recommendation: string;
}

export interface BoilerplateHint {
  kind: string;
  language: string;
  severity: "info";
  recommendation: string;
  occurrences: BoilerplateHintOccurrence[];
}

export interface BoilerplateHintOccurrence {
  path: string;
  start_byte: number;
  end_byte: number;
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

// ReportDelta (deslop_core::delta) — live updates.
export interface ReportDelta {
  from_generation: number;
  to_generation: number;
  clusters_added: ReportCluster[];
  clusters_removed: string[];
  clusters_updated: ReportCluster[];
  cache_stats: CacheStats;
  tool_version: string;
}

// Wire-format models generated from `docs/models/live-ipc.td` by
// `scripts/typediagram-gen.mjs`. Re-exported here so the historical
// `clients/vscode/src/types/report` import path keeps resolving for
// every consumer. The generated source is gitignored; `make
// typediagram-gen` (chained into `make vsix-build`) regenerates it.
export type {
  AnalysisState,
  ChangeSummary,
  EmbeddingModelInfo,
  EmbeddingPhase,
  EmbeddingProgress,
  ReportChangedNotification,
  SessionConfig,
} from "./wire-generated";

// Severity bucketing per [LSP-SEVERITY]. Orthogonal to Bucket:
// severity = "how bad is this cluster in the ranking?", bucket =
// "what kind of clone is it?".
export type Severity = "worst" | "top10" | "mid" | "faint";

export function severityOf(weightPercentile: number): Severity {
  if (weightPercentile >= 0.99) return "worst";
  if (weightPercentile >= 0.9) return "top10";
  if (weightPercentile >= 0.5) return "mid";
  return "faint";
}

// ---------------------------------------------------------------------------
// Canonical clone buckets — mirrors deslop-core::buckets.
// Single source of truth for every user-facing surface in the VS Code
// extension per docs/specs/taxonomy.md [CLONE-BUCKETS-DUAL-LABEL].
// ---------------------------------------------------------------------------

// Wire label used in JSON `cluster.bucket` (schema v4). Stable contract;
// never rename without bumping the schema version.
export type Bucket =
  | "identical"
  | "nearly_identical"
  | "loosely_similar"
  | "same_behavior";

export const BUCKETS: readonly Bucket[] = [
  "identical",
  "nearly_identical",
  "loosely_similar",
  "same_behavior",
] as const;

export interface BucketLabels {
  // Pure-visual surfaces (bubble, tree view, webview card titles) — no Type-N.
  plainTitle: string;
  // Shared-text surfaces (Problems panel, hover, diagnostic message) —
  // plain prose + bracketed Type-N suffix for AI scrapers.
  hybridTitle: string;
  // Plain-English one-liner shown under the title on every surface.
  actionSentence: string;
  // Academic taxonomy reference composed into AI-only sentences.
  taxonomyLabel: string;
  // CSS class suffix for HTML / webview cards.
  cssSuffix: string;
  // True only for SameBehavior (Type-4, embedding-pass output).
  aiMatch: boolean;
}

const LABELS: Record<Bucket, BucketLabels> = {
  identical: {
    plainTitle: "Identical code",
    hybridTitle: "Identical code [Type-1/2]",
    actionSentence: "Safe to extract — every copy is the same.",
    taxonomyLabel: "Type-1 or Type-2 exact clone",
    cssSuffix: "identical",
    aiMatch: false,
  },
  nearly_identical: {
    plainTitle: "Nearly identical code",
    hybridTitle: "Nearly identical code [Type-3]",
    actionSentence: "Review the locations — small differences may matter.",
    taxonomyLabel: "Type-3 near-miss",
    cssSuffix: "nearly-identical",
    aiMatch: false,
  },
  loosely_similar: {
    plainTitle: "Loosely similar code",
    hybridTitle: "Loosely similar code [weak LSH]",
    actionSentence: "Loose textual overlap. Treat as a hint.",
    taxonomyLabel: "weak LSH-only signal (sub-Type-3)",
    cssSuffix: "loosely-similar",
    aiMatch: false,
  },
  same_behavior: {
    plainTitle: "Same behavior, different code",
    hybridTitle: "Same behavior, different code [Type-4, AI match]",
    actionSentence:
      "The AI noticed these do the same thing written two ways — read both before merging.",
    taxonomyLabel: "Type-4 semantic clone (AI match)",
    cssSuffix: "same-behavior",
    aiMatch: true,
  },
};

export function bucketLabels(bucket: Bucket): BucketLabels {
  return LABELS[bucket];
}

// Routing from signal triple onto a canonical bucket. Must match
// deslop-core::buckets::classify_signals byte-for-byte; the
// Deslop core owns the routing table in [CLONE-BUCKETS-ROUTING].
export function classifyCluster(signals: ReportSignals): Bucket {
  if (signals.structural >= 0.99 && signals.token_jaccard >= 0.99) {
    return "identical";
  }
  if (signals.embedding_cos >= 0.8 && signals.structural < 0.5) {
    return "same_behavior";
  }
  if (
    signals.structural >= 0.99 ||
    (signals.structural > 0.0 && signals.token_jaccard >= 0.95) ||
    (signals.structural <= 0.01 && signals.token_jaccard >= 0.9)
  ) {
    return "nearly_identical";
  }
  return "loosely_similar";
}

// Resolves a cluster's bucket, preferring the JSON-carried wire label
// (schema v4) and falling back to re-routing from signals for older
// v3 reports loaded via --from-report.
export function resolveBucket(cluster: ReportCluster): Bucket {
  if (cluster.bucket && (BUCKETS as readonly string[]).includes(cluster.bucket)) {
    return cluster.bucket;
  }
  return classifyCluster(cluster.signals);
}

// Returns the cluster's interpretation line, falling back to the
// bucket's action sentence when the live wire has blanked the field.
// Every UI surface (hover, decorations, panels) funnels through this
// so the "what does this cluster mean" prose stays consistent whether
// the cluster came from a live LSP response or a CLI-loaded report.
export function clusterInterpretation(cluster: ReportCluster): string {
  return cluster.interpretation && cluster.interpretation.length > 0
    ? cluster.interpretation
    : bucketLabels(resolveBucket(cluster)).actionSentence;
}

// ---------------------------------------------------------------------------
// Legacy Verdict alias for [VSIX-LIVE-BUBBLE] call sites — prefer
// `resolveBucket` + `bucketLabels` on new code. Kept for transition.
// ---------------------------------------------------------------------------

export type Verdict =
  | "DUPLICATE"
  | "NEAR-MISS"
  | "SEMANTIC MATCH"
  | "LOOSELY SIMILAR";

export function verdictOf(signals: ReportSignals): Verdict {
  switch (classifyCluster(signals)) {
    case "identical":
      return "DUPLICATE";
    case "nearly_identical":
      return "NEAR-MISS";
    case "same_behavior":
      return "SEMANTIC MATCH";
    case "loosely_similar":
      return "LOOSELY SIMILAR";
  }
}

export const FUSED_THRESHOLD = 0.85;
