// Unit: multi-step live-surface journeys ([VSIX-LIVE-BUBBLE],
// [FUSION-CONTENT-GATE]).
//
// The per-step suites pin one transition each. These drive a whole
// editing session — cursor moves, rescans, mode switches, dismissals,
// deltas — and assert the full rendered state after every step, because
// the defects that reach users are the ones where each step looks right
// and the sequence does not.

import * as assert from "node:assert/strict";
import { FUSED_THRESHOLD, bucketLabels } from "../../types/report";
import {
  assertBubbleShows,
  bubbleCluster,
  bubbleFixture,
  setBubbleMode,
  span,
} from "./bubble.helpers";
import { repoMetrics, reportWithClusters } from "./report.helpers";

const SHAPE_ONLY_TITLE = bucketLabels("structural_only").plainTitle;

function provenClone(id: string, fused: number) {
  return bubbleCluster(id, 40, fused, {
    bucket: "nearly_identical",
    structural: 1,
    token: 1,
    occurrenceTotal: 4,
  });
}

function shapeFamily(id: string) {
  return bubbleCluster(id, 900, 0.31, {
    bucket: "structural_only",
    structural: 1,
    token: 0.3,
    occurrenceTotal: 9,
  });
}

suite("LiveBubble journeys", () => {
  test("a rescan that changes confidence changes what the live surface offers", async () => {
    const proven = provenClone("c-1", 0.9);
    const family = shapeFamily("c-2");
    const { store, capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([family, proven]),
    });

    try {
      // 1. Cursor on the proven clone: it is offered, in full.
      bubble.render(capture.editor, span(0), [proven]);
      const first = assertBubbleShows(capture, "Nearly identical code", "step 1");
      assert.match(first, /×\s*4/, "step 1: renders the proven clone's occurrence count");
      assert.match(first, /A\.cs/, "step 1: names the canonical file");
      assert.doesNotMatch(first, new RegExp(SHAPE_ONLY_TITLE), "step 1: no demoted title");
      assert.ok(capture.visibleHover() !== undefined, "step 1: carries a hover card");

      // 2. Cursor onto the shape-only family: nothing is offered.
      bubble.render(capture.editor, span(6), [family]);
      assert.equal(capture.visible(), undefined, "step 2: a demoted family offers nothing");
      assert.ok(
        family.signals.fused < FUSED_THRESHOLD,
        "step 2: fixture is genuinely demoted",
      );
      assert.ok(
        capture.history().every((text) => !text.includes(SHAPE_ONLY_TITLE)),
        `step 2: no demoted title was ever painted: ${capture.history().join(" | ")}`,
      );

      // 3. A rescan finds the anchors and promotes that family.
      const promoted = bubbleCluster("c-2", 900, 0.9, {
        bucket: "nearly_identical",
        structural: 1,
        token: 1,
        occurrenceTotal: 9,
      });
      store.setSnapshot(reportWithClusters([promoted, proven]), 1);
      bubble.render(capture.editor, span(12), [promoted]);
      const afterPromote = assertBubbleShows(capture, "Nearly identical code", "step 3");
      assert.match(afterPromote, /×\s*9/, "step 3: the promoted family brings its own count");
      assert.doesNotMatch(
        afterPromote,
        new RegExp(SHAPE_ONLY_TITLE),
        "step 3: the promoted family sheds its demoted title",
      );

      // 4. A later rescan demotes it again; the offer is withdrawn.
      store.setSnapshot(reportWithClusters([family, proven]), 2);
      bubble.render(capture.editor, span(18), [family]);
      assert.equal(
        capture.visible(),
        undefined,
        "step 4: re-demotion must withdraw the live offer",
      );

      // 5. The proven clone is still offered — the churn did not lose it.
      bubble.render(capture.editor, span(24), [proven]);
      const last = assertBubbleShows(capture, "Nearly identical code", "step 5");
      assert.match(last, /×\s*4/, "step 5: the untouched clone keeps its count");
      assert.ok(capture.visibleHover() !== undefined, "step 5: and its hover card");
    } finally {
      bubble.dispose();
    }
  });

  test("mode switching never changes the engine's verdict, only its presentation", async () => {
    const proven = provenClone("c-mode", 0.9);
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([proven]),
    });

    try {
      // 1. Inline: title, count, hover, no ghost furniture.
      bubble.render(capture.editor, span(0), [proven]);
      const inline = assertBubbleShows(capture, "Nearly identical code", "inline");
      assert.doesNotMatch(inline, /└─/, "inline: no ghost prefix");
      assert.match(inline, /×\s*4/, "inline: carries the count");
      assert.ok(capture.visibleHover() !== undefined, "inline: carries a hover card");

      // 2. Ghost: same verdict, different furniture, no hover.
      await setBubbleMode("ghost");
      bubble.render(capture.editor, span(6), [proven]);
      const ghost = assertBubbleShows(capture, "Nearly identical code", "ghost");
      assert.match(ghost, /└─/, "ghost: renders the tree-branch prefix");
      assert.match(ghost, /[▁▂▃▄▅▆▇█]{3}/u, "ghost: renders the three-bar strip");
      assert.match(ghost, /×\s*4/, "ghost: carries the same count");
      assert.equal(capture.visibleHover(), undefined, "ghost: carries no hover card");

      // 3. Back to inline: the verdict is unchanged across the round trip.
      await setBubbleMode("inline");
      bubble.render(capture.editor, span(12), [proven]);
      const back = assertBubbleShows(capture, "Nearly identical code", "back to inline");
      assert.doesNotMatch(back, /└─/, "back to inline: ghost furniture is gone");
      assert.match(back, /×\s*4/, "back to inline: the count survived the round trip");
      assert.ok(capture.visibleHover() !== undefined, "back to inline: hover restored");

      // 4. Dismissing clears whichever surface is current.
      bubble.dismiss();
      assert.equal(capture.visible(), undefined, "dismiss clears the inline surface");

      // 5. Plain dismiss is not sticky — the next probe paints it again.
      bubble.render(capture.editor, span(18), [proven]);
      assertBubbleShows(capture, "Nearly identical code", "after a plain dismiss");
    } finally {
      await setBubbleMode("inline");
      bubble.dispose();
    }
  });

  test("dismissal is per cluster and outlives snapshot churn and deltas", async () => {
    const first = provenClone("c-first", 0.9);
    const second = bubbleCluster("c-second", 30, 0.95, {
      bucket: "identical",
      occurrenceTotal: 2,
    });
    const { store, capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([first, second]),
      generation: 1,
    });

    try {
      // 1. Both clusters are offerable to begin with.
      bubble.render(capture.editor, span(0), [first]);
      assertBubbleShows(capture, "Nearly identical code", "step 1");
      bubble.render(capture.editor, span(6), [second]);
      const secondText = assertBubbleShows(capture, "Identical code", "step 1b");
      assert.match(secondText, /×\s*2/, "step 1b: the second cluster brings its own count");

      // 2. Dismissing the first hides only the first.
      bubble.dismissCluster("c-first");
      bubble.render(capture.editor, span(12), [first]);
      assert.equal(capture.visible(), undefined, "step 2: the dismissed cluster stays hidden");
      bubble.render(capture.editor, span(18), [second]);
      assertBubbleShows(capture, "Identical code", "step 2b");

      // 3. A fresh snapshot must not resurrect a dismissed cluster.
      store.setSnapshot(reportWithClusters([first, second]), 2);
      bubble.render(capture.editor, span(24), [first]);
      assert.equal(
        capture.visible(),
        undefined,
        "step 3: dismissal must outlive a rescan",
      );
      bubble.render(capture.editor, span(30), [second]);
      assertBubbleShows(capture, "Identical code", "step 3b");

      // 4. A delta removing the surviving cluster clears the surface.
      store.applyDelta({
        from_generation: 2,
        to_generation: 3,
        clusters_added: [],
        clusters_removed: ["c-second"],
        clusters_updated: [],
        metrics: repoMetrics({ analysed_loc: 10 }),
        cache_stats: { hits: 0, misses: 0 },
        tool_version: "v3",
      });
      assert.equal(
        capture.visible(),
        undefined,
        "step 4: a removed cluster must clear its bubble",
      );
      assert.ok(
        capture.history().every((text) => !text.includes(SHAPE_ONLY_TITLE)),
        "step 4: no demoted title anywhere in the journey",
      );
    } finally {
      bubble.dispose();
    }
  });
});
