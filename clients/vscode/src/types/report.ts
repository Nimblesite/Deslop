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

/** The cluster's severity band as the engine stamped it
 * ([SEVERITY-BAND]). The band classifies the cluster's rank percentile,
 * which is a calculation, so it is computed once in
 * `report_weight::rank_band` and carried on the wire. A report written
 * before the field existed carries an empty string and reads as the
 * tail band. */
export function clusterBand(cluster: ReportCluster): Severity {
  return SEVERITIES.find((band) => band === cluster.rank_band) ?? "faint";
}

// [SEVERITY-DESLOP-MAP] The Deslop severity level — the *other* visual
// channel, and the one that answers "how alarming is this kind of
// duplicate?". It is a function of the bucket alone, never of the ranking:
// per [SEVERITY-COLOR] colour carries the bucket and glyph density carries
// the weight percentile, and the two are orthogonal by design. A faint
// identical clone is a red `○`; a high-impact shape-only family is a grey
// `●●`. Collapsing them into one channel is what let a demoted family wear
// the loudest paint in the editor.
export type DeslopSeverity = "error" | "warning" | "information" | "hint";

/** Every Deslop severity level, loudest first. */
export const DESLOP_SEVERITIES: readonly DeslopSeverity[] = [
  "error",
  "warning",
  "information",
  "hint",
] as const;

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

export const IDENTICAL_BUCKET_VALUE: Bucket = "identical";

export const BUCKETS: readonly Bucket[] = [
  IDENTICAL_BUCKET_VALUE,
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
    cssSuffix: IDENTICAL_BUCKET_VALUE,
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

// [CLONE-BUCKETS-ROUTING] The engine owns the routing and is the only
// place it can be decided. `deslop-core::report_render::report_bucket_kind`
// weighs the *raw* signal triple, measured `ContentEvidence`, raw-source
// byte-equivalence, and the member spread — and the triple that reaches
// this client is the elected pair's own measurement, projected by the
// engine: `content_gated_signals` overwrites `token_jaccard` to 1.0 for a
// shape-identical near miss (#232). Re-running the engine's raw-signal
// table over rendered signals is therefore a category error, and every
// arm that tried it shipped a defect: a proven rename read back as
// byte-identical ("Safe to extract — every copy is the same" about code
// whose identifiers all differ), a content-gated family promoted to
// act-now, and two low-structural arms the engine never had. The UI reads
// the engine's label and never manufactures one.
export function resolveBucket(cluster: ReportCluster): Bucket {
  if (
    cluster.bucket &&
    (BUCKETS as readonly string[]).includes(cluster.bucket)
  ) {
    return cluster.bucket as Bucket;
  }
  // A report carrying no engine label carries no verdict. `loosely_similar`
  // is the only honest destination: it is the sole bucket whose action
  // sentence claims nothing beyond "treat as a hint", so an unlabelled
  // cluster can never be repainted as something to act on.
  return "loosely_similar";
}

// Buckets the engine considers actionable. A surface that withholds one of
// these is a false negative; a surface that paints anything else with them
// is a false positive. Exported so the live bubble, the tree, and the tests
// share one definition ([VSIX-LIVE-BUBBLE]).
export const ACT_NOW_BUCKETS: readonly Bucket[] = [
  IDENTICAL_BUCKET_VALUE,
  "nearly_identical",
] as const;

export function isActNow(bucket: Bucket): boolean {
  return ACT_NOW_BUCKETS.includes(bucket);
}

// ---------------------------------------------------------------------------
// Canonical clone categories — mirrors deslop-core::clone_category.
// Orthogonal to Bucket per [FACET-MODEL]: bucket = "how similar",
// category = "what kind of repetition". The shipped registry is
// logic + data; the literal families join when [LITERAL-CATEGORY] ships.
// ---------------------------------------------------------------------------

// Wire label carried in JSON `cluster.category`.
export type Category = "logic" | "data";

export const DATA_CATEGORY_VALUE: Category = "data";

export const CATEGORIES: readonly Category[] = ["logic", DATA_CATEGORY_VALUE] as const;

export interface CategoryLabels {
  // Plain title for facet surfaces (filter QuickPick, webview category
  // options, HTML facet chips): the shared chip for chip-carrying
  // categories, "Code clones" for the chip-less logic default.
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
  return cluster.category === DATA_CATEGORY_VALUE ? DATA_CATEGORY_VALUE : "logic";
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
