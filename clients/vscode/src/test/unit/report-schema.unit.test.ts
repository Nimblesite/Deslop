// Unit tests for the report-schema pure helpers.

import * as assert from "node:assert/strict";
import {
  FUSED_THRESHOLD,
  severityOf,
  verdictOf,
} from "../../types/report";

suite("report schema helpers", () => {
  test("FUSED_THRESHOLD is 0.85", () => {
    assert.equal(FUSED_THRESHOLD, 0.85);
  });

  test("severityOf worst boundary", () => {
    assert.equal(severityOf(0.995), "worst");
    assert.equal(severityOf(1.0), "worst");
  });

  test("severityOf top10 boundary", () => {
    assert.equal(severityOf(0.95), "top10");
    assert.equal(severityOf(0.9), "top10");
  });

  test("severityOf mid boundary", () => {
    assert.equal(severityOf(0.75), "mid");
    assert.equal(severityOf(0.5), "mid");
  });

  test("severityOf faint boundary", () => {
    assert.equal(severityOf(0.49), "faint");
    assert.equal(severityOf(0), "faint");
  });

  test("verdictOf DUPLICATE on structural 1.0", () => {
    assert.equal(
      verdictOf({ structural: 1.0, token_jaccard: 0, embedding_cos: 0, fused: 1.0 }),
      "DUPLICATE",
    );
  });

  test("verdictOf NEAR-MISS on jaccard >= 0.9", () => {
    assert.equal(
      verdictOf({ structural: 0.5, token_jaccard: 0.95, embedding_cos: 0, fused: 0.9 }),
      "NEAR-MISS",
    );
  });

  test("verdictOf SEMANTIC MATCH on everything else", () => {
    assert.equal(
      verdictOf({ structural: 0.5, token_jaccard: 0.3, embedding_cos: 0.9, fused: 0.85 }),
      "SEMANTIC MATCH",
    );
  });
});
