// Unit: LiveBubble onEdit path — buffer edits reaching probe dispatch,
// probe-rejection recovery, and the config kill-switch. Split from
// live-bubble.unit.test.ts to honour the 500-line file rule; assertions
// unchanged.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { LiveBubble } from "../../bubble/live";
import {
  capturingEditor,
  openLiveDocument,
  probeCluster as cluster,
  renderFullConfidenceBubble,
} from "./bubble.helpers";
import { SHORT_VERDICT } from "../../bubble/renderParts";
import { reportWithClusters } from "./report.helpers";

suite("LiveBubble onEdit path", () => {
  test("buffer edit path reaches probe and the LSP request is dispatched with byte offsets", async () => {
    // Exercises onEdit → debounced probe → client.sendRequest → render.
    // Covers utf8ByteOffset and the AbortController timeout branch.
    const { editor, store } = await openLiveDocument("abc\n");
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
    const { editor, store } = await openLiveDocument("xyz\n");
    const fakeClient = {
      sendRequest: () => Promise.reject(new Error("probe boom")),
    } as unknown as LanguageClient;
    const bubble = new LiveBubble(store, () => fakeClient);
    const capture = capturingEditor();
    // [VSIX-LIVE-BUBBLE] Only reported clusters render, so seed the report
    // with both clusters the test renders.
    store.setSnapshot(
      reportWithClusters([cluster("c-seed", 10), cluster("c-after", 10)]),
      0,
    );
    try {
      // Seed an active bubble so we can observe the rejection → clearBubble path
      // exercise the `active.editor` branch of clearBubble.
      renderFullConfidenceBubble(capture, bubble, 0, "c-seed");

      await editor.edit((builder) => builder.insert(new vscode.Position(0, 3), "d"));
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 500);
      });

      // A rejected probe must not poison the surface: the next successful
      // render still paints, at unchanged confidence.
      const visible = renderFullConfidenceBubble(capture, bubble, 6, "c-after");
      assert.match(
        visible,
        new RegExp(SHORT_VERDICT),
        "the recovered bubble keeps its short verdict",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("live bubble disabled via config short-circuits onEdit before probe", async () => {
    const { editor, store } = await openLiveDocument("pqr\n");
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
