// Seven-field pair `ReportSignals` fixtures.
//
// Every axis is one admitted pair's measurement — the pair named by
// `signal_source`, when the wire names one — never a cluster mean. One builder instead of twenty
// copy-pasted literals, keyed by the engine's bucket so the evidence a
// fixture claims stays coherent with the bucket: an `identical` cluster's
// pair shares all of its content, a `structural_only` cluster's
// shares almost none of it. There is no combined score on the wire to
// fixture: admission and routing are the engine's bucket verdict.

import type { Bucket, ReportSignals } from "../types/report";

/** The signal breakdown a fixture cluster of `bucket` carries. `shape`
 * is spelled out per bucket rather than reduced from the two shape axes
 * here — the engine owns that reduction ([FUSED-CONTENT-GATE]) and a
 * fixture that recomputed it could never catch the engine getting it
 * wrong. */
export function bucketSignals(bucket: Bucket): ReportSignals {
  switch (bucket) {
    case "nearly_identical":
      return signals(0.99, 0.96, 0.99, 0, 0.97, 1, 0);
    case "structural_only":
      return signals(1, 1, 1, 0, 0.16, 0, 0.85);
    case "loosely_similar":
      return signals(0.2, 0.4, 0.4, 0, 0.35, 0, 0);
    case "same_behavior":
      return signals(0.2, 0.3, 0.3, 0.9, 0.05, 0, 0);
    case "identical":
      return signals(1, 1, 1, 0, 1, 1, 0);
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
  shape: number,
  embeddingCos: number,
  pairAgreement: number,
  pairRenameConsistency: number,
  literalFraction: number,
): ReportSignals {
  return {
    structural,
    token_jaccard: tokenJaccard,
    shape,
    embedding_cos: embeddingCos,
    pair_agreement: pairAgreement,
    pair_rename_consistency: pairRenameConsistency,
    literal_fraction: literalFraction,
  };
}
