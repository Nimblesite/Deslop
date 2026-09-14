// Unit: multi-step live-surface journeys ([VSIX-LIVE-BUBBLE]).
//
// The per-step suites pin one transition each. These drive a whole
// editing session — cursor moves, rescans, mode switches, dismissals,
// deltas — and assert the full rendered state after every step, because
// the defects that reach users are the ones where each step looks right
// and the sequence does not.

import * as assert from "node:assert/strict";
import { bubbleCluster, renderStep, renderStepOffersNothing, setBubbleMode, withBubble } from "./bubble.helpers";
import { repoMetrics, reportWithClusters } from "./report.helpers";


function provenClone(id: string, mass: number) {
  return bubbleCluster(id, mass, { occurrenceTotal: 4 });
}

suite("LiveBubble journeys", () => {
  test("a rescan that changes mass and count changes what the live surface offers", async () => {
    const proven = provenClone("c-1", 40);
    const family = bubbleCluster("c-2", 900, { occurrenceTotal: 9 });
    await withBubble({
      snapshot: reportWithClusters([family, proven]),
    }, ({ store, capture, bubble }) => {
      // 1. Cursor on the proven clone: it is offered, in full.
      const first = renderStep(capture, bubble, 0, [proven], "step 1");
      assert.match(first, /×\s*4/, "step 1: renders the proven clone's occurrence count");
      assert.match(first, /A\.cs/, "step 1: names the canonical file");
      assert.ok(capture.visibleHover() !== undefined, "step 1: carries a hover card");

      // 2. A rescan grows the family: the count on the surface follows.
      const promoted = bubbleCluster("c-2", 900, { occurrenceTotal: 9 });
      store.setSnapshot(reportWithClusters([promoted, proven]), 1);
      const afterPromote = renderStep(capture, bubble, 12, [promoted], "step 2");
      assert.match(afterPromote, /×\s*9/, "step 2: the rescan's count reaches the bubble");

      // 3. A rescan that drops the cluster withdraws the offer.
      renderStepOffersNothing(
        capture,
        bubble,
        18,
        [bubbleCluster("c-3", 1)],
        "step 3: an unreported cluster offers nothing",
      );

      // 4. The proven clone is still offered — the churn did not lose it.
      const last = renderStep(capture, bubble, 24, [proven], "step 4");
      assert.match(last, /×\s*4/, "step 4: the untouched clone keeps its count");
      assert.ok(capture.visibleHover() !== undefined, "step 4: and its hover card");
    
    });
  });

  test("mode switching never changes the engine's verdict, only its presentation", async () => {
    const proven = provenClone("c-mode", 40);
    await withBubble({
      snapshot: reportWithClusters([proven]),
    }, async ({ capture, bubble }) => {
      // 1. Inline: title, count, hover, no ghost furniture.
      const inline = renderStep(capture, bubble, 0, [proven], "inline");
      assert.doesNotMatch(inline, /└─/, "inline: no ghost prefix");
      assert.match(inline, /×\s*4/, "inline: carries the count");
      assert.ok(capture.visibleHover() !== undefined, "inline: carries a hover card");

      // 2. Ghost: same verdict, different furniture, no hover.
      await setBubbleMode("ghost");
      const ghost = renderStep(capture, bubble, 6, [proven], "ghost");
      assert.match(ghost, /└─/, "ghost: renders the tree-branch prefix");
      // [FUSED-PAIR-SIGNALS] Admission signals are pair measurements; a cluster
      // surface never renders the three-bar signal strip.
      assert.doesNotMatch(ghost, /[▁▂▃▄▅▆▇█]/u, "ghost: renders no pair signal strip");
      assert.match(ghost, /×\s*4/, "ghost: carries the same count");
      assert.equal(capture.visibleHover(), undefined, "ghost: carries no hover card");

      // 3. Back to inline: the verdict is unchanged across the round trip.
      await setBubbleMode("inline");
      const back = renderStep(capture, bubble, 12, [proven], "back to inline");
      assert.doesNotMatch(back, /└─/, "back to inline: ghost furniture is gone");
      assert.match(back, /×\s*4/, "back to inline: the count survived the round trip");
      assert.ok(capture.visibleHover() !== undefined, "back to inline: hover restored");

      // 4. Dismissing clears whichever surface is current.
      bubble.dismiss();
      assert.equal(capture.visible(), undefined, "dismiss clears the inline surface");

      // 5. Plain dismiss is not sticky — the next probe paints it again.
      renderStep(capture, bubble, 18, [proven], "after a plain dismiss");
    
    });
  });

  test("dismissal is per cluster and outlives snapshot churn and deltas", async () => {
    const first = provenClone("c-first", 40);
    const second = bubbleCluster("c-second", 30, { occurrenceTotal: 2 });
    await withBubble({
      snapshot: reportWithClusters([first, second]),
      generation: 1,
    }, ({ store, capture, bubble }) => {
      // 1. Both clusters are offerable to begin with.
      renderStep(capture, bubble, 0, [first], "step 1");
      const secondText = renderStep(capture, bubble, 6, [second], "step 1b");
      assert.match(secondText, /×\s*2/, "step 1b: the second cluster brings its own count");

      // 2. Dismissing the first hides only the first.
      bubble.dismissCluster("c-first");
      renderStepOffersNothing(
        capture,
        bubble,
        12,
        [first],
        "step 2: the dismissed cluster stays hidden",
      );
      renderStep(capture, bubble, 18, [second], "step 2b");

      // 3. A fresh snapshot must not resurrect a dismissed cluster.
      store.setSnapshot(reportWithClusters([first, second]), 2);
      renderStepOffersNothing(
        capture,
        bubble,
        24,
        [first],
        "step 3: dismissal must outlive a rescan",
      );
      renderStep(capture, bubble, 30, [second], "step 3b");

      // 4. A delta removing the surviving cluster clears the surface.
      store.applyDelta({
        from_generation: 2,
        to_generation: 3,
        clusters_added: [],
        clusters_removed: ["c-second"],
        clusters_updated: [],
        literal_findings_added: [],
        literal_findings_removed: [],
        literal_findings_updated: [],
        metrics: repoMetrics({ analysed_loc: 10 }),
        cache_stats: { hits: 0, misses: 0 },
        tool_version: "v3",
      });
      assert.equal(
        capture.visible(),
        undefined,
        "step 4: a removed cluster must clear its bubble",
      );
    
    });
  });
});
