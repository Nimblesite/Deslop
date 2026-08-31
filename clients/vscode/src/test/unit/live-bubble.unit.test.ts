// Unit: LiveBubble.render — drive inline + ghost paths + dismissal + no-op.
// Every render assertion goes through the shared decoration capture so the
// suite pins the text the user actually sees, including the engine bucket
// that decided whether the bubble appeared at all.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { LiveBubble } from "../../bubble/live";
import {
  DEFAULT_BUBBLE_CLUSTER_WEIGHT,
  HIGH_PAIR_AGREEMENT,
  PRIMARY_BUBBLE_CLUSTER_ID,
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

const DISMISSIBLE_CLUSTER_ID = "c-dismiss";
const SHORT_SPAN_LENGTH = 6;

suite("LiveBubble render", () => {
  test("inline mode renders the bubble decoration", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      bubble.render(capture.editor, span(0), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
      const visible = capture.visible();

      assert.ok(
        visible !== undefined,
        `an act-now bucket renders at agreement ${HIGH_PAIR_AGREEMENT}`,
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
      bubble.render(capture.editor, span(0), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
      assert.equal(
        capture.calls.length,
        before,
        "re-rendering the same cluster at the same range must be a no-op",
      );

      // A probe whose only cluster is demoted clears the surface.
      //
      // Admission is the bucket and nothing else: the same hint with
      // perfect elected evidence must stay hidden too, or the assertion
      // above would also pass if only weak evidence were banned.
      const weakHint = bubbleCluster("c-low", DEFAULT_BUBBLE_CLUSTER_WEIGHT, 0.2, {
        bucket: "loosely_similar",
        structural: 0.3,
        token: 0.4,
      });
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [weakHint]);
      assert.equal(
        capture.visible(),
        undefined,
        `a ${weakHint.bucket} cluster is demoted and must clear the bubble`,
      );
      bubble.render(capture.editor, span(12), [
        bubbleCluster("c-hint", DEFAULT_BUBBLE_CLUSTER_WEIGHT, 1, {
          bucket: "loosely_similar",
          structural: 1,
          token: 1,
        }),
      ]);
      assert.equal(
        capture.visible(),
        undefined,
        "a demoted bucket stays hidden even at agreement 1.0: signals never admit",
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
      bubble.render(capture.editor, span(0), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, 100, HIGH_PAIR_AGREEMENT, 35)]);
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
      bubble.render(capture.editor, span(0), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
      const ghost = capture.visible() ?? "";

      assert.match(ghost, /└─/, "ghost mode renders the tree-branch prefix");
      assert.match(ghost, /Identical code/, "ghost line carries the bucket title");
      assert.doesNotMatch(
        ghost,
        /[▁▂▃▄▅▆▇█]/u,
        "ghost line renders no signal bar: pair evidence is pair-only ([FUSED-PAIR-SIGNALS])",
      );
      assert.equal(
        ghost.includes("pair"),
        false,
        "ghost line renders no pair label",
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
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
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
      bubble.render(capture.editor, span(0), [cluster("x", 1, HIGH_PAIR_AGREEMENT)]);

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
      renderFullConfidenceBubble(capture, bubble, 0, PRIMARY_BUBBLE_CLUSTER_ID);

      // The bucket is load-bearing: a `loosely_similar` cluster is demoted,
      // and no signal value can admit it.
      const belowCutoff = bubbleCluster("y", 1, 0.5, {
        bucket: "loosely_similar",
        structural: 0.4,
        token: 0.5,
      });
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [belowCutoff]);
      assert.equal(
        capture.visible(),
        undefined,
        `a ${belowCutoff.bucket} cluster must clear the bubble at any agreement`,
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
      assert.match(visible, /Identical code/, "seeded on an act-now bucket");

      retractCluster(store, PRIMARY_BUBBLE_CLUSTER_ID);

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
  test("a stale probe cannot resurrect a cluster the visible report dropped", async () => {
    const { store, capture, bubble } = await bubbleFixture({ generation: 1 });

    try {
      renderFullConfidenceBubble(capture, bubble, 0, PRIMARY_BUBBLE_CLUSTER_ID);

      retractCluster(store, PRIMARY_BUBBLE_CLUSTER_ID);
      assert.equal(capture.visible(), undefined, "the delta must clear the bubble");

      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
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
      renderFullConfidenceBubble(capture, bubble, 0, DISMISSIBLE_CLUSTER_ID);

      bubble.dismissCluster(DISMISSIBLE_CLUSTER_ID);
      // The dismissedClusters filter drops it before the sort step, so
      // even at unchanged confidence it must not come back.
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [cluster(DISMISSIBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
      assert.equal(
        capture.visible(),
        undefined,
        "a dismissed cluster must stay hidden even at agreement 0.95",
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
      renderFullConfidenceBubble(capture, bubble, SHORT_SPAN_LENGTH, "c-clear");
    } finally {
      bubble.dispose();
    }
  });

  test("no inlay hint provider remains after render is populated", async () => {
    const { doc, editor, store } = await openLiveDocument("line one\nline two\n");
    const bubble = new LiveBubble(store, () => undefined);
    try {
      const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
      bubble.render(editor, range, [cluster("c-inlay", DEFAULT_BUBBLE_CLUSTER_WEIGHT, HIGH_PAIR_AGREEMENT)]);
      const hints = await vscode.commands.executeCommand<vscode.InlayHint[]>(
        "vscode.executeInlayHintProvider",
        doc.uri,
        new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 8)),
      );
      assert.ok(Array.isArray(hints), "inlay hint query must return an array");
      assert.equal(
        hints.length,
        0,
        `LiveBubble registers no inlay provider — pair signals render on no editor surface ([FUSED-PAIR-SIGNALS]), got ${JSON.stringify(hints)}`,
      );
    } finally {
      bubble.dispose();
    }
  });

});
