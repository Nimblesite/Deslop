// Unit: live-bubble admission is the engine's report, nothing else
// ([VSIX-LIVE-BUBBLE]).
//
// There is no second admission path. The old suite staged clusters on
// either side of a UI-restated clone-kind cutoff; that classification is
// gone from the wire and from this client ([REPORTING-CONTEXT]). These
// tests pin what the user must see: a reported cluster always renders —
// however low its mass — the bubble carries cluster facts (mass severity,
// count, canonical) and never pair evidence ([FUSED-PAIR-SIGNALS]).

import * as assert from "node:assert/strict";
import { ghostText, inlineText } from "../../bubble/live";
import {
  BubbleCapture,
  assertBubbleShows,
  bubbleCluster,
  bubbleFixture,
  setBubbleMode,
  span,
} from "./bubble.helpers";
import { reportWithClusters } from "./report.helpers";

// The spec'd short verdict, rendered by every bubble surface.
const DUPLICATION_TITLE = "DUPLICATION";

// Asserts the surface is showing `title` and nothing from the pair
// vocabulary (bars, per-axis values, verdicts).
function assertShowing(
  capture: BubbleCapture,
  title: string,
  context: string,
): string {
  const visible = assertBubbleShows(capture, title, context);
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
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([near]),
    });

    try {
      // 1. The user's cursor lands on the near miss.
      bubble.render(capture.editor, span(0), [near]);
      const visible = assertShowing(capture, DUPLICATION_TITLE, "at cursor land");
      assert.match(visible, /×\s*3/, "bubble renders the occurrence count");
      assert.match(visible, /A\.cs/, "bubble names the canonical file");
      assert.ok(capture.visibleHover() !== undefined, "inline bubble carries a hover card");

      // 2. The user moves the cursor within the same cluster.
      bubble.render(capture.editor, span(6), [near]);
      const moved = assertShowing(capture, DUPLICATION_TITLE, "after cursor move");
      assert.match(moved, /×\s*3/, "the count survives a cursor move");
      assert.ok(capture.visibleHover() !== undefined, "the hover survives a cursor move");

      // 3. The user switches to ghost mode.
      await setBubbleMode("ghost");
      bubble.render(capture.editor, span(12), [near]);
      const ghost = assertShowing(capture, DUPLICATION_TITLE, "in ghost mode");
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
      bubble.render(capture.editor, span(18), [near]);
      assert.equal(
        capture.visible(),
        undefined,
        "a dismissed cluster must not return",
      );
    } finally {
      await setBubbleMode("inline");
      bubble.dispose();
    }
  });

  test("the lowest-mass reported cluster renders, exactly like the worst", async () => {
    // [VSIX-LIVE-BUBBLE] Admission is "the engine reported it". There is
    // no mass cutoff a client may restate: a faint-ranked cluster at the
    // bottom of a large report is still a reported duplicate.
    const faint = bubbleCluster("c-faint", 1, { rank: 9 });
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([faint]),
    });

    try {
      bubble.render(capture.editor, span(0), [faint]);
      const visible = assertShowing(capture, DUPLICATION_TITLE, "faint cluster");
      assert.match(visible, /×\s*2/, "count renders for the faint cluster");
      assert.doesNotMatch(
        visible,
        /evidence|verdict|shape|token|embedding/i,
        "no pair-evidence word may reach the bubble",
      );
      assert.equal(ghostText(faint, "faint").includes("pair"), false, "ghost text carries no pair label");
      assert.equal(inlineText(faint, "faint").includes("pair"), false, "inline text carries no pair label");
    } finally {
      bubble.dispose();
    }
  });

  test("no reported cluster, no bubble", async () => {
    const { capture, bubble } = await bubbleFixture({ snapshot: null });
    try {
      bubble.render(capture.editor, span(0), []);
      assert.equal(capture.visible(), undefined, "an empty probe paints nothing");
      assert.equal(capture.history().length, 0, "no decoration was ever set");
    } finally {
      bubble.dispose();
    }
  });
});
