// Mirrors deslop-core::report.
// Every wire shape has a typeDiagram .td entry and is re-exported from
// `./wire-generated`. UI-only augmentation (`displayLocation`) lives as
// an intersection so the wire shape stays the source of truth.

import type {
  Report as WireReport,
  ReportCluster as WireReportCluster,
  ReportOccurrence as WireReportOccurrence,
  ReportSignals,
} from "./wire-generated";

export type {
  CacheStats,
  EmbeddingProvenance,
  ReportSignals,
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

// Wire-format models generated from `docs/models/live-ipc.td` by
// `scripts/typediagram-gen.mjs`. Re-exported here so the historical
// `clients/vscode/src/types/report` import path keeps resolving for
// every consumer. The generated source is gitignored; `make
// typediagram-gen` (chained into `make vsix-build`) regenerates it.
export type {
  ActionHint,
  AnalysisState,
  ChangeSummary,
  EmbeddingModelInfo,
  EmbeddingPhase,
  EmbeddingProgress,
  FileMetric,
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

// Severity bucketing per [LSP-SEVERITY]. Orthogonal to Bucket:
// severity = "how bad is this cluster in the ranking?", bucket =
// "what kind of clone is it?".
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

// Wire label used in JSON `cluster.bucket`. Stable contract for the
// current report shape.
export type Bucket =
  | "identical"
  | "nearly_identical"
  | "structural_only"
  | "loosely_similar"
  | "same_behavior";

export const BUCKETS: readonly Bucket[] = [
  "identical",
  "nearly_identical",
  "structural_only",
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
  structural_only: {
    plainTitle: "Same shape, different content",
    hybridTitle: "Same shape, different content [structural-only]",
    actionSentence:
      "Only the code shape matches — usually sibling boilerplate. Verify before extracting.",
    taxonomyLabel: "structural-only match (unverified Type-2/3 candidate)",
    cssSuffix: "structural-only",
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
  // [RANK-STRUCTURAL-ONLY]: shape is the only positive evidence.
  // Ceilings mirror deslop-core::buckets::STRUCTURAL_ONLY_MAX_SUPPORT.
  if (
    signals.structural >= 0.99 &&
    signals.token_jaccard < 0.05 &&
    signals.embedding_cos < 0.05
  ) {
    return "structural_only";
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
  if (
    cluster.bucket &&
    (BUCKETS as readonly string[]).includes(cluster.bucket)
  ) {
    return cluster.bucket as Bucket;
  }
  return classifyCluster(cluster.signals);
}

// ---------------------------------------------------------------------------
// Canonical clone categories — mirrors deslop-core::clone_category.
// Orthogonal to Bucket per [FACET-MODEL]: bucket = "how similar",
// category = "what kind of repetition". The shipped registry is
// logic + data; the literal families join when [LITERAL-CATEGORY] ships.
// ---------------------------------------------------------------------------

// Wire label carried in JSON `cluster.category`.
export type Category = "logic" | "data";

export const CATEGORIES: readonly Category[] = ["logic", "data"] as const;

export interface CategoryLabels {
  // Plain group title for facet and grouping surfaces: the shared chip
  // for chip-carrying categories, "Code clones" for the chip-less
  // logic default ([FACET-GROUP-BY-TYPE]).
  groupTitle: string;
  // Short chip shown next to bucket titles; null for logic — the
  // absence of a chip already communicates "ordinary logic clone".
  chip: string | null;
}

const CATEGORY_LABELS: Record<Category, CategoryLabels> = {
  logic: { groupTitle: "Code clones", chip: null },
  data: { groupTitle: "data table", chip: "data table" },
};

export function categoryLabels(category: Category): CategoryLabels {
  return CATEGORY_LABELS[category];
}

// Resolves a cluster's category from the wire label, defaulting to
// "logic" for absent or unknown values — mirrors
// deslop-core::clone_category::from_wire_label.
export function resolveCategory(cluster: ReportCluster): Category {
  return cluster.category === "data" ? "data" : "logic";
}

/** A sanitized facet filter: only registry-known values survive. */
export interface FacetFilter {
  buckets: Bucket[];
  categories: Category[];
}

// [FACET-TOP-OFFENDERS-FILTER] Drops unknown values from the persisted
// filter arrays (the typo fallback — a bad value must never yield an
// empty tree). Both lists empty means the filter is inactive.
export function sanitizeFacetFilter(
  filterBuckets: readonly string[],
  filterCategories: readonly string[],
): FacetFilter {
  return {
    buckets: filterBuckets.filter((value): value is Bucket =>
      (BUCKETS as readonly string[]).includes(value),
    ),
    categories: filterCategories.filter((value): value is Category =>
      (CATEGORIES as readonly string[]).includes(value),
    ),
  };
}

// [FACET-TOP-OFFENDERS-FILTER] The one facet-filter slice shared by the
// Top Offenders tree, the report webview, and the status-bar count so
// the three surfaces always agree. An empty value list means "show all"
// for that axis; the two axes compose as an AND. Presentation-only:
// never mutates the report.
export function applyFacetFilter(
  clusters: ReportCluster[],
  filter: FacetFilter,
): ReportCluster[] {
  const { buckets, categories } = filter;
  if (buckets.length === 0 && categories.length === 0) return clusters;
  return clusters.filter(
    (cluster) =>
      (buckets.length === 0 || buckets.includes(resolveBucket(cluster))) &&
      (categories.length === 0 || categories.includes(resolveCategory(cluster))),
  );
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

export const FUSED_THRESHOLD = 0.85;
