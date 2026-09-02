// One `ReportCluster` fixture builder for every VS Code suite.
//
// A cluster carries the figures the engine computed for it — the global
// rank, the severity band, the mass, and the occurrence count — and
// every surface reads them verbatim. A suite that hand-rolled its own
// literal would be free to omit one, and a surface reading an omitted
// field renders a zero instead of failing, which is exactly the silent
// wrong answer the accuracy contract forbids. One builder means one
// place where a new wire field has to be answered for.
//
// The defaults describe a single mid-band cluster; suites override
// whatever they are pinning. Pair signals are NOT fixture data: they
// belong to explicit pair records ([FACET-MODEL]) and never ride on a
// cluster.

import type {
  ReportCluster,
  ReportOccurrence,
  Severity,
} from "../types/report";

/** Everything a suite may pin on a fixture cluster. */
export interface ClusterFixture {
  id: string;
  occurrences: ReportOccurrence[];
  rank?: number;
  rank_band?: Severity;
  mass?: number;
  canonical_node_count?: number;
  occurrences_total?: number;
  occurrence_count?: number;
  occurrences_truncated?: boolean;
  intersects_diff?: boolean;
  is_newly_introduced?: boolean;
}

/** A complete wire cluster: every field present, engine-derived fields
 * consistent unless the suite pins them. */
export function wireCluster(fixture: ClusterFixture): ReportCluster {
  const occurrences = fixture.occurrences;
  // What `report::occurrence_count` would stamp for this fixture: the
  // tracked total, never below the carried list. Reproduced once, here,
  // so a suite never has to state a count its own occurrence list
  // contradicts.
  const count =
    fixture.occurrence_count ??
    Math.max(fixture.occurrences_total ?? 0, occurrences.length);
  return {
    id: fixture.id,
    rank: fixture.rank ?? 1,
    rank_band: fixture.rank_band ?? "mid",
    mass: fixture.mass ?? 1,
    canonical_node_count: fixture.canonical_node_count ?? 4,
    occurrences,
    occurrences_total: fixture.occurrences_total ?? count,
    occurrence_count: count,
    occurrences_truncated: fixture.occurrences_truncated ?? false,
    ...(fixture.intersects_diff === undefined
      ? {}
      : { intersects_diff: fixture.intersects_diff }),
    ...(fixture.is_newly_introduced === undefined
      ? {}
      : { is_newly_introduced: fixture.is_newly_introduced }),
  };
}

/** One occurrence, with the fields every suite spells out. Line numbers
 * are fixture facts — suites that pin locations pass their own. */
export function occurrence(
  path: string,
  startByte = 0,
  endByte = 20,
  hidden = false,
): ReportOccurrence {
  return {
    path,
    start_byte: startByte,
    end_byte: endByte,
    start_line: 1,
    end_line: 2,
    hidden,
  };
}

/**
 * Stamps the ranking the engine would have stamped on this list —
 * `deslop-core::report_weight::stamp_ranks` — so a fixture report's
 * clusters carry ranks and bands consistent with each other and with
 * their order.
 *
 * Fixture staging, not a client calculation: production code reads
 * `rank` and `rank_band` and never computes them
 * ([PRINCIPLES-ONE-CALCULATION]). The cut points themselves are pinned
 * in Rust by `report_weight::rank_band_cut_points`.
 */
export function stampRanks(clusters: ReportCluster[]): ReportCluster[] {
  const total = clusters.length;
  return clusters.map((cluster, index) => ({
    ...cluster,
    rank: index + 1,
    rank_band: bandOf(index + 1, total),
  }));
}

/** The engine's band for a rank in a report of `total` clusters —
 * `report_weight::rank_band_cut_points` (pinned in Rust). Exported so
 * suites can compute the bands a stamped fixture WILL carry; production
 * code never uses this ([PRINCIPLES-ONE-CALCULATION]). The cut points are
 * the engine's integer ceil boundaries — e.g. the sole cluster of a
 * one-cluster report is rank 1 of 1 and is "worst". */
export function bandOf(rank: number, total: number): Severity {
  if (rank <= Math.ceil(total / 100)) return "worst";
  if (rank <= Math.ceil(total / 10)) return "top10";
  if (rank <= Math.ceil(total / 2)) return "mid";
  return "faint";
}
