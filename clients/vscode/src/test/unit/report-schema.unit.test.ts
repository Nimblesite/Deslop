// Unit tests for the report-schema pure helpers. Bucket tests exercise
// the canonical [CLONE-BUCKETS-ROUTING] table — every assertion here
// mirrors one row of that table.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import {
  FUSED_THRESHOLD,
  bucketLabels,
  classifyCluster,
  occurrenceCount,
  resolveBucket,
  severityOf,
  type ReportCluster,
  type ReportSignals,
} from "../../types/report";

const signals = (s: number, j: number, e: number): ReportSignals => ({
  structural: s,
  token_jaccard: j,
  embedding_cos: e,
  fused: s + j + e,
});

const cluster = (overrides: Partial<ReportCluster> = {}): ReportCluster => ({
  id: "x",
  weight: 1,
  size: 4,
  canonical_node_count: 10,
  signals: signals(0, 0, 0),
  occurrences: [
    { path: "A.cs", start_byte: 0, end_byte: 10, hidden: false },
    { path: "B.cs", start_byte: 0, end_byte: 10, hidden: false },
  ],
  summary: "",
  interpretation: "",
  ...overrides,
});

function reportTypesPath(): string {
  const compiledRun = path.resolve(__dirname, "../../../src/types/report.ts");
  if (fs.existsSync(compiledRun)) {
    return compiledRun;
  }
  return path.resolve(__dirname, "../../types/report.ts");
}

function reportTypesSource(): string {
  return fs.readFileSync(reportTypesPath(), "utf8");
}

function legacyName(): string {
  return ["Verd", "ict"].join("");
}

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

  test("classifyCluster identical when both signals saturate", () => {
    assert.equal(classifyCluster(signals(1.0, 1.0, 0)), "identical");
  });

  test("classifyCluster same_behavior when embedding dominates syntactic mismatch", () => {
    assert.equal(classifyCluster(signals(0.2, 0.3, 0.9)), "same_behavior");
  });

  test("classifyCluster nearly_identical on high jaccard + low structural", () => {
    assert.equal(classifyCluster(signals(0.0, 0.95, 0)), "nearly_identical");
  });

  test("classifyCluster nearly_identical on fused-family band", () => {
    assert.equal(classifyCluster(signals(0.4, 0.96, 0)), "nearly_identical");
  });

  test("classifyCluster loosely_similar as the safe fallback", () => {
    assert.equal(classifyCluster(signals(0.3, 0.4, 0.2)), "loosely_similar");
  });

  test("report types do not keep legacy clone bucket aliases (#84)", () => {
    const source = reportTypesSource();
    const alias = legacyName();
    const helper = ["verd", "ict", "Of"].join("");
    assert.doesNotMatch(source, new RegExp(`export\\s+type\\s+${alias}\\b`));
    assert.doesNotMatch(source, new RegExp(`function\\s+${helper}\\b`));
    assert.doesNotMatch(source, new RegExp(`Legacy\\s+${alias}`));
    assert.doesNotMatch(source, /\bDUPLICATE\b/);
    assert.doesNotMatch(source, /\bNEAR-MISS\b/);
    assert.doesNotMatch(source, /\bSEMANTIC MATCH\b/);
  });

  test("resolveBucket prefers JSON wire label over recomputation", () => {
    const bucket = resolveBucket(
      cluster({
        bucket: "same_behavior",
      }),
    );
    assert.equal(bucket, "same_behavior");
  });

  test("resolveBucket falls back to signals when v3 JSON has no bucket", () => {
    const bucket = resolveBucket(cluster({ signals: signals(1.0, 1.0, 0) }));
    assert.equal(bucket, "identical");
  });

  test("occurrenceCount prefers the authoritative total over the loaded subset", () => {
    assert.equal(occurrenceCount(cluster({ occurrences_total: 35 })), 35);
  });

  test("occurrenceCount falls back to size when total is missing or zero", () => {
    assert.equal(occurrenceCount(cluster()), 4);
    assert.equal(occurrenceCount(cluster({ occurrences_total: 0 })), 4);
  });

  test("bucketLabels hybrid_title carries bracketed Type-N on every bucket", () => {
    assert.ok(bucketLabels("identical").hybridTitle.includes("[Type-1/2]"));
    assert.ok(
      bucketLabels("nearly_identical").hybridTitle.includes("[Type-3]"),
    );
    assert.ok(
      bucketLabels("loosely_similar").hybridTitle.includes("[weak LSH]"),
    );
    assert.ok(bucketLabels("same_behavior").hybridTitle.includes("[Type-4"));
  });

  test("bucketLabels plain_title never contains Type-N", () => {
    for (const b of [
      "identical",
      "nearly_identical",
      "loosely_similar",
      "same_behavior",
    ] as const) {
      const title = bucketLabels(b).plainTitle;
      assert.ok(
        !/\bType-\d/.test(title),
        `plain_title must be jargon-free: ${title}`,
      );
    }
  });

  test("only same_behavior is flagged as an AI match", () => {
    assert.equal(bucketLabels("identical").aiMatch, false);
    assert.equal(bucketLabels("nearly_identical").aiMatch, false);
    assert.equal(bucketLabels("loosely_similar").aiMatch, false);
    assert.equal(bucketLabels("same_behavior").aiMatch, true);
  });
});
