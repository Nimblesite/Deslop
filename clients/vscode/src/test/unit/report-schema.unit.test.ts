// Unit tests for the report-schema pure helpers. Bucket tests exercise
// the canonical [CLONE-BUCKETS-ROUTING] table — every assertion here
// mirrors one row of that table.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import {
  ACT_NOW_BUCKETS,
  FUSED_THRESHOLD,
  bucketLabels,
  isActNow,
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

  // [CLONE-BUCKETS-ROUTING] These five rows were assertions about a
  // UI-local routing table that no longer exists. They are re-stated —
  // not weakened — against `resolveBucket`, the surface the extension
  // actually calls: the engine's label is what every row must produce,
  // whatever its signal triple would once have suggested. Two of the
  // five previously asserted the *defective* contract and are inverted
  // here; both inversions are called out on the row that carries them.
  test("resolveBucket carries the engine's identical verdict", () => {
    assert.equal(
      resolveBucket(cluster({ bucket: "identical", signals: signals(1.0, 1.0, 0) })),
      "identical",
    );
  });

  test("resolveBucket carries the engine's same_behavior verdict", () => {
    assert.equal(
      resolveBucket(cluster({ bucket: "same_behavior", signals: signals(0.2, 0.3, 0.9) })),
      "same_behavior",
    );
  });

  // ⚠️ INVERTED. This row used to assert `nearly_identical` for
  // `structural 0.00, token 0.95`. The engine calls that
  // `loosely_similar` — `classify_signals` requires `structural >= 0.20`
  // before a token signal can reach an act-now bucket and has no
  // low-structural arm at all — so the old expectation encoded the very
  // divergence this change removes. Every assertion is kept; the
  // expected value now agrees with the engine instead of contradicting it.
  test("resolveBucket carries a weak-shape verdict as the hint the engine made it", () => {
    const weakShape = cluster({
      bucket: "loosely_similar",
      signals: signals(0.0, 0.95, 0),
    });
    assert.equal(resolveBucket(weakShape), "loosely_similar");
    assert.equal(
      bucketLabels(resolveBucket(weakShape)).actionSentence,
      "Loose textual overlap. Treat as a hint.",
      "the user must not be told to act on a pair the engine ranked as a hint",
    );
  });

  test("resolveBucket carries the engine's fused-family near-miss verdict", () => {
    assert.equal(
      resolveBucket(cluster({ bucket: "nearly_identical", signals: signals(0.4, 0.96, 0) })),
      "nearly_identical",
    );
  });

  test("resolveBucket carries the engine's loosely_similar verdict", () => {
    assert.equal(
      resolveBucket(cluster({ bucket: "loosely_similar", signals: signals(0.3, 0.4, 0.2) })),
      "loosely_similar",
    );
  });

  test("resolveBucket carries the engine's structural_only verdict", () => {
    // The demoted tier is reachable only with content evidence, which is
    // not on the wire — so this row is proof on its own that the label
    // has to come from the engine.
    assert.equal(
      resolveBucket(cluster({ bucket: "structural_only", signals: signals(1.0, 0.0, 0) })),
      "structural_only",
    );
    assert.equal(
      bucketLabels("structural_only").plainTitle,
      "Same shape, different content",
    );
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

  // [CLONE-BUCKETS-ROUTING] The routing divergence found 17 Aug, pinned
  // at the surface that ships. `classifyCluster` claimed byte-for-byte
  // parity with `deslop-core::buckets::classify_signals` and had two arms
  // the engine never carried: it gated on `structural > 0.0` where the
  // engine gates on `structural >= 0.20`, and added
  // `structural <= 0.01 && token >= 0.9` outright. Both triples below are
  // `loosely_similar` in the engine, so a hint was repainted as an
  // act-now "Review the locations" on the flagship surface.
  test("a weak-shape pair the engine called a hint is never promoted to act-now", () => {
    for (const triple of [signals(0.1, 0.96, 0), signals(0.0, 0.92, 0)]) {
      const routed = resolveBucket(cluster({ bucket: "loosely_similar", signals: triple }));
      assert.equal(
        routed,
        "loosely_similar",
        `the engine's hint verdict must survive the triple ${JSON.stringify(triple)}`,
      );
      assert.equal(
        bucketLabels(routed).actionSentence,
        "Loose textual overlap. Treat as a hint.",
        "the user must not be told to act on a pair the engine ranked as a hint",
      );
      assert.equal(bucketLabels(routed).aiMatch, false);
    }
  });

  // DEFECT B1 — restored, re-stated against `resolveBucket`. The old
  // `classifyCluster` could not see content evidence (it is not on the
  // wire, #344) *and* was handed post-gate signals, so it read a proven
  // rename's corrected triple as "identical" and told the user "Safe to
  // extract — every copy is the same" about code whose identifiers all
  // differ. Every assertion is preserved; the surface under test is the
  // one the extension calls.
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test("a content-gated rename is never labelled byte-identical", () => {
    // A maximal Type-2 rename proven by its literal anchors: the engine
    // routes `nearly_identical` at fused 0.9 and renders token_jaccard
    // 1.0 because the Merkle match already proves the token multiset
    // (#232). The triple alone therefore reads "identical" — this is the
    // exact shape that produced the false claim.
    const rename = signals(1.0, 1.0, 0, 0.9);
    assert.ok(rename.fused < 1.0, "fixture: a proven rename is not full confidence");
    assert.equal(
      rename.structural,
      signals(1.0, 1.0, 0, 1.0).structural,
      "fixture: its shape evidence is indistinguishable from a verbatim copy",
    );
    const routed = resolveBucket(cluster({ bucket: "nearly_identical", signals: rename }));
    assert.equal(
      routed,
      "nearly_identical",
      "a rename below full confidence must not be labelled byte-identical",
    );
    assert.equal(
      bucketLabels(routed).actionSentence,
      "Review the locations — small differences may matter.",
      "the user must be told to review, not that extraction is safe",
    );
    assert.notEqual(
      bucketLabels(routed).actionSentence,
      bucketLabels("identical").actionSentence,
      "the rename must not borrow the byte-identical action sentence",
    );
  });

  // DEFECT B2 — restored, re-stated against `resolveBucket`. A shape-only
  // family fell through the old `structural >= 0.99` arm into an act-now
  // bucket — the exact false positive #341 exists to stop — because
  // `lacks_content_support` is invisible from the signal triple.
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test("a shape-only family the content gate demoted is never promoted", () => {
    // Sibling boilerplate: shape saturates, content evidence is absent,
    // so the engine demotes it to `structural_only` at fused 0.31.
    const shapeOnly = signals(1.0, 0.3, 0, 0.31);
    assert.ok(shapeOnly.fused < FUSED_THRESHOLD, "fixture: demoted, well under the cutoff");
    assert.ok(
      shapeOnly.structural >= 0.99,
      "fixture: its shape signal is exactly what used to promote it",
    );
    const routed = resolveBucket(cluster({ bucket: "structural_only", signals: shapeOnly }));
    assert.equal(
      routed,
      "structural_only",
      "shape without content evidence must never reach an act-now bucket",
    );
    assert.equal(
      bucketLabels(routed).plainTitle,
      "Same shape, different content",
      "the demoted family must keep its honest title",
    );
    assert.equal(bucketLabels(routed).aiMatch, false);
  });

  // The anti-regression assertion for the whole change: nothing in the
  // client may re-derive a bucket from a signal triple. A cluster whose
  // triple saturates on every axis, carrying no engine label, must still
  // come back as the hint bucket — any surviving re-derivation would
  // answer "identical" here.
  test("an unlabelled cluster is a hint, however loudly its signals saturate", () => {
    const unlabelled = resolveBucket(cluster({ bucket: "", signals: signals(1.0, 1.0, 1.0, 1.0) }));
    assert.equal(
      unlabelled,
      "loosely_similar",
      "a report with no engine verdict carries no verdict to render",
    );
    assert.equal(
      bucketLabels(unlabelled).actionSentence,
      "Loose textual overlap. Treat as a hint.",
    );
    assert.equal(
      resolveBucket(cluster({ bucket: "not_a_bucket", signals: signals(1.0, 1.0, 1.0, 1.0) })),
      "loosely_similar",
      "an unrecognised label is no more a verdict than a missing one",
    );
  });

  test("every routed bucket carries a coherent, self-consistent label set", () => {
    // Walks the routing table row by row. Each row asserts the bucket the
    // UI resolves, then that the labels it will render for that bucket are
    // usable on every surface: a jargon-free plain title, a hybrid title
    // carrying the bracketed taxonomy for AI scrapers, and a complete
    // action sentence. A row that routes correctly but renders an empty
    // or malformed label is still a broken user-facing surface.
    //
    // The `nearly_identical` row's triple is `structural 0.00,
    // token 0.95`, which the engine itself calls `loosely_similar` — it
    // is kept verbatim on purpose. It is now a *cross-check*: the engine
    // label must win over the triple, so a row whose two disagree is
    // exactly the row that catches a re-derivation coming back.
    const rows = [
      { signals: signals(1.0, 1.0, 0, 1.0), bucket: "identical" as const },
      { signals: signals(0.2, 0.3, 0.9, 0.9), bucket: "same_behavior" as const },
      { signals: signals(1.0, 0.0, 0.0, 0.31), bucket: "structural_only" as const },
      { signals: signals(0.0, 0.95, 0, 0.95), bucket: "nearly_identical" as const },
      { signals: signals(0.3, 0.4, 0.2, 0.4), bucket: "loosely_similar" as const },
    ];

    for (const row of rows) {
      const routed = resolveBucket(cluster({ bucket: row.bucket, signals: row.signals }));
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

  // ⚠️ INVERTED. This row used to assert that a report with no engine
  // label is re-routed from its signal triple to `identical`. That is the
  // defect: the client cannot see the content evidence, byte-equivalence
  // proof or member spread the engine routed on, so "identical" there is
  // manufactured — "Safe to extract — every copy is the same" asserted
  // about code nothing has proven. The assertion is kept and its expected
  // value corrected to the honest one.
  test("resolveBucket never manufactures a verdict for a v3 report with no bucket", () => {
    const bucket = resolveBucket(cluster({ bucket: "", signals: signals(1.0, 1.0, 0) }));
    assert.equal(bucket, "loosely_similar");
    assert.notEqual(
      bucket,
      "identical",
      "an unproven triple must never be presented as byte-identical",
    );
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

  // [VSIX-LIVE-BUBBLE] The act-now set is what the live bubble admits
  // without a second opinion, so it must be exactly the buckets whose
  // action sentence tells the user to do something now, and nothing else.
  test("the act-now set is exactly the buckets that tell the user to act", () => {
    assert.deepEqual([...ACT_NOW_BUCKETS], ["identical", "nearly_identical"]);
    assert.ok(isActNow("identical"), "a byte-proven copy is act-now");
    assert.ok(isActNow("nearly_identical"), "a proven near miss is act-now");
    assert.equal(
      isActNow("structural_only"),
      false,
      "the demoted tier says 'verify before extracting' — that is not act-now",
    );
    assert.equal(
      isActNow("loosely_similar"),
      false,
      "a hint is not something to act on",
    );
    assert.equal(
      isActNow("same_behavior"),
      false,
      "an AI match says 'read both before merging' — it earns its place on confidence, not on a verdict",
    );
    for (const bucket of ACT_NOW_BUCKETS) {
      assert.match(
        bucketLabels(bucket).actionSentence,
        /extract|Review/,
        `${bucket}: an act-now bucket must actually ask for an action`,
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
