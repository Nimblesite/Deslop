// Seven-field `ReportSignals` fixtures ([FUSION-CONTENT-GATE], #344).
//
// The four-field literal `{ structural, token_jaccard, embedding_cos, fused }`
// was copy-pasted across twenty suites. Once the wire carried the measured
// content evidence — the only thing separating a corroborated Type-2 rename
// from an anchor-poor scaffolding family, since both render structural 1.00
// and jaccard 1.00 — twenty copies became twenty places to leave that evidence
// at a dishonest zero under a perfect shape score.
//
// One builder instead, keyed by the engine's bucket so the confidence and the
// evidence a fixture claims stay coherent with each other: an `identical`
// cluster shares all of its content, a `structural_only` cluster shares almost
// none of it, and the fused score each carries is what that evidence supports.

import type { Bucket, ReportSignals } from "../types/report";

/** The signal breakdown a fixture cluster of `bucket` carries. */
export function bucketSignals(bucket: Bucket): ReportSignals {
  switch (bucket) {
    case "nearly_identical":
      return signals(0.99, 0.96, 0, 0.96, 0.97, 1, 0);
    case "structural_only":
      return signals(1, 1, 0, 0.16, 0.16, 0, 0.85);
    case "loosely_similar":
      return signals(0.2, 0.4, 0, 0.4, 0.35, 0, 0);
    case "same_behavior":
      return signals(0.2, 0.3, 0.9, 0.9, 0.05, 0, 0);
    case "identical":
      return signals(1, 1, 0, 1, 1, 1, 0);
  }
}

/** A bucket's breakdown with specific fields pinned by the suite under test. */
export function signalsWith(
  bucket: Bucket,
  overrides: Partial<ReportSignals> = {},
): ReportSignals {
  return { ...bucketSignals(bucket), ...overrides };
}

function signals(
  structural: number,
  tokenJaccard: number,
  embeddingCos: number,
  fused: number,
  agreement: number,
  renameConsistency: number,
  literalFraction: number,
): ReportSignals {
  return {
    structural,
    token_jaccard: tokenJaccard,
    embedding_cos: embeddingCos,
    fused,
    agreement,
    rename_consistency: renameConsistency,
    literal_fraction: literalFraction,
  };
}
