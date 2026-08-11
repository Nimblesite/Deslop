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

// `fused` is a confidence in [0,1], never a raw sum — the engine's gate
// multiplies shape evidence by content evidence ([FUSION-CONTENT-GATE]).
// Tests that need a specific band pass it explicitly.
const signals = (
  s: number,
  j: number,
  e: number,
  fused = Math.min(1, Math.max(s, j, e)),
): ReportSignals => ({
  structural: s,
  token_jaccard: j,
  embedding_cos: e,
  fused,
});

const cluster = (overrides: Partial<ReportCluster> = {}): ReportCluster => ({
  id: "x",
  weight: 1,
  size: 4,
  canonical_node_count: 10,
  bucket: "identical",
  signals: signals(0, 0, 0),
  occurrences: [
    { path: "A.cs", start_byte: 0, end_byte: 10, hidden: false },
    { path: "B.cs", start_byte: 0, end_byte: 10, hidden: false },
  ],
  occurrences_total: 0,
  occurrences_truncated: false,
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

  test("fused is a confidence in [0,1] that the content gate may pull below shape", () => {
    // A fused value outside the unit interval is not a confidence, and a
    // fixture carrying one silently invalidates every band built on it.
    // The gate is one-directional: content evidence can only discount
    // shape evidence, never inflate it past full confidence.
    const gated = signals(1.0, 0.3, 0, 0.31);
    for (const triple of [signals(1.0, 1.0, 0), signals(0.2, 0.3, 0.9), gated]) {
      assert.ok(
        triple.fused >= 0 && triple.fused <= 1,
        `fused must be a confidence in [0,1], got ${triple.fused}`,
      );
    }
    assert.ok(
      gated.fused < gated.structural,
      "a demoted cluster's confidence must sit below its shape evidence",
    );
    assert.equal(
      signals(1.0, 1.0, 0).fused,
      1,
      "byte-identical evidence carries full confidence",
    );
  });

  test("classifyCluster must not call a content-gated rename byte-identical", () => {
    // A maximal Type-2 rename proven by its literal anchors: the engine
    // routes `nearly_identical` at fused 0.9, and renders token_jaccard
    // 1.0 because the Merkle match already proves the token multiset
    // (#232). Reading the triple alone therefore says "identical" —
    // "Safe to extract — every copy is the same" — about code whose
    // identifiers all differ. The UI must not out-claim the engine.
    const rename = signals(1.0, 1.0, 0, 0.9);
    assert.ok(rename.fused < 1.0, "fixture: a proven rename is not full confidence");
    assert.equal(
      classifyCluster(rename),
      "nearly_identical",
      "a rename below full confidence must not be labelled byte-identical",
    );
    assert.equal(
      bucketLabels(classifyCluster(rename)).actionSentence,
      "Review the locations — small differences may matter.",
      "the user must be told to review, not that extraction is safe",
    );
  });

  test("classifyCluster must not promote a shape-only family the content gate demoted", () => {
    // Sibling boilerplate: shape saturates, content evidence is absent,
    // so the engine demotes it to `structural_only` at fused 0.31. The
    // signal triple alone reads `structural >= 0.99` and promotes it to
    // an act-now bucket — the exact false positive #341 exists to stop.
    const shapeOnly = signals(1.0, 0.3, 0, 0.31);
    assert.ok(shapeOnly.fused < FUSED_THRESHOLD, "fixture: demoted, well under the cutoff");
    assert.equal(
      classifyCluster(shapeOnly),
      "structural_only",
      "shape without content evidence must never reach an act-now bucket",
    );
    assert.equal(
      bucketLabels(classifyCluster(shapeOnly)).plainTitle,
      "Same shape, different content",
      "the demoted family must keep its honest title",
    );
  });

  test("every routed bucket carries a coherent, self-consistent label set", () => {
    // Walks the routing table row by row. Each row asserts the bucket the
    // UI derives, then that the labels it will render for that bucket are
    // usable on every surface: a jargon-free plain title, a hybrid title
    // carrying the bracketed taxonomy for AI scrapers, and a complete
    // action sentence. A row that routes correctly but renders an empty
    // or malformed label is still a broken user-facing surface.
    const rows = [
      { signals: signals(1.0, 1.0, 0, 1.0), bucket: "identical" as const },
      { signals: signals(0.2, 0.3, 0.9, 0.9), bucket: "same_behavior" as const },
      { signals: signals(1.0, 0.0, 0.0, 0.31), bucket: "structural_only" as const },
      { signals: signals(0.0, 0.95, 0, 0.95), bucket: "nearly_identical" as const },
      { signals: signals(0.3, 0.4, 0.2, 0.4), bucket: "loosely_similar" as const },
    ];

    for (const row of rows) {
      const routed = classifyCluster(row.signals);
      assert.equal(
        routed,
        row.bucket,
        `routing drifted for ${JSON.stringify(row.signals)}`,
      );
      const labels = bucketLabels(routed);
      assert.ok(labels.plainTitle.length > 0, `${routed}: plain title must not be empty`);
      assert.doesNotMatch(
        labels.plainTitle,
        /\bType-\d/,
        `${routed}: the plain title must stay jargon-free`,
      );
      assert.match(
        labels.hybridTitle,
        /\[.+\]/,
        `${routed}: the hybrid title must carry a bracketed taxonomy`,
      );
      assert.ok(
        labels.hybridTitle.startsWith(labels.plainTitle),
        `${routed}: the hybrid title must extend the plain title, not restate it`,
      );
      assert.match(
        labels.actionSentence,
        /\.$/,
        `${routed}: the action sentence must be a complete sentence`,
      );
      assert.equal(
        labels.aiMatch,
        routed === "same_behavior",
        `${routed}: only the embedding-pass bucket is an AI match`,
      );
    }
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
