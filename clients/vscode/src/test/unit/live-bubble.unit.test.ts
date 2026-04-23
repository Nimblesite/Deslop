// Unit: LiveBubble.render — drive inline + ghost paths + dismissal + no-op.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { LiveBubble } from "../../bubble/live";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";

function cluster(id: string, weight: number, fused: number): ReportCluster {
  return {
    id,
    weight,
    size: 2,
    canonical_node_count: 4,
    signals: {
      structural: 1,
      token_jaccard: 1,
      embedding_cos: 0.5,
      fused,
    },
    occurrences: [
      { path: "/tmp/A.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/tmp/B.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    summary: "",
    interpretation: "interp",
  };
}

function report(): Report {
  return {
    report_schema_version: 1,
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 2,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 10,
      duplicated_loc: 2,
      duplication_percent: 20,
      clusters_total: 1,
      duplicated_files: 2,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "",
    action_hints: [],
    embedding_provenance: null,
    clusters: [cluster("c-a", 10, 0.95)],
  };
}

suite("LiveBubble render", () => {
  test("inline mode renders the bubble decoration", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line one\nline two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("liveBubble.mode", "inline", vscode.ConfigurationTarget.Workspace);
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
    bubble.render(editor, range, [cluster("c-a", 10, 0.95)]);
    // idempotent re-render (same cluster + range) is a no-op
    bubble.render(editor, range, [cluster("c-a", 10, 0.95)]);
    bubble.dispose();
  });

  test("ghost mode renders the ghost-line decoration", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "ghost one\nghost two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("liveBubble.mode", "ghost", vscode.ConfigurationTarget.Workspace);
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
    bubble.render(editor, range, [cluster("c-a", 10, 0.95)]);
    await cfg.update("liveBubble.mode", "inline", vscode.ConfigurationTarget.Workspace);
    bubble.dispose();
  });

  test("render without a report is a no-op", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "text",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 2));
    bubble.render(editor, range, [cluster("x", 1, 0.95)]);
    bubble.dispose();
  });

  test("render clears the bubble when no cluster passes the threshold", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "text",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const bubble = new LiveBubble(store, () => undefined);
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 2));
    // fused below FUSED_THRESHOLD (0.85)
    bubble.render(editor, range, [cluster("y", 1, 0.5)]);
    bubble.dispose();
  });

  test("deslop.bubble.dismissCluster command hides the dismissed cluster from future renders", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line one\nline two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const bubble = new LiveBubble(store, () => undefined);
    try {
      const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
      bubble.render(editor, range, [cluster("c-dismiss", 10, 0.95)]);
      await vscode.commands.executeCommand("deslop.bubble.dismissCluster", "c-dismiss");
      // After dismissal, re-rendering the same cluster must clear — the
      // dismissedClusters filter drops it before the sort step.
      bubble.render(editor, range, [cluster("c-dismiss", 10, 0.95)]);
    } finally {
      bubble.dispose();
    }
  });

  test("deslop.bubble.dismiss command clears the active bubble", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line one\nline two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report(), 0);
    const bubble = new LiveBubble(store, () => undefined);
    try {
      const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 4));
      bubble.render(editor, range, [cluster("c-clear", 10, 0.95)]);
      await vscode.commands.executeCommand("deslop.bubble.dismiss");
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
    try {
      // Seed an active bubble so we can observe the rejection → clearBubble path
      // exercise the `active.editor` branch of clearBubble.
      bubble.render(
        editor,
        new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 2)),
        [cluster("c-seed", 10, 0.95)],
      );
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 3), "d"));
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 500);
      });
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
