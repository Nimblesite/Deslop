// Unit tests for the report-schema pure helpers. Bucket tests exercise
// the canonical [CLONE-BUCKETS-ROUTING] table — every assertion here
// mirrors one row of that table.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as reportModule from "../../types/report";
import {
  LIVE_BUBBLE_BUCKETS,
  bucketLabels,
  clusterInterpretation,
  measuredPairForCluster,
  isLiveBubbleBucket,
  occurrenceCount,
  resolveBucket,
  type ReportCluster,
  type ReportSignals,
} from "../../types/report";
import { wireCluster, type ClusterFixture } from "../cluster.helpers";
import { signalsWith } from "../signals.helpers";

const IDENTICAL_BUCKET = "identical";
const NEARLY_IDENTICAL_BUCKET = "nearly_identical";
const STRUCTURAL_ONLY_BUCKET = "structural_only";
const LOOSELY_SIMILAR_BUCKET = "loosely_similar";
const SAME_BEHAVIOR_BUCKET = "same_behavior";
const LOOSE_INTERPRETATION = "Engine-authored loose-match evidence.";
const UTF8_ENCODING = "utf8";
const LOW_SCORE = 0.2;
const SHAPE_SCORE = 0.3;
const MID_SCORE = 0.4;
const HIGH_SCORE = 0.9;
const TOKEN_ANCHOR_SCORE = 0.95;
const NEAR_TOKEN_SCORE = 0.96;
const FIXTURE_TEN = 10;
const ENGINE_OCCURRENCE_COUNT = 35;
const PAIR_COUNT = 2;
const HINT_INTERPRETATION_ASSERTION =
  "the client must carry the engine's hint interpretation unchanged";
const STRUCTURAL_ONLY_TITLE = "Same shape, different content";
const LEGACY_WORD_SUFFIX = "ict";

// The pair's evidence axes, staged exactly as the engine stamps them. There is no combined score to stage:
// `fused` is deleted from the wire, and no client fixture may carry one.
const signals = (
  s: number,
  j: number,
  e: number,
  // What the engine would have stamped as this triple's shape reading.
  // Staged here, never derived by the client ([FUSED-CONTENT-GATE]).
  shape = Math.max(s, j),
): ReportSignals =>
  signalsWith(IDENTICAL_BUCKET, {
    structural: s,
    token_jaccard: j,
    shape,
    embedding_cos: e,
  });

// Some rows here deliberately stage a bucket label the wire type
// forbids — the empty label a v3 report carries, and an unknown one —
// because `resolveBucket` must refuse to manufacture a verdict from
// either. The engine-derived fields still come from the known bucket
// when there is one.
type ClusterOverrides = Partial<Omit<ClusterFixture, "bucket">> & { bucket?: string };

const cluster = (overrides: ClusterOverrides = {}): ReportCluster => {
  const { bucket, ...rest } = overrides;
  const known = reportModule.BUCKETS.find((candidate) => candidate === bucket);
  const base = wireCluster({
    id: "x",
    weight: 1,
    size: 4,
    canonical_node_count: FIXTURE_TEN,
    bucket: known ?? IDENTICAL_BUCKET,
    signals: signals(0, 0, 0),
    occurrences: [
      { path: "A.cs", start_byte: 0, end_byte: FIXTURE_TEN, hidden: false },
      { path: "B.cs", start_byte: 0, end_byte: FIXTURE_TEN, hidden: false },
    ],
    ...rest,
  });
  return bucket === undefined ? base : { ...base, bucket };
};

function reportTypesPath(): string {
  const compiledRun = path.resolve(__dirname, "../../../src/types/report.ts");
  if (fs.existsSync(compiledRun)) {
    return compiledRun;
  }
  return path.resolve(__dirname, "../../types/report.ts");
}

function reportTypesSource(): string {
  return fs.readFileSync(reportTypesPath(), UTF8_ENCODING);
}

function legacyName(): string {
  return ["Verd", LEGACY_WORD_SUFFIX].join("");
}

