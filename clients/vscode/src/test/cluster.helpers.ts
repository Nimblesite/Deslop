// One `ReportCluster` fixture builder for every VS Code suite.
//
// A cluster carries the figures the engine computed for it — the global
// rank, the severity band, the elected pair's measured axes, the
// occurrence count and the evidence sentence — and every surface reads
// them verbatim. A suite that hand-rolled its own literal would be free
// to omit one, and a surface reading an omitted field renders a zero
// instead of failing, which is exactly the silent wrong answer the
// accuracy contract forbids. One builder means one place where a new
// wire field has to be answered for.
//
// The defaults describe a single-cluster report of the given bucket;
// suites override whatever they are pinning.

import { bucketSignals } from "./signals.helpers";
import type {
  Bucket,
  ReportCluster,
  ReportOccurrence,
  ReportSignals,
  ReportSignalSource,
  Severity,
} from "../types/report";

/** Everything a suite may pin on a fixture cluster. */
export interface ClusterFixture {
  id: string;
  occurrences: ReportOccurrence[];
  rank?: number;
  rank_band?: Severity;
  weight?: number;
  size?: number;
  canonical_node_count?: number;
  bucket?: Bucket;
  category?: string;
  language?: string;
  signals?: ReportSignals;
  signal_source?: ReportSignalSource;
  evidence_verdict?: string;
  occurrences_total?: number;
  occurrence_count?: number;
  occurrences_truncated?: boolean;
  summary?: string;
  interpretation?: string;
  intersects_diff?: boolean;
  is_newly_introduced?: boolean;
}

/** A complete wire cluster: every field present, engine-derived fields
 * consistent with the bucket unless the suite pins them. */
export function wireCluster(fixture: ClusterFixture): ReportCluster {
  const bucket = fixture.bucket ?? "identical";
  const occurrences = fixture.occurrences;
  const size = fixture.size ?? occurrences.length;
  // What `report::occurrence_count` would stamp for this fixture: the
  // tracked total, never below the carried list. Reproduced once, here,
  // so a suite never has to state a count its own occurrence list
  // contradicts.
  const count =
    fixture.occurrence_count ??
    Math.max(fixture.occurrences_total ?? 0, size, occurrences.length);
  return {
    id: fixture.id,
    rank: fixture.rank ?? 1,
    rank_band: fixture.rank_band ?? "mid",
    weight: fixture.weight ?? 1,
    size,
    canonical_node_count: fixture.canonical_node_count ?? 4,
    signals: fixture.signals ?? bucketSignals(bucket),
    // The elected pair whose measurement `signals` carries
    // ([FUSED-CLUSTER-SIGNALS]). Default: the fixture's first two
    // occurrences, the pair a multi-member fixture elects; a
    // single-occurrence fixture has no admitted pair and carries no
    // source, matching the engine's no-pair convention. Every rendered
    // axis must trace to a real pair, so a fixture without one is a
    // fixture lying about where its numbers came from.
    signal_source:
      fixture.signal_source ?? (occurrences.length >= 2 ? { left: 0, right: 1 } : undefined),
    bucket,
    category: fixture.category ?? "logic",
    language: fixture.language ?? "csharp",
    evidence_verdict: fixture.evidence_verdict ?? "",
    occurrences,
    occurrences_total: fixture.occurrences_total ?? count,
    occurrence_count: count,
    occurrences_truncated: fixture.occurrences_truncated ?? false,
    summary: fixture.summary ?? "",
    interpretation: fixture.interpretation ?? "",
    ...(fixture.intersects_diff === undefined
      ? {}
      : { intersects_diff: fixture.intersects_diff }),
    ...(fixture.is_newly_introduced === undefined
      ? {}
      : { is_newly_introduced: fixture.is_newly_introduced }),
  };
}

/** One occurrence, with the fields every suite spells out. */
export function occurrence(
  path: string,
  startByte = 0,
  endByte = 20,
  hidden = false,
): ReportOccurrence {
  return { path, start_byte: startByte, end_byte: endByte, hidden };
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

function bandOf(rank: number, total: number): Severity {
  const percentile = total <= 1 ? 0 : 1 - (rank - 1) / (total - 1);
  if (percentile >= 0.99) return "worst";
  if (percentile >= 0.9) return "top10";
  if (percentile >= 0.5) return "mid";
  return "faint";
}
