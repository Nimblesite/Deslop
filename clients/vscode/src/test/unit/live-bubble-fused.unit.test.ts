// Unit: how the live bubble consumes the content-gated fused confidence
// ([FUSION-CONTENT-GATE], [VSIX-LIVE-BUBBLE]).
//
// The engine multiplies shape evidence by content evidence, so `fused` no
// longer saturates for every act-now cluster: a proven maximal rename
// renders 0.9 and a Type-3 near miss with moderate positional agreement
// renders lower still, while the engine routes both to an act-now bucket.
// These tests pin what the user must see on the live surface as a result.

import * as assert from "node:assert/strict";
import { LiveBubble, signalStrip } from "../../bubble/live";
import { ReportStore } from "../../reportStore";
import { FUSED_THRESHOLD, bucketLabels } from "../../types/report";
import {
  BubbleCapture,
  assertBubbleShows,
  bubbleCluster,
  capturingEditor,
  setBubbleMode,
  span,
} from "./bubble.helpers";
import { reportWithClusters } from "./report.helpers";

// Buckets the engine considers actionable — the user is told to act on
// these, so the live surface must not withhold them.
const ACT_NOW = ["identical", "nearly_identical"] as const;

const SHAPE_ONLY_TITLE = bucketLabels("structural_only").plainTitle;

// Asserts the surface is showing `title` and nothing from a demoted bucket.
function assertShowing(
  capture: BubbleCapture,
  title: string,
  context: string,
): string {
  const visible = assertBubbleShows(capture, title, context);
  assert.doesNotMatch(
    visible,
    new RegExp(SHAPE_ONLY_TITLE),
    `${context}: a demoted title must never appear`,
  );
  return visible;
}

