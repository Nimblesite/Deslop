// Unit tests for severity bucketing. Pure function — no VS Code needed.

import * as assert from "node:assert/strict";
import {
  rankPercentile,
  severityForRank,
  indexedSeverity,
} from "../../severity";
import { Bucket, ReportCluster } from "../../types/report";

function cluster(id: string, fused = 0, bucket: Bucket = "identical"): ReportCluster {
  return {
    id,
    weight: 0,
    size: 0,
    canonical_node_count: 0,
    bucket,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused },
    occurrences: [],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

suite("severity", () => {
  test("rankPercentile handles single-element list", () => {
    assert.equal(rankPercentile(1, 1), 0);
  });

  test("rankPercentile handles empty", () => {
    assert.equal(rankPercentile(1, 0), 0);
  });

  test("rankPercentile rank 1 of N is top", () => {
    assert.equal(rankPercentile(1, 100), 1);
  });

  test("rankPercentile rank N of N is zero", () => {
    assert.equal(rankPercentile(100, 100), 0);
  });

  test("severityForRank worst bucket", () => {
    assert.equal(severityForRank(1, 100), "worst");
  });

  test("severityForRank top10 bucket", () => {
    assert.equal(severityForRank(5, 100), "top10");
  });

  test("severityForRank mid bucket", () => {
    assert.equal(severityForRank(40, 100), "mid");
  });

  test("severityForRank faint bucket", () => {
    assert.equal(severityForRank(80, 100), "faint");
  });

  test("indexedSeverity produces correct buckets", () => {
    const clusters = Array.from({ length: 10 }, (_, i) => cluster(`c${i}`));
    const map = indexedSeverity(clusters);
    assert.equal(map.size, 10);
    assert.equal(map.get("c0"), "worst");
    assert.equal(map.get("c9"), "faint");
  });

  test("indexedSeverity on empty returns empty map", () => {
    assert.equal(indexedSeverity([]).size, 0);
  });

  test("severity never brightens as rank worsens, at any confidence", () => {
    // Whatever severity reads, it must be monotonic down the report: a
    // cluster further from the top can never be painted louder than one
    // above it, or the decoration ordering contradicts the ranking.
    const order = ["worst", "top10", "mid", "faint"] as const;
    const clusters = [
      cluster("a", 1.0),
      cluster("b", 0.3, "structural_only"),
      cluster("c", 0.95),
      cluster("d", 0.31, "structural_only"),
      ...Array.from({ length: 16 }, (_, i) => cluster(`e${i}`, 0.9)),
    ];
    const map = indexedSeverity(clusters);

    assert.equal(map.size, clusters.length, "every cluster must be assigned a severity");
    let previous = 0;
    for (const [index, entry] of clusters.entries()) {
      const severity = map.get(entry.id);
      assert.ok(severity !== undefined, `cluster ${entry.id} must have a severity`);
      const band = severity ?? "faint";
      const position = order.indexOf(band);
      assert.ok(position >= 0, `${entry.id}: severity must be a known band`);
      assert.ok(
        position >= previous,
        `${entry.id} at rank ${index + 1} brightened to ${band} after ${order[previous] ?? "faint"}`,
      );
      previous = position;
    }
    assert.equal(map.get("a"), "worst", "the top-ranked cluster is the loudest");
    assert.equal(
      map.get(`e15`),
      "faint",
      "the last-ranked cluster is the quietest",
    );
  });

  // 🛑 SKIPPED — DEFECT D. This test is correct and the code is wrong.
  // Severity is derived purely from rank, so a large shape-only family
  // that still sorts first is painted "worst" — the loudest decoration in
  // the editor — while the engine says "verify before extracting".
  // Skipped under an explicit owner mandate to unblock the release. Do
  // not delete, do not weaken: un-skip it as part of the fix.
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test.skip("a demoted shape-only family is not painted with act-now severity", () => {
    // Ranking now scales weight by the content-gated confidence, but a
    // large enough shape-only family can still sort first. Severity is
    // pure rank, so that family gets the "worst" paint — the loudest
    // decoration in the editor — while the engine says "verify before
    // extracting". The colour must follow the evidence, not just the
    // position ([FUSION-CONTENT-GATE], #344).
    const clusters = [
      cluster("shape-giant", 0.31, "structural_only"),
      cluster("proven", 0.95, "identical"),
      ...Array.from({ length: 8 }, (_, i) => cluster(`filler-${i}`, 0.9)),
    ];
    const map = indexedSeverity(clusters);

    assert.equal(map.size, 10, "every cluster must be assigned a severity");
    assert.equal(
      map.get("shape-giant"),
      "faint",
      "a cluster the content gate demoted must never be painted the loudest",
    );
    assert.notEqual(
      map.get("shape-giant"),
      map.get("proven"),
      "a demoted family and a proven clone must not share a severity",
    );
    assert.equal(
      clusters[0]?.bucket,
      "structural_only",
      "fixture: the demoted family is the top-ranked cluster",
    );
    assert.ok(
      (clusters[0]?.signals.fused ?? 1) < (clusters[1]?.signals.fused ?? 0),
      "fixture: it also carries strictly lower confidence than the proven clone",
    );
  });
});
