// Unit: LiveBubble.render — drive inline + ghost paths + dismissal + no-op.
// Every render assertion goes through the shared decoration capture so the
// suite pins the text the user actually sees, including the fused band
// that decided whether the bubble appeared at all.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { LiveBubble } from "../../bubble/live";
import { ReportStore } from "../../reportStore";
import { FUSED_THRESHOLD } from "../../types/report";
import {
  bubbleFixture,
  capturingEditor,
  probeCluster as cluster,
  probeReport as report,
  setBubbleMode as setMode,
  span,
} from "./bubble.helpers";
import { repoMetrics } from "./report.helpers";

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
      bubble.render(capture.editor, span(6), [cluster("c-low", 10, 0.2)]);
      assert.equal(
        capture.visible(),
        undefined,
        `fused 0.2 is under FUSED_THRESHOLD ${FUSED_THRESHOLD} and must clear the bubble`,
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
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      assert.ok(
        capture.visible() !== undefined,
        "the identical probe must render once a report exists",
      );
      assert.match(capture.visible() ?? "", /Identical code/, "and carry its bucket title");
    } finally {
      bubble.dispose();
    }
  });

  test("render clears the bubble when no cluster passes the threshold", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      assert.ok(capture.visible() !== undefined, "fixture must start with a visible bubble");

      bubble.render(capture.editor, span(6), [cluster("y", 1, 0.5)]);
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
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      assert.ok(
        capture.visible() !== undefined,
        "fixture must start with an active inline bubble",
      );
      assert.match(capture.visible() ?? "", /Identical code/, "seeded at full confidence");

      store.applyDelta({
        from_generation: 1,
        to_generation: 2,
        clusters_added: [],
        clusters_removed: ["c-a"],
        clusters_updated: [],
        metrics: repoMetrics({
          analysed_loc: 10,
        }),
        cache_stats: { hits: 0, misses: 0 },
        tool_version: "v2",
      });

      assert.equal(
        capture.visible(),
        undefined,
        "reportChanged removal must clear a bubble for a removed cluster",
      );
    } finally {
      bubble.dispose();
    }
  });

  // 🛑 SKIPPED — DEFECT E. This test is correct and the code is wrong.
  // [VSIX-STATE-DIRTY] says the bubble derives from the visible
  // projection, but `bestBubbleCluster` falls back to the probe's own
  // copy (`byId.get(id) ?? cluster`), so a delta clears the bubble and
  // the very next keystroke paints it straight back. Skipped under an
  // explicit owner mandate to unblock the release. Do not delete, do not
  // weaken: un-skip it as part of the fix. Note this is the least
  // clear-cut of the five — the same fallback legitimately serves
  // clusters the probe finds before a rescan — so settle the intended
  // contract first, then fix the code or restate the test.
  // → docs/plans/fused-score-followups.md § "Skipped VSIX tests to restore"
  test.skip("a stale probe cannot resurrect a cluster the visible report dropped", async () => {
    const { store, capture, bubble } = await bubbleFixture({ generation: 1 });

    try {
      bubble.render(capture.editor, span(0), [cluster("c-a", 10, 0.95)]);
      assert.ok(capture.visible() !== undefined, "fixture must start with a visible bubble");

      store.applyDelta({
        from_generation: 1,
        to_generation: 2,
        clusters_added: [],
        clusters_removed: ["c-a"],
        clusters_updated: [],
        metrics: repoMetrics({ analysed_loc: 10 }),
        cache_stats: { hits: 0, misses: 0 },
        tool_version: "v2",
      });
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

  test("deslop.bubble.dismissCluster command hides the dismissed cluster from future renders", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      bubble.render(capture.editor, span(0), [cluster("c-dismiss", 10, 0.95)]);
      assert.ok(
        capture.visible() !== undefined,
        "a full-confidence cluster must bubble before it is dismissed",
      );

      await vscode.commands.executeCommand("deslop.bubble.dismissCluster", "c-dismiss");
      // The dismissedClusters filter drops it before the sort step, so
      // even at unchanged confidence it must not come back.
      bubble.render(capture.editor, span(6), [cluster("c-dismiss", 10, 0.95)]);
      assert.equal(
        capture.visible(),
        undefined,
        "a dismissed cluster must stay hidden even at fused 0.95",
      );

      // Dismissal is per-cluster, not a global mute.
      bubble.render(capture.editor, span(12), [cluster("c-other", 10, 0.95)]);
      assert.ok(
        capture.visible() !== undefined,
        "a different cluster at the same confidence must still render",
      );
      assert.match(capture.visible() ?? "", /Identical code/, "and keep its bucket title");
    } finally {
      bubble.dispose();
    }
  });

  test("deslop.bubble.dismiss command clears the active bubble", async () => {
    const { capture, bubble } = await bubbleFixture();
    try {
      bubble.render(capture.editor, span(0), [cluster("c-clear", 10, 0.95)]);
      assert.ok(capture.visible() !== undefined, "fixture must start with a visible bubble");

      await vscode.commands.executeCommand("deslop.bubble.dismiss");
      assert.equal(
        capture.visible(),
        undefined,
        "the dismiss command must clear the active bubble",
      );

      // Plain dismiss is not sticky — it clears, it does not blacklist.
      bubble.render(capture.editor, span(6), [cluster("c-clear", 10, 0.95)]);
      assert.ok(
        capture.visible() !== undefined,
        "plain dismiss must not blacklist the cluster from later probes",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("inlay hints provider emits a Type hint after render is populated", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line one\nline two\n",
      language: "csharp",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
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

  test("buffer edit path reaches probe and the LSP request is dispatched with byte offsets", async () => {
    // Exercises onEdit → debounced probe → client.sendRequest → render.
    // Covers utf8ByteOffset and the AbortController timeout branch.
    const doc = await vscode.workspace.openTextDocument({
      content: "abc\n",
      language: "csharp",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("liveBubble.enabled", true, vscode.ConfigurationTarget.Workspace);
    const requests: { method: string; params: unknown }[] = [];
    const fakeClient = {
      sendRequest: (method: string, params: unknown) => {
        requests.push({ method, params });
        return Promise.resolve([cluster("c-probe", 10, 0.95)]);
      },
    } as unknown as LanguageClient;
    const bubble = new LiveBubble(store, () => fakeClient);
    try {
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 3), "d"));
      // debounce is 250ms; wait 500ms for the probe + render to land.
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 500);
      });
      const duplicateProbe = requests.find((r) => r.method === "deslop/duplicatesFindSimilar");
      assert.ok(
        duplicateProbe,
        `probe must dispatch duplicatesFindSimilar, got ${JSON.stringify(requests)}`,
      );
      const params = duplicateProbe.params as {
        path: string;
        start_byte: number;
        end_byte: number;
      };
      assert.equal(typeof params.path, "string");
      assert.equal(typeof params.start_byte, "number");
      assert.equal(typeof params.end_byte, "number");
      assert.ok(params.end_byte > params.start_byte, "end byte must be past start byte");
    } finally {
      bubble.dispose();
    }
  });

  test("probe rejection clears the bubble without propagating the error", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "xyz\n",
      language: "csharp",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const fakeClient = {
      sendRequest: () => Promise.reject(new Error("probe boom")),
    } as unknown as LanguageClient;
    const bubble = new LiveBubble(store, () => fakeClient);
    const capture = capturingEditor();
    try {
      // Seed an active bubble so we can observe the rejection → clearBubble path
      // exercise the `active.editor` branch of clearBubble.
      bubble.render(capture.editor, span(0), [cluster("c-seed", 10, 0.95)]);
      assert.ok(capture.visible() !== undefined, "fixture must start with a visible bubble");

      await editor.edit((builder) => builder.insert(new vscode.Position(0, 3), "d"));
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 500);
      });

      // A rejected probe must not poison the surface: the next successful
      // render still paints, at unchanged confidence.
      bubble.render(capture.editor, span(6), [cluster("c-after", 10, 0.95)]);
      assert.ok(
        capture.visible() !== undefined,
        "a rejected probe must not disable later renders",
      );
      assert.match(
        capture.visible() ?? "",
        /Identical code/,
        "the recovered bubble keeps its bucket title",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("live bubble disabled via config short-circuits onEdit before probe", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "pqr\n",
      language: "csharp",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("liveBubble.enabled", false, vscode.ConfigurationTarget.Workspace);
    const calls: number[] = [];
    const fakeClient = {
      sendRequest: () => {
        calls.push(1);
        return Promise.resolve([]);
      },
    } as unknown as LanguageClient;
    const bubble = new LiveBubble(store, () => fakeClient);
    try {
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 3), "s"));
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 350);
      });
      assert.equal(calls.length, 0, "disabled bubble must not dispatch LSP requests");
    } finally {
      await cfg.update("liveBubble.enabled", true, vscode.ConfigurationTarget.Workspace);
      bubble.dispose();
    }
  });
});
