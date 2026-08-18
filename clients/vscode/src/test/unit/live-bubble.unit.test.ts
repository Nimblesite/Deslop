// Unit: LiveBubble.render — drive inline + ghost paths + dismissal + no-op.
// Every render assertion goes through the shared decoration capture so the
// suite pins the text the user actually sees, including the fused band
// that decided whether the bubble appeared at all.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { LiveBubble } from "../../bubble/live";
import { FUSED_THRESHOLD } from "../../types/report";
import {
  bubbleCluster,
  bubbleFixture,
  openLiveDocument,
  probeCluster as cluster,
  probeReport as report,
  renderFullConfidenceBubble,
  retractCluster,
  setBubbleMode as setMode,
  span,
} from "./bubble.helpers";

suite("LiveBubble render", () => {
  test("inline mode renders the bubble decoration", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      const visible = capture.visible();

      assert.ok(
        visible !== undefined,
        `fused 0.95 clears FUSED_THRESHOLD ${FUSED_THRESHOLD} and must render`,
      );
      assert.match(visible ?? "", /Identical code/, "bubble carries the wire bucket title");
      assert.match(visible ?? "", /×\s*5/, "count comes from the authoritative report");
      assert.match(visible ?? "", /A\.cs/, "bubble names the canonical file");
      assert.ok(
        capture.visibleHover() !== undefined,
        "an inline bubble must carry its hover card",
      );

      // Idempotent re-render (same cluster + range) must not repaint.
      const before = capture.calls.length;
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      assert.equal(
        capture.calls.length,
        before,
        "re-rendering the same cluster at the same range must be a no-op",
      );

      // A probe whose only cluster sits under the cutoff clears the surface.
      //
      // The bucket is part of the fixture, not decoration. This row used to
      // carry the `identical` default, a pairing the engine cannot produce:
      // `content_gated_signals` returns an `Identical` cluster's signals
      // untouched, and `Identical` requires structural ≥ 0.99 *and*
      // token_jaccard ≥ 0.99, so its `bounded_fused` is ≥ 0.99 by
      // construction — a byte-proven copy never renders 0.2. A weak hint
      // is what the engine actually pairs with a low confidence, and it is
      // the population the cutoff exists to gate.
      const weakHint = bubbleCluster("c-low", 10, 0.2, {
        bucket: "loosely_similar",
        structural: 0.3,
        token: 0.4,
      });
      bubble.render(capture.editor, span(6), [weakHint]);
      assert.equal(
        capture.visible(),
        undefined,
        `fused 0.2 is under FUSED_THRESHOLD ${FUSED_THRESHOLD} and must clear the bubble`,
      );
      // …and the gate is the confidence, not the bucket: the same hint at
      // the cutoff comes back. Without this the assertion above would also
      // pass if hints were banned from the surface outright.
      bubble.render(capture.editor, span(12), [
        bubbleCluster("c-hint", 10, FUSED_THRESHOLD, {
          bucket: "loosely_similar",
          structural: 0.3,
          token: 0.4,
        }),
      ]);
      assert.ok(
        capture.visible() !== undefined,
        `a hint at exactly ${FUSED_THRESHOLD} clears the cutoff and must render`,
      );
    } finally {
      bubble.dispose();
    }
  });

  test("inline render uses the authoritative report occurrence count for a probe hit", async () => {
    // [VSIX-LIVE-BUBBLE] Issue #26: probe results can be a filtered or
    // broader query shape, but every user-facing surface for the same
    // cluster id must render the occurrence set from the current report.
    const { capture, bubble } = await bubbleFixture();

    try {
      bubble.render(capture.editor, span(0), [cluster("c-a", 100, 0.95, 35)]);
      const visible = capture.visible() ?? "";

      assert.equal(
        capture.history().length,
        1,
        `expected one inline decoration: ${capture.history().join(", ")}`,
      );
      assert.match(visible, /×\s*5/, "bubble count must match the report snapshot");
      assert.doesNotMatch(
        visible,
        /×\s*35/,
        "bubble count must not use the live probe occurrence total",
      );
      assert.match(visible, /A\.cs/, "bubble keeps the authoritative representative");
      assert.match(
        visible,
        /Identical code/,
        "the report's bucket wins over the probe's copy of the cluster",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("ghost mode renders the ghost-line decoration", async () => {
    const { capture, bubble } = await bubbleFixture({ mode: "ghost" });
    try {
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      const ghost = capture.visible() ?? "";

      assert.match(ghost, /└─/, "ghost mode renders the tree-branch prefix");
      assert.match(ghost, /Identical code/, "ghost line carries the bucket title");
      assert.match(
        ghost,
        /[▁▂▃▄▅▆▇█]{3}/u,
        "ghost line carries the three-bar signal strip",
      );
      assert.match(ghost, /×\s*5/, "ghost line carries the occurrence count");
      assert.equal(
        capture.visibleHover(),
        undefined,
        "ghost decorations are pure-visual and carry no hover card",
      );

      // Switching mode mid-session must move the same cluster to the
      // other surface rather than leaving both painted.
      await setMode("inline");
      bubble.render(capture.editor, span(6), [cluster("c-a", 10, 0.95)]);
      const inline = capture.visible() ?? "";
      assert.doesNotMatch(inline, /└─/, "inline mode drops the ghost prefix");
      assert.match(inline, /Identical code/, "the bucket title survives the mode switch");
      assert.ok(
        capture.visibleHover() !== undefined,
        "the inline surface restores the hover card",
      );
    } finally {
      await setMode("inline");
      bubble.dispose();
    }
  });

  test("render without a report is a no-op", async () => {
    const { store, capture, bubble } = await bubbleFixture({ snapshot: null });
    try {
      bubble.render(capture.editor, span(0), [cluster("x", 1, 0.95)]);

      assert.equal(
        capture.calls.length,
        0,
        "with no snapshot the bubble must not touch the decoration surface at all",
      );
      assert.equal(capture.visible(), undefined, "nothing can be visible without a report");

      // Once a snapshot lands the very same probe renders.
      store.setSnapshot(report(), 0);
      const visible = renderFullConfidenceBubble(capture, bubble, 0, "c-a");
      assert.match(visible, /Identical code/, "and carry its bucket title");
    } finally {
      bubble.dispose();
    }
  });

  test("render clears the bubble when no cluster passes the threshold", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      renderFullConfidenceBubble(capture, bubble, 0, "c-a");

      // The bucket is load-bearing: an `identical` cluster is byte-proven and
      // its `bounded_fused` is ≥ 0.99 by construction, so the engine cannot
      // hand this surface an act-now bucket at 0.5. A weak hint is the
      // population the cutoff governs.
      const belowCutoff = bubbleCluster("y", 1, 0.5, {
        bucket: "loosely_similar",
        structural: 0.4,
        token: 0.5,
      });
      bubble.render(capture.editor, span(6), [belowCutoff]);
      assert.ok(0.5 < FUSED_THRESHOLD, "fixture must sit below the cutoff to prove anything");
      assert.equal(
        capture.visible(),
        undefined,
        `fused 0.5 under FUSED_THRESHOLD ${FUSED_THRESHOLD} must clear the bubble`,
      );

      // An empty probe keeps the surface clear rather than restoring the
      // previous winner.
      bubble.render(capture.editor, span(12), []);
      assert.equal(
        capture.visible(),
        undefined,
        "an empty probe must leave the surface clear",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("store delta removing the active cluster clears the bubble", async () => {
    // [VSIX-LIVE-BUBBLE] A removed cluster must clear its bubble immediately
    // on the delta — the bubble must never outlive the cluster in the report.
    const { store, capture, bubble } = await bubbleFixture({ generation: 1 });

    try {
      const visible = renderFullConfidenceBubble(capture, bubble, 0, "c-a");
      assert.match(visible, /Identical code/, "seeded at full confidence");

      retractCluster(store, "c-a");

      assert.equal(
        capture.visible(),
        undefined,
        "reportChanged removal must clear a bubble for a removed cluster",
      );
    } finally {
      bubble.dispose();
    }
  });

  // DEFECT E — restored, with the contract settled first. The
  // `byId.get(id) ?? cluster` fallback served two populations that
  // `bestBubbleCluster` could not tell apart, and each has a test in this
  // file: a cluster the report has **never seen** may bubble on the
  // probe's own evidence (`deslop.bubble.dismissCluster …` below renders
  // `c-dismiss`, absent from the seeded snapshot, and requires it to
  // show), while a cluster a delta **explicitly retracted** must stay
  // gone. Absence from `report.clusters` cannot separate them, so
  // `ReportStore` now records `clusters_removed` instead of dropping it —
  // the discriminator is retraction, not absence ([VSIX-STATE-DIRTY]).
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test("a stale probe cannot resurrect a cluster the visible report dropped", async () => {
    const { store, capture, bubble } = await bubbleFixture({ generation: 1 });

    try {
      renderFullConfidenceBubble(capture, bubble, 0, "c-a");

      retractCluster(store, "c-a");
      assert.equal(capture.visible(), undefined, "the delta must clear the bubble");

      bubble.render(capture.editor, span(6), [cluster("c-a", 10, 0.95)]);
      assert.equal(
        capture.visible(),
        undefined,
        "a stale probe must not resurrect a cluster the visible report dropped",
      );
    } finally {
      bubble.dispose();
    }
  });

  // The full stale-probe races — a superseded probe rejecting after a newer
  // one rendered, a probe resolving after a newer snapshot, generation ABA —
  // are driven through the real async `probe()` path with deferred responses
  // in live-bubble-race.unit.test.ts (RA-05).
  test("a probe is also discarded when its document moves under it", async () => {
    // The store revision is not the only thing the answer was scoped to: a
    // `findSimilar` reply describes byte offsets in one version of one file.
    const { store, capture, bubble } = await bubbleFixture({ generation: 1 });
    try {
      const base = {
        revision: store.current.revision,
        uri: capture.editor.document.uri.toString(),
        version: capture.editor.document.version,
      };
      assert.equal(bubble.hasMovedOn(capture.editor.document, base), false, "the baseline must be live");
      assert.ok(
        bubble.hasMovedOn(capture.editor.document, { ...base, version: base.version + 1 }),
        "a later document version invalidates the byte offsets the reply carries",
      );
      assert.ok(
        bubble.hasMovedOn(capture.editor.document, { ...base, uri: "file:///tmp/somewhere-else.cs" }),
        "and a reply for another document must never paint this one",
      );
      assert.ok(
        bubble.hasMovedOn(capture.editor.document, { ...base, revision: base.revision - 1 }),
        "an answer captured at an older store revision describes a dead world",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("deslop.bubble.dismissCluster command hides the dismissed cluster from future renders", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      renderFullConfidenceBubble(capture, bubble, 0, "c-dismiss");

      bubble.dismissCluster("c-dismiss");
      // The dismissedClusters filter drops it before the sort step, so
      // even at unchanged confidence it must not come back.
      bubble.render(capture.editor, span(6), [cluster("c-dismiss", 10, 0.95)]);
      assert.equal(
        capture.visible(),
        undefined,
        "a dismissed cluster must stay hidden even at fused 0.95",
      );

      // Dismissal is per-cluster, not a global mute.
      const visible = renderFullConfidenceBubble(capture, bubble, 12, "c-other");
      assert.match(visible, /Identical code/, "and keep its bucket title");
    } finally {
      bubble.dispose();
    }
  });

  test("deslop.bubble.dismiss command clears the active bubble", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      renderFullConfidenceBubble(capture, bubble, 0, "c-clear");

      bubble.dismiss();
      assert.equal(
        capture.visible(),
        undefined,
        "the dismiss command must clear the active bubble",
      );

      // Plain dismiss is not sticky — it clears, it does not blacklist.
      renderFullConfidenceBubble(capture, bubble, 6, "c-clear");
    } finally {
      bubble.dispose();
    }
  });

  test("inlay hints provider emits a Type hint after render is populated", async () => {
    const { doc, editor, store } = await openLiveDocument("line one\nline two\n");
    const bubble = new LiveBubble(store, () => undefined);
    try {
      const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
      bubble.render(editor, range, [cluster("c-inlay", 10, 0.95)]);
      const hints = await vscode.commands.executeCommand<vscode.InlayHint[]>(
        "vscode.executeInlayHintProvider",
        doc.uri,
        new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 8)),
      );
      assert.ok(Array.isArray(hints), "inlay hint provider must return an array");
      const ours = hints.filter((h) => h.kind === vscode.InlayHintKind.Type);
      assert.ok(
        ours.length >= 1,
        `expected at least one Type inlay hint from LiveBubble, got ${JSON.stringify(hints)}`,
      );
    } finally {
      bubble.dispose();
    }
  });

});