/**
 * Asserts `resolveBucket` hands back the engine's own label for a signal
 * triple. Each [CLONE-BUCKETS-ROUTING] row respelled the same
 * build-cluster / resolve / compare shape; Deslop scored the copies
 * against this repo's own corpus. The row's name and comment stay on the
 * `test(..)` that carries them, so what each row proves is unchanged.
 */
function assertCarriesBucket(
  bucket: ReportCluster["bucket"],
  structural: number,
  token: number,
  embedding: number,
): void {
  assert.equal(resolveBucket(cluster({ bucket, signals: signals(structural, token, embedding) })), bucket);
}

suite("report schema helpers", () => {
  // The severity cut points were once client constants; the assertions
  // that pinned their values moved with them to
  // `deslop-core::report_weight::rank_band` and its
  // `rank_band_cut_points` test. The fused cutoff has a different fate:
  // it is deleted outright — from the engine, the wire, and this client —
  // because admission is the engine's bucket and nothing else. The test
  // below pins that no copy of either survived, and that the wire types
  // carry no fused field for a copy to hang off.
  test("the client owns neither a fused cutoff nor the severity cut points", () => {
    assert.ok(
      !("FUSED_THRESHOLD" in reportModule),
      "the reportable-confidence cutoff must exist only in the engine",
    );
    assert.ok(
      !("severityOf" in reportModule),
      "the severity cut points must exist only in the engine",
    );
    assert.ok(
      !("rankPercentile" in reportModule),
      "the rank percentile must exist only in the engine",
    );
    const proven = cluster({ bucket: IDENTICAL_BUCKET }) as Record<string, unknown>;
    const demoted = cluster({ bucket: STRUCTURAL_ONLY_BUCKET }) as Record<string, unknown>;
    assert.equal(
      "meets_fused_gate" in proven,
      false,
      "no cluster carries a gate verdict: admission is the bucket, not a flag",
    );
    assert.equal(
      "meets_fused_gate" in demoted,
      false,
      "the demoted family is demoted by its bucket label alone",
    );
  });

  // The wire contract itself: the generated types are the single source
  // the extension compiles against. If a fused field ever reappears on
  // them, every admission surface regains a threshold to argue with —
  // the exact defect this cutover removed.
  test("the generated wire types carry no fused field on signals or clusters", () => {
    const source = reportTypesSource();
    assert.doesNotMatch(source, /\bfused\b/, "no fused on the wire types");
    assert.doesNotMatch(source, /\bmeets_fused_gate\b/, "no gate flag on the wire types");
    const generated = fs.readFileSync(
      path.resolve(__dirname, "../../../src/types/wire-generated.ts"),
      UTF8_ENCODING,
    );
    assert.doesNotMatch(generated, /\bfused\b/, "no fused in the generated wire model");
    assert.doesNotMatch(
      generated,
      /\bmeets_fused_gate\b/,
      "no gate flag in the generated wire model",
    );
  });

  // [CLONE-BUCKETS-ROUTING] These five rows were assertions about a
  // UI-local routing table that no longer exists. They are re-stated —
  // not weakened — against `resolveBucket`, the surface the extension
  // actually calls: the engine's label is what every row must produce,
  // whatever its signal triple would once have suggested. Two of the
  // five previously asserted the *defective* contract and are inverted
  // here; both inversions are called out on the row that carries them.
  test("resolveBucket carries the engine's identical verdict", () => {
    assertCarriesBucket(IDENTICAL_BUCKET, 1.0, 1.0, 0);
  });

  test("resolveBucket carries the engine's same_behavior verdict", () => {
    assertCarriesBucket(SAME_BEHAVIOR_BUCKET, LOW_SCORE, SHAPE_SCORE, HIGH_SCORE);
  });

  // ⚠️ INVERTED. This row used to assert `nearly_identical` for
  // `structural 0.00, token 0.95`. The engine calls that
  // `loosely_similar` — `classify_signals` requires `structural >= 0.20`
  // before a token signal can reach a confirmed duplicate bucket and has no
  // low-structural arm at all — so the old expectation encoded the very
  // divergence this change removes. Every assertion is kept; the
  // expected value now agrees with the engine instead of contradicting it.
  test("resolveBucket carries a weak-shape verdict as the hint the engine made it", () => {
    const weakShape = cluster({
      bucket: LOOSELY_SIMILAR_BUCKET,
      signals: signals(0.0, TOKEN_ANCHOR_SCORE, 0),
      interpretation: LOOSE_INTERPRETATION,
    });
    assert.equal(resolveBucket(weakShape), LOOSELY_SIMILAR_BUCKET);
    assert.equal(
      clusterInterpretation(weakShape),
      LOOSE_INTERPRETATION,
      HINT_INTERPRETATION_ASSERTION,
    );
  });

  test("resolveBucket carries the engine's elected-pair near-miss verdict", () => {
    assertCarriesBucket(NEARLY_IDENTICAL_BUCKET, MID_SCORE, NEAR_TOKEN_SCORE, 0);
  });

  test("resolveBucket carries the engine's loosely_similar verdict", () => {
    assert.equal(
      resolveBucket(
        cluster({
          bucket: LOOSELY_SIMILAR_BUCKET,
          signals: signals(SHAPE_SCORE, MID_SCORE, LOW_SCORE),
        }),
      ),
      LOOSELY_SIMILAR_BUCKET,
    );
  });

  test("resolveBucket carries the engine's structural_only verdict", () => {
    // The demoted tier is reachable only with content evidence, which is
    // not on the wire — so this row is proof on its own that the label
    // has to come from the engine.
    assert.equal(
      resolveBucket(cluster({ bucket: STRUCTURAL_ONLY_BUCKET, signals: signals(1.0, 0.0, 0) })),
      STRUCTURAL_ONLY_BUCKET,
    );
    assert.equal(
      bucketLabels(STRUCTURAL_ONLY_BUCKET).plainTitle,
      STRUCTURAL_ONLY_TITLE,
    );
  });

  test("pair evidence is measured, and no combined score rides beside it", () => {
    // The pair evidence axes are measurements in [0,1]; a fixture carrying
    // anything else invalidates every family built on them. There is no
    // fused field left to bound: the type-level proof lives in the
    // generated-types test above, and the value-level proof is that the
    // staged fixtures cannot even spell the field.
    const staged = signals(1.0, SHAPE_SCORE, 0) as unknown as Record<string, unknown>;
    assert.equal("fused" in staged, false, "no staged fixture carries a fused value");
    for (const triple of [signals(1.0, 1.0, 0), signals(LOW_SCORE, SHAPE_SCORE, HIGH_SCORE)]) {
      for (const [axis, value] of Object.entries(triple)) {
        assert.ok(
          value >= 0 && value <= 1,
          `${axis} must be a measurement in [0,1], got ${value}`,
        );
      }
    }
  });

  test("pair evidence resolves only from two distinct carried occurrences", () => {
    const candidate = cluster();
    assert.deepEqual(measuredPairForCluster(candidate), {
      source: { left: 0, right: 1 },
      occurrences: candidate.occurrences,
    });
    const rejectedSources: ReportCluster["signal_source"][] = [
      undefined,
      { left: 0, right: 0 },
      { left: 0, right: PAIR_COUNT },
    ];
    for (const signalSource of rejectedSources) {
      assert.equal(
        measuredPairForCluster({ ...candidate, signal_source: signalSource }),
        undefined,
        "anonymous, self-referential, and out-of-range evidence must stay hidden",
      );
    }
  });

  // [CLONE-BUCKETS-ROUTING] The routing divergence found 17 Aug, pinned
  // at the surface that ships. `classifyCluster` claimed byte-for-byte
  // parity with `deslop-core::buckets::classify_signals` and had two arms
  // the engine never carried: it gated on `structural > 0.0` where the
  // engine gates on `structural >= 0.20`, and added
  // `structural <= 0.01 && token >= 0.9` outright. Both triples below are
  // `loosely_similar` in the engine, so a hint was repainted as a
  // confirmed near-miss on the flagship surface.
  test("a weak-shape pair keeps the engine's hint interpretation", () => {
    for (const triple of [signals(0.1, NEAR_TOKEN_SCORE, 0), signals(0.0, 0.92, 0)]) {
      const candidate = cluster({
        bucket: LOOSELY_SIMILAR_BUCKET,
        signals: triple,
        interpretation: LOOSE_INTERPRETATION,
      });
      const routed = resolveBucket(candidate);
      assert.equal(
        routed,
        LOOSELY_SIMILAR_BUCKET,
        `the engine's hint verdict must survive the triple ${JSON.stringify(triple)}`,
      );
      assert.equal(
        clusterInterpretation(candidate),
        LOOSE_INTERPRETATION,
        HINT_INTERPRETATION_ASSERTION,
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
  // → docs/plans/fused-score-followups.md § "Elected-pair evidence"
  test("a content-gated rename is never labelled byte-identical", () => {
    // A maximal Type-2 rename proven by its literal anchors: the engine
    // renders token_jaccard 1.0 because the Merkle match already proves
    // the token multiset (#232), while the elected pair's content
    // evidence shows the renaming. The structural axes alone therefore
    // read "identical" — this is the exact shape that produced the false
    // claim, and the bucket is what separates it.
    const rename = signalsWith(NEARLY_IDENTICAL_BUCKET, {
      structural: 1.0,
      token_jaccard: 1.0,
      shape: 1.0,
      embedding_cos: 0,
      pair_agreement: HIGH_SCORE,
      pair_rename_consistency: 1,
      literal_fraction: 0,
    });
    assert.ok(
      rename.pair_agreement < 1,
      "fixture: a renamed copy does not share every byte of matched content",
    );
    assert.equal(rename.token_jaccard, 1.0, "#232: the Merkle proof carries token_jaccard to 1.0");
    assert.equal(
      rename.structural,
      signals(1.0, 1.0, 0, 1.0).structural,
      "fixture: its shape evidence is indistinguishable from a verbatim copy",
    );
    const nearMissInterpretation = "Engine-authored near-miss evidence.";
    const renamedCluster = cluster({
      bucket: NEARLY_IDENTICAL_BUCKET,
      signals: rename,
      interpretation: nearMissInterpretation,
    });
    const routed = resolveBucket(renamedCluster);
    assert.equal(
      routed,
      NEARLY_IDENTICAL_BUCKET,
      "a rename below full confidence must not be labelled byte-identical",
    );
    assert.equal(
      clusterInterpretation(renamedCluster),
      nearMissInterpretation,
      "the client must render the engine-authored interpretation",
    );
    assert.notEqual(
      clusterInterpretation(renamedCluster),
      LOOSE_INTERPRETATION,
      "the near miss must not borrow a different cluster's interpretation",
    );
  });

  // DEFECT B2 — restored, re-stated against `resolveBucket`. A shape-only
  // family fell through the old `structural >= 0.99` arm into an act-now
  // bucket — the exact false positive #341 exists to stop — because
  // `lacks_content_support` is invisible from the signal triple.
  // → docs/plans/fused-score-followups.md § "Elected-pair evidence"
  test("a shape-only family the content gate demoted is never promoted", () => {
    // Sibling boilerplate: shape saturates, the elected pair shares almost
    // no content, so the engine demotes the family to `structural_only`.
    const shapeOnly = signalsWith(STRUCTURAL_ONLY_BUCKET, {
      structural: 1.0,
      token_jaccard: SHAPE_SCORE,
      shape: 1.0,
      embedding_cos: 0,
      pair_agreement: LOW_SCORE,
      pair_rename_consistency: 0,
      literal_fraction: 0.91,
    });
    const demoted = cluster({ bucket: STRUCTURAL_ONLY_BUCKET, signals: shapeOnly });
    assert.ok(
      shapeOnly.structural >= 0.99,
      "fixture: its shape signal is exactly what used to promote it",
    );
    const routed = resolveBucket(demoted);
    assert.equal(
      routed,
      STRUCTURAL_ONLY_BUCKET,
      "shape without content evidence must never reach an act-now bucket",
    );
    assert.equal(
      bucketLabels(routed).plainTitle,
      STRUCTURAL_ONLY_TITLE,
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
    const candidate = cluster({
      bucket: "",
      signals: signals(1.0, 1.0, 1.0, 1.0),
      interpretation: LOOSE_INTERPRETATION,
    });
    const unlabelled = resolveBucket(candidate);
    assert.equal(
      unlabelled,
      LOOSELY_SIMILAR_BUCKET,
      "a report with no engine verdict carries no verdict to render",
    );
    assert.equal(
      clusterInterpretation(candidate),
      LOOSE_INTERPRETATION,
    );
    assert.equal(
      resolveBucket(cluster({ bucket: "not_a_bucket", signals: signals(1.0, 1.0, 1.0, 1.0) })),
      LOOSELY_SIMILAR_BUCKET,
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
      { signals: signals(1.0, 1.0, 0, 1.0), bucket: IDENTICAL_BUCKET },
      {
        signals: signals(LOW_SCORE, SHAPE_SCORE, HIGH_SCORE, HIGH_SCORE),
        bucket: SAME_BEHAVIOR_BUCKET,
      },
      {
        signals: signals(1.0, 0.0, 0.0),
        bucket: STRUCTURAL_ONLY_BUCKET,
      },
      {
        signals: signals(0.0, TOKEN_ANCHOR_SCORE, 0, TOKEN_ANCHOR_SCORE),
        bucket: NEARLY_IDENTICAL_BUCKET,
      },
      {
        signals: signals(SHAPE_SCORE, MID_SCORE, LOW_SCORE, MID_SCORE),
        bucket: LOOSELY_SIMILAR_BUCKET,
      },
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
      assert.equal(
        labels.aiMatch,
        routed === SAME_BEHAVIOR_BUCKET,
        `${routed}: only the embedding-pass bucket is an AI match`,
      );
    }
  });

  test("report types do not keep legacy clone bucket aliases (#84)", () => {
    const source = reportTypesSource();
    const alias = legacyName();
    const helper = ["verd", LEGACY_WORD_SUFFIX, "Of"].join("");
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
        bucket: SAME_BEHAVIOR_BUCKET,
      }),
    );
    assert.equal(bucket, SAME_BEHAVIOR_BUCKET);
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
    assert.equal(bucket, LOOSELY_SIMILAR_BUCKET);
    assert.notEqual(
      bucket,
      IDENTICAL_BUCKET,
      "an unproven triple must never be presented as byte-identical",
    );
  });

  test("occurrenceCount reports the engine's count, not the loaded subset", () => {
    // The live wire truncates `occurrences`, so the carried list is not
    // the cluster. The count is computed once by
    // `deslop-core::report::occurrence_count` and read verbatim here.
    assert.equal(
      occurrenceCount(cluster({ occurrence_count: ENGINE_OCCURRENCE_COUNT })),
      ENGINE_OCCURRENCE_COUNT,
    );
    const truncated = cluster({
      occurrence_count: ENGINE_OCCURRENCE_COUNT,
      occurrences_truncated: true,
    });
    assert.equal(truncated.occurrences.length, PAIR_COUNT, "fixture: only two occurrences travelled");
    assert.equal(
      occurrenceCount(truncated),
      ENGINE_OCCURRENCE_COUNT,
      "a truncated wire list must never shrink the reported count",
    );
  });

  test("occurrenceCount never falls back to a client-derived number", () => {
    assert.equal(occurrenceCount(cluster()), 4, "the fixture's own count, stamped as the engine would");
    assert.equal(
      occurrenceCount(cluster({ occurrence_count: PAIR_COUNT })),
      PAIR_COUNT,
      "a smaller engine count is still the engine's answer",
    );
  });

  test("bucketLabels hybrid_title carries bracketed Type-N on every bucket", () => {
    assert.ok(bucketLabels(IDENTICAL_BUCKET).hybridTitle.includes("[Type-1/2]"));
    assert.ok(
      bucketLabels(NEARLY_IDENTICAL_BUCKET).hybridTitle.includes("[Type-3]"),
    );
    assert.ok(
      bucketLabels(LOOSELY_SIMILAR_BUCKET).hybridTitle.includes("[weak LSH]"),
    );
    assert.ok(bucketLabels(SAME_BEHAVIOR_BUCKET).hybridTitle.includes("[Type-4"));
  });

  test("bucketLabels plain_title never contains Type-N", () => {
    for (const b of [
      IDENTICAL_BUCKET,
      NEARLY_IDENTICAL_BUCKET,
      LOOSELY_SIMILAR_BUCKET,
      SAME_BEHAVIOR_BUCKET,
    ] as const) {
      const title = bucketLabels(b).plainTitle;
      assert.ok(
        !/\bType-\d/.test(title),
        `plain_title must be jargon-free: ${title}`,
      );
    }
  });

  // [VSIX-LIVE-BUBBLE] The live-bubble set is what the bubble admits
  // without a second opinion, so it must be exactly the buckets whose
  // engine-authored interpretation tells the user to act, and nothing
  // else. The interpretation is wire data ([VSIX-COMMON-RENDERING]): the
  // client passes it through untouched, and each eligible bucket's
  // engine sentence (staged here verbatim from
  // `deslop-core::buckets`) is the one that asks for an action.
  test("the live-bubble set is exactly the buckets whose engine verdict demands action", () => {
    assert.deepEqual([...LIVE_BUBBLE_BUCKETS], [IDENTICAL_BUCKET, NEARLY_IDENTICAL_BUCKET]);
    assert.ok(isLiveBubbleBucket(IDENTICAL_BUCKET), "a byte-proven copy is bubble-eligible");
    assert.ok(
      isLiveBubbleBucket(NEARLY_IDENTICAL_BUCKET),
      "a proven near miss is bubble-eligible",
    );
    assert.equal(
      isLiveBubbleBucket(STRUCTURAL_ONLY_BUCKET),
      false,
      "the demoted tier says 'verify before extracting' — the bubble must not render it",
    );
    assert.equal(
      isLiveBubbleBucket(LOOSELY_SIMILAR_BUCKET),
      false,
      "a hint is not something to act on",
    );
    assert.equal(
      isLiveBubbleBucket(SAME_BEHAVIOR_BUCKET),
      false,
      "an AI match earns its place on confidence, not on a verdict",
    );
    const ENGINE_SENTENCES: { bucket: string; sentence: string }[] = [
      { bucket: IDENTICAL_BUCKET, sentence: "Safe to extract — every copy is the same." },
      {
        bucket: NEARLY_IDENTICAL_BUCKET,
        sentence: "Review the locations — small differences may matter.",
      },
    ];
    for (const { bucket, sentence } of ENGINE_SENTENCES) {
      const routed = cluster({ bucket, interpretation: sentence });
      const interpretation = clusterInterpretation(routed);
      assert.equal(
        interpretation,
        sentence,
        `${bucket}: the client must pass the engine's sentence through untouched`,
      );
      assert.match(
        interpretation,
        /extract|Review/,
        `${bucket}: a bubble-eligible bucket must carry an engine sentence that asks for action`,
      );
    }
  });

  test("only same_behavior is flagged as an AI match", () => {
    assert.equal(bucketLabels(IDENTICAL_BUCKET).aiMatch, false);
    assert.equal(bucketLabels(NEARLY_IDENTICAL_BUCKET).aiMatch, false);
    assert.equal(bucketLabels(LOOSELY_SIMILAR_BUCKET).aiMatch, false);
    assert.equal(bucketLabels(SAME_BEHAVIOR_BUCKET).aiMatch, true);
  });
});
