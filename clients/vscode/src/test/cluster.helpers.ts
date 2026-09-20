// [FACET-MODEL] Shared fixtures carry complete engine fields. Individual
// tests override the facts they exercise; pair evidence belongs to pairs.

import type {
  ClusterKind,
  ReportCluster,
  ReportOccurrence,
  Severity,
} from "../types/report";

/** The clone kind every fixture cluster carries unless a suite pins
 * another — the ordinary admitted near-copy, mirroring
 * `deslop_core::report_fixtures::FIXTURE_KIND`. */
export const FIXTURE_KIND: ClusterKind = "nearly_identical";
export const FIXTURE_ROUTING = {
  nearly_identical_min_shape: 0.9,
  nearly_identical_min_content: 0.7,
  similar_min_content: 0.5,
  shape_only_max_content: 0.05,
};

/** Everything a suite may pin on a fixture cluster. */
export interface ClusterFixture {
  id: string;
  occurrences: ReportOccurrence[];
  rank?: number;
  severity?: Severity;
  kind?: ClusterKind;
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
    severity: fixture.severity ?? "information",
    kind: fixture.kind ?? FIXTURE_KIND,
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


export function stampRanks(clusters: ReportCluster[]): ReportCluster[] {
  return clusters.map((cluster, index) => ({ ...cluster, rank: index + 1 }));
}

