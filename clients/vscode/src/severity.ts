// Severity per [SEVERITY-MODEL] / [SEVERITY-COLOR]. Colors live in design.ts.
//
// Severity is two orthogonal channels, and the whole point of the model is
// that neither one may answer for the other:
//
//   colour        = Deslop severity, a function of the BUCKET
//   glyph density = severity band,   a function of the WEIGHT PERCENTILE
//
// "A faint identical clone renders as a red ○, while a high-impact
// loosely-similar cluster renders as a blue ●●" ([SEVERITY-COLOR]). Asking
// the percentile band to also carry the bucket is unsatisfiable — the band
// is monotonic down the ranking by construction, so any answer it gives for
// one demoted cluster it must give for everything below it too.

import {
  Bucket,
  DeslopSeverity,
  ReportCluster,
  Severity,
  resolveBucket,
  severityOf,
} from "./types/report";

export { SEVERITY_COLOR, DESLOP_SEVERITY_COLOR, SEVERITY_DOT } from "./design";

// [SEVERITY-DESLOP-MAP] Bucket → level. `structural_only` sits at `hint`
// with the muted/outline colour band taxonomy.md gives it: the engine's own
// action sentence is "Verify before extracting", which is the opposite of an
// act-now claim, and a cluster the content gate demoted must never wear the
// same paint as a byte-proven clone however heavily it ranks.
const BUCKET_SEVERITY: Record<Bucket, DeslopSeverity> = {
  identical: "error",
  nearly_identical: "warning",
  loosely_similar: "information",
  structural_only: "hint",
  same_behavior: "hint",
};

/** [SEVERITY-DESLOP-MAP] The always-on Deslop severity level of a bucket. */
export function deslopSeverityOf(bucket: Bucket): DeslopSeverity {
  return BUCKET_SEVERITY[bucket];
}

/** The colour channel of a cluster, resolved from the engine's own label. */
export function clusterSeverity(cluster: ReportCluster): DeslopSeverity {
  return deslopSeverityOf(resolveBucket(cluster));
}

/** Both visual channels of one cluster. */
export interface ResolvedSeverity {
  /** Drives colour — bucket-derived, never rank-derived. */
  level: DeslopSeverity;
  /** Drives glyph density — percentile-derived, never bucket-derived. */
  band: Severity;
}

/**
 * [SEVERITY-COLOR] The single resolver every visual surface consumes. The two
 * channels are returned together precisely so a caller cannot reach for one
 * and silently render the other fact.
 */
export function resolveSeverity(bucket: Bucket, percentile: number): ResolvedSeverity {
  return { level: deslopSeverityOf(bucket), band: severityOf(percentile) };
}

export function rankPercentile(rank: number, total: number): number {
  if (total <= 1) return 0;
  return 1 - (rank - 1) / (total - 1);
}

export function severityForRank(rank: number, total: number): Severity {
  return severityOf(rankPercentile(rank, total));
}

export function indexedSeverity(clusters: ReportCluster[]): Map<string, Severity> {
  const total = clusters.length;
  const out = new Map<string, Severity>();
  clusters.forEach((cluster, i) => out.set(cluster.id, severityForRank(i + 1, total)));
  return out;
}