suite("LiveBubble fused confidence", () => {
  // DEFECT A — restored. `bestBubbleCluster` gated on a UI-local
  // `fused >= FUSED_THRESHOLD` instead of the engine's bucket, so act-now
  // clusters below 0.85 were silently withheld from the flagship live
  // surface. `bubbleAdmits` now takes the engine's verdict for an act-now
  // bucket and keeps the fused cutoff for everything below it.
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test("an act-now near miss below the fused cutoff still reaches the bubble", async () => {
    // A genuine Type-3 near miss: identical shape, real edits, so
    // positional agreement is ~0.8 and the gate renders fused = 0.80
    // while the engine still routes `nearly_identical`. The bubble must
    // follow the engine's routing — a UI-local cutoff that hides an
    // act-now cluster is a false negative on the flagship live surface.
    const near = bubbleCluster("c-near", 40, 0.8, {
      bucket: "nearly_identical",
      structural: 1,
      token: 1,
      occurrenceTotal: 3,
    });
    const store = new ReportStore();
    store.setSnapshot(reportWithClusters([near]), 0);
    await setBubbleMode("inline");
    const capture = capturingEditor();
    const bubble = new LiveBubble(store, () => undefined);

    try {
      // 1. The user's cursor lands on the near miss.
      bubble.render(capture.editor, span(0), [near]);
      const visible = capture.visible();
      assert.ok(
        visible !== undefined,
        `an act-now ${near.bucket} cluster must bubble at fused ${near.signals.fused}, ` +
          `below FUSED_THRESHOLD ${FUSED_THRESHOLD}: ${JSON.stringify(capture.calls)}`,
      );
      assert.ok(
        near.signals.fused < FUSED_THRESHOLD,
        "fixture must sit below the cutoff or it proves nothing",
      );
      assert.ok(
        ACT_NOW.includes(near.bucket as (typeof ACT_NOW)[number]),
        "fixture must be an act-now bucket",
      );
      assert.match(visible ?? "", /Nearly identical code/, "bubble carries the engine's title");
      assert.doesNotMatch(
        visible ?? "",
        new RegExp(SHAPE_ONLY_TITLE),
        "a content-supported near miss must never render as shape-only",
      );
      assert.match(visible ?? "", /×\s*3/, "bubble renders the occurrence count");
      assert.match(visible ?? "", /A\.cs/, "bubble names the canonical file");
      assert.ok(capture.visibleHover() !== undefined, "inline bubble carries a hover card");

      // 2. The user moves the cursor within the same cluster.
      bubble.render(capture.editor, span(6), [near]);
      const moved = assertShowing(capture, "Nearly identical code", "after cursor move");
      assert.match(moved, /×\s*3/, "the count survives a cursor move");
      assert.ok(capture.visibleHover() !== undefined, "the hover survives a cursor move");

      // 3. The user switches to ghost mode.
      await setBubbleMode("ghost");
      bubble.render(capture.editor, span(12), [near]);
      const ghost = assertShowing(capture, "Nearly identical code", "in ghost mode");
      assert.match(ghost, /└─/, "ghost mode renders the tree-branch prefix");
      assert.match(ghost, /[▁▂▃▄▅▆▇█]{3}/u, "ghost mode renders the signal strip");
      assert.equal(capture.visibleHover(), undefined, "ghost decorations carry no hover");

      // 4. The user dismisses it; the act-now cluster must stay gone.
      await setBubbleMode("inline");
      bubble.dismissCluster("c-near");
      bubble.render(capture.editor, span(18), [near]);
      assert.equal(
        capture.visible(),
        undefined,
        "a dismissed act-now cluster must not return",
      );
      assert.ok(
        capture.history().every((text) => !text.includes(SHAPE_ONLY_TITLE)),
        `no demoted title may ever have been painted: ${capture.history().join(" | ")}`,
      );
    } finally {
      await setBubbleMode("inline");
      bubble.dispose();
    }
  });

  test("a demoted shape-only family never wins the bubble over a proven clone", async () => {
    // The #341 contract on the live surface: a huge sibling-boilerplate
    // family outweighs a small proven clone, but its content evidence is
    // absent, so it must never be the thing the user is nudged to act on.
    const shapeOnly = bubbleCluster("c-shape", 900, 0.31, {
      bucket: "structural_only",
      structural: 1,
      token: 0.3,
    });
    const proven = bubbleCluster("c-proven", 40, 0.9, {
      bucket: "nearly_identical",
      structural: 1,
      token: 1,
    });
    const store = new ReportStore();
    store.setSnapshot(reportWithClusters([shapeOnly, proven]), 0);
    await setBubbleMode("inline");
    const capture = capturingEditor();
    const bubble = new LiveBubble(store, () => undefined);

    try {
      // 1. The probe returns both; the heavier demoted family must lose.
      bubble.render(capture.editor, span(0), [shapeOnly, proven]);
      assert.ok(
        shapeOnly.weight > proven.weight,
        "fixture must stage the demoted family as the heavier cluster",
      );
      const visible = assertShowing(capture, "Nearly identical code", "with both in probe");
      assert.match(visible, /A\.cs/, "the winner names its canonical file");
      assert.match(visible, /×\s*2/, "and carries its occurrence count");
      assert.ok(capture.visibleHover() !== undefined, "the winner carries a hover card");

      // 2. The cursor moves somewhere only the demoted family covers.
      bubble.render(capture.editor, span(6), [shapeOnly]);
      assert.equal(
        capture.visible(),
        undefined,
        `a demoted family alone (fused ${shapeOnly.signals.fused}) must show nothing`,
      );

      // 3. Back onto the proven clone — it returns unchanged.
      bubble.render(capture.editor, span(12), [shapeOnly, proven]);
      assertShowing(capture, "Nearly identical code", "back on the proven clone");

      // 4. Dismissing the winner must not promote the demoted family.
      bubble.dismissCluster("c-proven");
      bubble.render(capture.editor, span(18), [shapeOnly, proven]);
      assert.equal(
        capture.visible(),
        undefined,
        "dismissing the only act-now cluster must clear, not fall back to a demoted family",
      );
      assert.ok(
        capture.history().every((text) => !text.includes(SHAPE_ONLY_TITLE)),
        `the demoted family must never have been rendered: ${capture.history().join(" | ")}`,
      );
    } finally {
      bubble.dispose();
    }
  });

  test("the report's fused confidence overrides the probe's for the same cluster id", async () => {
    // [VSIX-LIVE-BUBBLE] The probe is a transient LSP query; the report
    // snapshot is authoritative. A stale probe must not be able to talk a
    // demoted cluster onto the live surface.
    const demotedInReport = bubbleCluster("c-x", 50, 0.3, {
      bucket: "structural_only",
      structural: 1,
      token: 0.3,
    });
    const staleProbe = bubbleCluster("c-x", 50, 0.99, { bucket: "identical" });
    const store = new ReportStore();
    store.setSnapshot(reportWithClusters([demotedInReport]), 0);
    await setBubbleMode("inline");
    const capture = capturingEditor();
    const bubble = new LiveBubble(store, () => undefined);

    try {
      // 1. An over-confident probe cannot override a demoted report entry.
      bubble.render(capture.editor, span(0), [staleProbe]);
      assert.equal(
        capture.visible(),
        undefined,
        `the report's fused ${demotedInReport.signals.fused} must beat the probe's ` +
          `${staleProbe.signals.fused}: ${JSON.stringify(capture.calls)}`,
      );
      assert.ok(
        capture.history().every((text) => !text.includes("Identical code")),
        "the probe's optimistic bucket must never have been painted",
      );

      // 2. A confident report entry renders even when the probe is stale and low.
      const confident = bubbleCluster("c-y", 60, 0.95, { bucket: "identical" });
      const lowProbe = bubbleCluster("c-y", 60, 0.1, { bucket: "loosely_similar" });
      store.setSnapshot(reportWithClusters([confident]), 1);
      bubble.render(capture.editor, span(6), [lowProbe]);
      const visible = assertShowing(capture, "Identical code", "with a confident report entry");
      assert.doesNotMatch(visible, /Loosely similar/, "the stale probe's bucket must not surface");
      assert.ok(capture.visibleHover() !== undefined, "the bubble carries its hover card");

      // 3. A rescan demotes that same cluster; the surface must follow it down.
      const demotedLater = bubbleCluster("c-y", 60, 0.3, {
        bucket: "structural_only",
        structural: 1,
        token: 0.3,
      });
      store.setSnapshot(reportWithClusters([demotedLater]), 2);
      bubble.render(capture.editor, span(12), [lowProbe]);
      assert.equal(
        capture.visible(),
        undefined,
        "a cluster demoted by a later snapshot must leave the live surface",
      );
    } finally {
      bubble.dispose();
    }
  });

  // DEFECT C — restored. `signalStrip` drew structural/token/embedding, so
  // a verbatim copy and a proven rename both rendered "█▁█" and the user
  // could not tell "safe to extract" from "identifiers differ". The strip
  // is still three bars — shape, semantic, confidence — because the two
  // shape views are one piece of evidence and only `fused` separates them.
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test("the signal strip distinguishes a proven rename from a verbatim copy", () => {
    // Both render structural 1.0 and token 1.0 — the rename's token
    // signal is corrected upward by the Merkle argument (#232) — so the
    // three-bar strip collapses them. The only thing separating a "safe
    // to extract" copy from renamed code that may differ is the fused
    // confidence, which the strip never draws.
    const verbatim = bubbleCluster("v", 10, 1.0, { bucket: "identical" });
    const rename = bubbleCluster("r", 10, 0.9, { bucket: "nearly_identical" });
    const demoted = bubbleCluster("d", 10, 0.31, {
      bucket: "structural_only",
      structural: 1,
      token: 0.3,
    });

    assert.equal(
      verbatim.signals.structural,
      rename.signals.structural,
      "fixture: shape evidence is identical for both",
    );
    assert.equal(
      verbatim.signals.token_jaccard,
      rename.signals.token_jaccard,
      "fixture: token evidence is identical for both",
    );
    assert.notEqual(
      verbatim.signals.fused,
      rename.signals.fused,
      "fixture: only the fused confidence separates them",
    );
    assert.equal(signalStrip(verbatim).length, 3, "the strip is three bars wide");
    assert.match(signalStrip(demoted), /[▁▂▃▄▅▆▇█]{3}/u, "every bar comes from the ramp");
    assert.notEqual(
      signalStrip(verbatim),
      signalStrip(demoted),
      "a demoted family must at least be distinguishable by its weaker token bar",
    );
    assert.notEqual(
      signalStrip(verbatim),
      signalStrip(rename),
      `the strip must show the confidence that separates a verbatim copy ` +
        `(fused ${verbatim.signals.fused}) from a proven rename ` +
        `(fused ${rename.signals.fused}); both render "${signalStrip(rename)}"`,
    );
  });

  test("the full block is reserved for proof, so 0.96 and 1.00 stay apart", () => {
    // The band the 0.90 fixture above never reached. `bar()` rounded
    // `value * 7`, which handed `█` to everything from ~0.929 up — so the
    // third bar, added precisely to separate proof from near-proof, drew the
    // same glyph for both. These are real rendered values: a scan of two F#
    // modules one identifier apart renders `nearly_identical` at fused 0.956
    // beside `identical` at 1.00.
    const proven = bubbleCluster("p", 10, 1.0, { bucket: "identical" });
    const nearly = bubbleCluster("n", 10, 0.956, { bucket: "nearly_identical" });

    assert.notEqual(
      signalStrip(proven),
      signalStrip(nearly),
      `fused 1.00 and 0.956 must not render the same strip; both drew ` +
        `"${signalStrip(nearly)}" before the top glyph was reserved`,
    );
    assert.ok(
      signalStrip(proven).endsWith("█"),
      "an exact 1.0 earns the full block",
    );
    assert.equal(
      signalStrip(nearly).endsWith("█"),
      false,
      "and anything short of proof must not",
    );

    // The reservation is on the value, not on the bucket: every band below
    // 1.0 has to stay off the top glyph, however close it sits.
    for (const value of [0.929, 0.95, 0.99, 0.999]) {
      const cluster = bubbleCluster("x", 10, value, { bucket: "nearly_identical" });
      assert.equal(
        signalStrip(cluster).endsWith("█"),
        false,
        `fused ${value} is not proof and must not draw the proof glyph`,
      );
    }
  });

  test("a sub-threshold hint bucket stays off the live surface at the exact cutoff", async () => {
    // Below the act-now bands the fused cutoff is the right gate: a weak
    // LSH hint is worth showing only once it clears FUSED_THRESHOLD.
    const atCutoff = bubbleCluster("c-at", 20, FUSED_THRESHOLD, {
      bucket: "loosely_similar",
      structural: 0.4,
      token: 0.9,
    });
    const underCutoff = bubbleCluster("c-under", 20, FUSED_THRESHOLD - 0.01, {
      bucket: "loosely_similar",
      structural: 0.4,
      token: 0.9,
    });
    const store = new ReportStore();
    store.setSnapshot(reportWithClusters([atCutoff, underCutoff]), 0);
    await setBubbleMode("inline");
    const capture = capturingEditor();
    const bubble = new LiveBubble(store, () => undefined);

    try {
      // 1. Just under the cutoff: nothing.
      bubble.render(capture.editor, span(0), [underCutoff]);
      assert.equal(
        capture.visible(),
        undefined,
        `fused ${underCutoff.signals.fused} is under the cutoff and must not bubble`,
      );
      assert.equal(capture.history().length, 0, "nothing may have been painted yet");

      // 2. Exactly at the cutoff: the hint appears, in its own bucket.
      bubble.render(capture.editor, span(6), [atCutoff]);
      const visible = assertShowing(capture, "Loosely similar code", "at the exact cutoff");
      assert.doesNotMatch(visible, /Identical code/, "a weak hint must not borrow an act-now title");
      assert.doesNotMatch(visible, /Nearly identical/, "nor a near-miss title");
      assert.ok(capture.visibleHover() !== undefined, "even a hint carries its hover card");

      // 3. Both offered at once: the one that clears the cutoff wins.
      bubble.render(capture.editor, span(12), [underCutoff, atCutoff]);
      assertShowing(capture, "Loosely similar code", "with both hints offered");

      // 4. Only the sub-threshold hint remains: the surface clears again.
      bubble.render(capture.editor, span(18), [underCutoff]);
      assert.equal(
        capture.visible(),
        undefined,
        "dropping back below the cutoff must clear the hint",
      );
    } finally {
      bubble.dispose();
    }
  });
});
