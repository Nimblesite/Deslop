// Unit: LiveBubble.render — drive inline + ghost paths + dismissal + no-op.
// Every render assertion goes through the shared decoration capture so the
// suite pins the text the user actually sees. Admission is the engine's
// report: a reported cluster renders, whatever its mass ([REPORTING-CONTEXT]).

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { LiveBubble } from "../../bubble/live";
import {
  DEFAULT_BUBBLE_CLUSTER_MASS,
  PRIMARY_BUBBLE_CLUSTER_ID,
  bubbleFixture,
  openLiveDocument,
  probeCluster as cluster,
  probeReport as report,
  renderFullConfidenceBubble,
  retractCluster,
  setBubbleMode as setMode,
  span,
} from "./bubble.helpers";
import { SHORT_VERDICT } from "../../bubble/renderParts";
import { reportWithClusters } from "./report.helpers";

const DISMISSIBLE_CLUSTER_ID = "c-dismiss";
const SHORT_SPAN_LENGTH = 6;
const FIVE_OCCURRENCE_REPORT = 5;

suite("LiveBubble render", () => {
  test("inline mode renders the bubble decoration", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      bubble.render(capture.editor, span(0), [
        cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS),
      ]);
      const visible = capture.visible();

      assert.ok(
        visible !== undefined,
        `a reported cluster renders: ${JSON.stringify(capture.calls)}`,
      );
      assert.match(visible ?? "", /×\s*5/, "count comes from the authoritative report");
      assert.match(visible ?? "", /A\.cs/, "bubble names the canonical file");
      assert.ok(
        capture.visibleHover() !== undefined,
        "an inline bubble must carry its hover card",
      );

      // Idempotent re-render (same cluster + range) must not repaint.
      const before = capture.calls.length;
      bubble.render(capture.editor, span(0), [
        cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS),
      ]);
      assert.equal(
        capture.calls.length,
        before,
        "re-rendering the same cluster at the same range must be a no-op",
      );

      // An empty probe clears the surface.
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), []);
      assert.equal(
        capture.visible(),
        undefined,
        "an empty probe must clear the bubble",
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
      bubble.render(capture.editor, span(0), [cluster(PRIMARY_BUBBLE_CLUSTER_ID, 100, FIVE_OCCURRENCE_REPORT)]);
      const visible = capture.visible() ?? "";

      assert.equal(
        capture.history().length,
        1,
        `expected one inline decoration: ${capture.history().join(", ")}`,
      );
      assert.match(visible, /×\s*5/, "bubble count must match the report snapshot");
      assert.doesNotMatch(
        visible,
        /×\s*100/,
        "bubble count must not use the live probe occurrence total",
      );
      assert.match(visible, /A\.cs/, "bubble keeps the authoritative representative");
    } finally {
      bubble.dispose();
    }
  });

  test("ghost mode renders the ghost-line decoration", async () => {
    const { capture, bubble } = await bubbleFixture({ mode: "ghost" });
    try {
      bubble.render(capture.editor, span(0), [
        cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS),
      ]);
      const ghost = capture.visible() ?? "";

      assert.match(ghost, /└─/, "ghost mode renders the tree-branch prefix");
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
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [
        cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS),
      ]);
      const inline = capture.visible() ?? "";
      assert.doesNotMatch(inline, /└─/, "inline mode drops the ghost prefix");
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
      bubble.render(capture.editor, span(0), [cluster("x", 1)]);

      assert.equal(
        capture.calls.length,
        0,
        "with no snapshot the bubble must not touch the decoration surface at all",
      );
      assert.equal(capture.visible(), undefined, "nothing can be visible without a report");

      // Once a snapshot lands the very same probe renders.
      store.setSnapshot(report(), 0);
      const visible = renderFullConfidenceBubble(capture, bubble, 0, "c-a");
      assert.match(visible, /×\s*5/, "the report's count renders");
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
      assert.match(visible, /×\s*5/, "seeded on a reported cluster");

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

      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [
        cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS),
      ]);
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
    // [VSIX-LIVE-BUBBLE] Both clusters are reported by the snapshot: the
    // gate admits reported clusters only, and dismissal is per-cluster.
    const dismissed = cluster(DISMISSIBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS);
    const survivor = cluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS, 5);
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([dismissed, survivor]),
    });
    try {
      renderFullConfidenceBubble(capture, bubble, 0, DISMISSIBLE_CLUSTER_ID);

      bubble.dismissCluster(DISMISSIBLE_CLUSTER_ID);
      // The dismissedClusters filter drops it before the sort step, so
      // even though the report still carries it, it must not come back.
      bubble.render(capture.editor, span(SHORT_SPAN_LENGTH), [
        cluster(DISMISSIBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS),
      ]);
      assert.equal(
        capture.visible(),
        undefined,
        "a dismissed cluster must stay hidden on re-render",
      );

      // Dismissal is per-cluster, not a global mute: the reported survivor
      // still renders its full inline title and hover card.
      const visible = renderFullConfidenceBubble(capture, bubble, 12, PRIMARY_BUBBLE_CLUSTER_ID);
      assert.match(visible, new RegExp(SHORT_VERDICT), "and the survivor keeps its rendered title");
      assert.match(visible, /×\s*5/, "and the survivor keeps its report count");
    } finally {
      bubble.dispose();
    }
  });

  test("deslop.bubble.dismiss command clears the active bubble", async () => {
    const clearable = cluster("c-clear", DEFAULT_BUBBLE_CLUSTER_MASS);
    const { capture, bubble } = await bubbleFixture({
      snapshot: reportWithClusters([clearable]),
    });
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
      bubble.render(editor, range, [cluster("c-inlay", DEFAULT_BUBBLE_CLUSTER_MASS)]);
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
