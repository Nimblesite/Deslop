// Severity per [SEVERITY-MODEL] / [SEVERITY-COLOR]. Colors live in design.ts.
//
// Severity is a single channel: the engine-stamped mass rank band. The
// retired per-bucket severity maps are invalid configuration
// ([SEVERITY-CONFIG]); a cluster's colour cannot imply that it is
// identical, near-identical, structural-only, semantic, or content-proven
// ([SEVERITY-COLOR]).

import { ReportCluster, Severity, clusterBand } from "./types/report";

/** The four Deslop severity levels ([SEVERITY-DESLOP-MAP]). A cluster's
 * level is a pure function of its engine-stamped mass rank band — never
 * a pair signal or clone-kind classification. */
export type DeslopSeverity = "error" | "warning" | "information" | "hint";

/** Every Deslop severity level, in rank order. */
export const DESLOP_SEVERITIES: readonly DeslopSeverity[] = [
  "error",
  "warning",
  "information",
  "hint",
] as const;

export { SEVERITY_COLOR, DESLOP_SEVERITY_COLOR, SEVERITY_DOT } from "./design";

// [SEVERITY-DESLOP-MAP] Mass rank band → level. Shared by the CLI, HTML,
// LSP, VSIX, and agent surfaces; consumers read `rank_band` and do not
// recompute percentiles.
const RANK_BAND_SEVERITY: Record<Severity, DeslopSeverity> = {
  worst: "error",
  top10: "warning",
  mid: "information",
  faint: "hint",
};

/** [SEVERITY-DESLOP-MAP] The Deslop severity level of a mass rank band. */
export function deslopSeverityOf(severity: Severity): DeslopSeverity {
  return RANK_BAND_SEVERITY[severity];
}

/** The colour channel of a cluster, resolved from the engine's own band. */
export function clusterSeverity(cluster: ReportCluster): DeslopSeverity {
  return deslopSeverityOf(clusterBand(cluster));
}

/** Both visual channels of one cluster. */
export interface ResolvedSeverity {
  /** Drives colour — the mass-band-derived Deslop severity. */
  level: DeslopSeverity;
  /** Drives glyph density — the engine's own `rank_band`
   * ([SEVERITY-BAND]). */
  band: Severity;
}

/**
 * [SEVERITY-COLOR] The single resolver every visual surface consumes.
 * Both channels are read from the engine-stamped band, never derived
 * from rank position or pair measurements.
 */
export function resolveSeverity(cluster: ReportCluster): ResolvedSeverity {
  return { level: clusterSeverity(cluster), band: clusterBand(cluster) };
}
