// Unit: live-bubble admission is the engine's report, nothing else
// ([VSIX-LIVE-BUBBLE]).
//
// There is no second admission path. The old suite staged clusters on
// either side of a UI-restated clone-kind cutoff; that classification is
// gone from the wire and from this client ([REPORT-CONTEXT-CLUSTER]). These
// tests pin what the user must see: a reported cluster always renders —
// however low its mass — the bubble carries cluster facts (mass severity,
// count, canonical) and never pair evidence ([FUSED-PAIR-SIGNALS]).

import * as assert from "node:assert/strict";
import { ghostText, inlineText } from "../../bubble/live";
import { BubbleCapture, bubbleCluster, renderStep, renderStepOffersNothing, setBubbleMode, withBubble } from "./bubble.helpers";
import type { LiveBubble } from "../../bubble/live";
import type { ReportCluster } from "../../types/report";
import { reportWithClusters } from "./report.helpers";


// One journey step that additionally pins the absence of the pair
// vocabulary (bars, per-axis values, verdicts) from the surface.
function renderWithoutSignalBars(
  capture: BubbleCapture,
  bubble: LiveBubble,
  startChar: number,
  clusters: ReportCluster[],
  context: string,
): string {
  const visible = renderStep(capture, bubble, startChar, clusters, context);
  assert.doesNotMatch(
    visible,
    /[▁▂▃▄▅▆▇█]/u,
    `${context}: no signal bar glyph may ever render`,
  );
  return visible;
}

suite("LiveBubble admission", () => {
  test("a reported near-miss cluster still reaches the bubble", async () => {
    // A genuine near miss reported by the engine: identical shape, real
    // edits, so the engine's own admission stands. The bubble must
    // render every reported cluster — no client-side signal value,
    // however low, may hide it from the flagship live surface.
    const near = bubbleCluster("c-near", 12, { occurrenceTotal: 3 });
    await withBubble({
      snapshot: reportWithClusters([near]),
    }, async ({ capture, bubble }) => {
      // 1. The user's cursor lands on the near miss.
      const visible = renderWithoutSignalBars(capture, bubble, 0, [near], "at cursor land");
      assert.match(visible, /×\s*3/, "bubble renders the occurrence count");
      assert.match(visible, /A\.cs/, "bubble names the canonical file");
      assert.ok(capture.visibleHover() !== undefined, "inline bubble carries a hover card");

      // 2. The user moves the cursor within the same cluster.
      const moved = renderWithoutSignalBars(capture, bubble, 6, [near], "after cursor move");
      assert.match(moved, /×\s*3/, "the count survives a cursor move");
      assert.ok(capture.visibleHover() !== undefined, "the hover survives a cursor move");

      // 3. The user switches to ghost mode.
      await setBubbleMode("ghost");
      const ghost = renderWithoutSignalBars(capture, bubble, 12, [near], "in ghost mode");
      assert.match(ghost, /└─/, "ghost mode renders the tree-branch prefix");
      assert.equal(
        ghost.includes("pair"),
        false,
        "ghost mode must not render a pair label",
      );
      assert.equal(capture.visibleHover(), undefined, "ghost decorations carry no hover");

      // 4. The user dismisses it; the cluster must stay gone.
      await setBubbleMode("inline");
      bubble.dismissCluster("c-near");
      renderStepOffersNothing(
        capture,
        bubble,
        18,
        [near],
        "a dismissed cluster must not return",
      );
    
    });
  });

  test("the lowest-mass reported cluster renders, exactly like the worst", async () => {
    // [VSIX-LIVE-BUBBLE] Admission is "the engine reported it". There is
    // no mass cutoff a client may restate: a faint-ranked cluster at the
    // bottom of a large report is still a reported duplicate.
    const faint = bubbleCluster("c-faint", 1, { rank: 9 });
    await withBubble({
      snapshot: reportWithClusters([faint]),
    }, ({ capture, bubble }) => {
      const visible = renderWithoutSignalBars(capture, bubble, 0, [faint], "faint cluster");
      assert.match(visible, /×\s*2/, "count renders for the faint cluster");
      assert.doesNotMatch(
        visible,
        /evidence|verdict|shape|token|embedding/i,
        "no pair-evidence word may reach the bubble",
      );
      assert.equal(ghostText(faint, "faint").includes("pair"), false, "ghost text carries no pair label");
      assert.equal(inlineText(faint, "faint").includes("pair"), false, "inline text carries no pair label");
    
    });
  });

  test("no reported cluster, no bubble", async () => {
    await withBubble({ snapshot: null }, ({ capture, bubble }) => {
      renderStepOffersNothing(capture, bubble, 0, [], "an empty probe paints nothing");
      assert.equal(capture.history().length, 0, "no decoration was ever set");
    
    });
  });
});
