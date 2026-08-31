// Unit: live-bubble admission is the engine's bucket, nothing else
// ([FUSED-CONTENT-GATE], [VSIX-LIVE-BUBBLE]).
//
// There is no second admission path. The old suite staged clusters on
// either side of a UI-restated fused cutoff; that gate is gone from the
// wire and from this client. These tests pin what the user must see:
// an act-now bucket always renders, a demoted bucket never renders —
// however high its signals — and no surface renders pair evidence,
// which is pair-only ([FUSED-PAIR-SIGNALS]).

import * as assert from "node:assert/strict";
import { ghostText, inlineText } from "../../bubble/live";
import { bucketLabels } from "../../types/report";
import {
  BubbleCapture,
  assertBubbleShows,
  bubbleCluster,
  bubbleFixture,
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

suite("LiveBubble admission", () => {
  test("an act-now near miss with weak content evidence still reaches the bubble", async () => {
    // A genuine Type-3 near miss: identical shape, real edits, so the
    // pair's byte agreement is ~0.8 while the engine still
    // routes `nearly_identical`. The bubble must follow the engine's
    // routing — no signal value, however low, may hide an act-now
    // cluster from the flagship live surface.
    const near = bubbleCluster("c-near", 40, 0.8, {
      bucket: "nearly_identical",
      structural: 1,
      token: 1,
      occurrenceTotal: 3,
    });
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([near]),
    });

    try {
      // 1. The user's cursor lands on the near miss.
      bubble.render(capture.editor, span(0), [near]);
      const visible = capture.visible();
      assert.ok(
        visible !== undefined,
        `an act-now ${near.bucket} cluster must bubble at agreement ` +
          `${near.signals.pair_agreement}: ${JSON.stringify(capture.calls)}`,
      );
      assert.ok(
        near.signals.pair_agreement < 1,
        "fixture must carry weak-but-real content evidence or it proves nothing",
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
      assert.equal(
        ghost.includes("pair"),
        false,
        "ghost mode must not render a pair label",
      );
      assert.doesNotMatch(
        ghost,
        /[▁▂▃▄▅▆▇█]/u,
        "ghost mode must not render any signal bar glyph",
      );
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
    // family outweighs a small proven clone, but the engine demoted it,
    // so it must never be the thing the user is nudged to act on.
    const shapeOnly = bubbleCluster("c-shape", 900, 0.16, {
      bucket: "structural_only",
      structural: 1,
      token: 0.3,
    });
    const proven = bubbleCluster("c-proven", 40, 0.9, {
      bucket: "nearly_identical",
      structural: 1,
      token: 1,
    });
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([shapeOnly, proven]),
    });

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
        `a demoted family alone must show nothing however heavy it is`,
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

  test("the report's bucket overrides the probe's for the same cluster id", async () => {
    // [VSIX-LIVE-BUBBLE] The probe is a transient LSP query; the report
    // snapshot is authoritative. A stale probe must not be able to talk a
    // demoted cluster onto the live surface.
    const demotedInReport = bubbleCluster("c-x", 50, 0.16, {
      bucket: "structural_only",
      structural: 1,
      token: 0.3,
    });
    const staleProbe = bubbleCluster("c-x", 50, 1, { bucket: "identical" });
    const { store, capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([demotedInReport]),
    });

    try {
      // 1. An over-confident probe cannot override a demoted report entry.
      bubble.render(capture.editor, span(0), [staleProbe]);
      assert.equal(
        capture.visible(),
        undefined,
        `the report's ${demotedInReport.bucket} bucket must beat the probe's ` +
          `${staleProbe.bucket}: ${JSON.stringify(capture.calls)}`,
      );
      assert.ok(
        capture.history().every((text) => !text.includes("Identical code")),
        "the probe's optimistic bucket must never have been painted",
      );

      // 2. A report entry routed act-now renders even when the probe is stale
      //    and carries a demoted bucket.
      const confident = bubbleCluster("c-y", 60, 0.95, { bucket: "identical" });
      const lowProbe = bubbleCluster("c-y", 60, 0.1, { bucket: "loosely_similar" });
      store.setSnapshot(reportWithClusters([confident]), 1);
      bubble.render(capture.editor, span(6), [lowProbe]);
      const visible = assertShowing(capture, "Identical code", "with a confident report entry");
      assert.doesNotMatch(visible, /Loosely similar/, "the stale probe's bucket must not surface");
      assert.ok(capture.visibleHover() !== undefined, "the bubble carries its hover card");

      // 3. A rescan demotes that same cluster; the surface must follow it down.
      const demotedLater = bubbleCluster("c-y", 60, 0.16, {
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

  test("no bubble surface renders pair evidence, even where it would separate clones", () => {
    // Both render structural 1.0 and token 1.0 — the rename's token
    // signal is corrected upward by the Merkle argument (#232) — so the
    // former strip's shape and semantic bars collapsed them, and only
    // the pair agreement bar told a "safe to extract" copy from renamed
    // code whose identifiers differ. That bar drew pair evidence on a
    // cluster surface, which is illegal ([FUSED-PAIR-SIGNALS]): the
    // engine's bucket is now the only thing that tells them apart, and
    // no surface may render the pair's values.
    const verbatim = bubbleCluster("v", 10, 1.0, { bucket: "identical" });
    const rename = bubbleCluster("r", 10, 0.9, { bucket: "nearly_identical" });
    const demoted = bubbleCluster("d", 10, 0.16, {
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
      verbatim.signals.pair_agreement,
      rename.signals.pair_agreement,
      "fixture: the pair's content evidence differs, yet must not render",
    );

    for (const [context, cluster] of [
      ["verbatim clone", verbatim],
      ["proven rename", rename],
      ["demoted family", demoted],
    ] as const) {
      const inline = inlineText(cluster, "top10");
      const ghost = ghostText(cluster, "top10");
      assert.equal(inline.includes("pair"), false, `${context}: inline line`);
      assert.equal(ghost.includes("pair"), false, `${context}: ghost line`);
      assert.doesNotMatch(
        ghost,
        /[▁▂▃▄▅▆▇█]/u,
        `${context}: ghost line renders no bar glyph`,
      );
      assert.equal(
        ghost.includes("↔"),
        false,
        `${context}: ghost line renders no pair separator`,
      );
    }

    // What does separate the two proven clones is the engine's own
    // bucket title, which is exactly what the bubble renders.
    assert.match(
      inlineText(verbatim, "top10"),
      new RegExp(bucketLabels("identical").plainTitle),
    );
    assert.match(
      inlineText(rename, "top10"),
      new RegExp(bucketLabels("nearly_identical").plainTitle),
    );
  });

  test("no signal value renders a pair bar at any agreement", () => {
    // The former bar ramp rendered pair evidence and reserved `█` for an
    // exact 1.0. Pair evidence is gone from every cluster surface, so no
    // agreement value — proof or near-proof — may draw any glyph.
    for (const value of [1.0, 0.999, 0.99, 0.956, 0.95, 0.929, 0.5, 0.1]) {
      const cluster = bubbleCluster("x", 10, value, { bucket: "nearly_identical" });
      const ghost = ghostText(cluster, "top10");
      assert.doesNotMatch(
        ghost,
        /[▁▂▃▄▅▆▇█]/u,
        `agreement ${value} must not render any bar glyph`,
      );
      assert.equal(
        ghost.includes("pair"),
        false,
        `agreement ${value} must not render a pair label`,
      );
    }
  });

  test("a hint bucket stays off the live surface at every signal value", async () => {
    // Admission reads the bucket and nothing else. The old suite let a
    // `loosely_similar` hint through once its confidence cleared a
    // cutoff; there is no cutoff anymore, so the hint stays hidden even
    // carrying perfect pair evidence — if the engine wanted it shown
    // it would have routed it act-now.
    const perfectHint = bubbleCluster("c-hint", 20, 1.0, {
      bucket: "loosely_similar",
      structural: 1,
      token: 1,
    });
    const weakHint = bubbleCluster("c-under", 20, 0.1, {
      bucket: "loosely_similar",
      structural: 0.4,
      token: 0.9,
    });
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([perfectHint, weakHint]),
    });

    try {
      // 1. The weak hint: nothing.
      bubble.render(capture.editor, span(0), [weakHint]);
      assert.equal(
        capture.visible(),
        undefined,
        `a ${weakHint.bucket} cluster must not bubble at any agreement`,
      );
      assert.equal(capture.history().length, 0, "nothing may have been painted yet");

      // 2. The perfect-evidence hint: still nothing — signals do not admit.
      bubble.render(capture.editor, span(6), [perfectHint]);
      assert.equal(
        capture.visible(),
        undefined,
        `agreement ${perfectHint.signals.pair_agreement} must not admit a ` +
          `${perfectHint.bucket} cluster: only the engine's bucket admits`,
      );
      assert.doesNotMatch(
        capture.history().join(" | "),
        /Loosely similar/,
        "a hint bucket title must never be painted",
      );

      // 3. Both offered at once: the surface stays clear.
      bubble.render(capture.editor, span(12), [weakHint, perfectHint]);
      assert.equal(
        capture.visible(),
        undefined,
        "two demoted clusters must leave the live surface clear",
      );
    } finally {
      bubble.dispose();
    }
  });
});
