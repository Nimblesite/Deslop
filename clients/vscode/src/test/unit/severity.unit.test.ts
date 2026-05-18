// Unit tests for severity bucketing. Pure function — no VS Code needed.

import * as assert from "node:assert/strict";
import {
  rankPercentile,
  severityForRank,
  indexedSeverity,
} from "../../severity";
import { ReportCluster } from "../../types/report";

function cluster(id: string): ReportCluster {
  return {
    id,
    weight: 0,
    size: 0,
    canonical_node_count: 0,
    bucket: "identical",
    signals: { structural: 0, token_jaccard: 0, embedding_cos: 0, fused: 0 },
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
});
