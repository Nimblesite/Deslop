// Unit: pure helper logic inside embeddingPicker. Runs under vscode-test so
// the transitive `vscode` import resolves against the real host.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import {
  buildItems,
  pickEmbeddingModel,
  isActive,
  setModel,
  setModelFromPicker,
} from "../../commands/embeddingPicker";
import { ReportStore } from "../../reportStore";
import { EmbeddingModelInfo } from "../../types/report";

function newStore(embedding?: {
  provider_id: string;
  model_id: string;
  model_version: string;
  dimensions: number;
}): ReportStore {
  const store = new ReportStore();
  store.setSnapshot(
    {
      tool_version: "x",
      min_nodes: 30,
      files_analysed: 0,
      clusters_hidden: 0,
      cache_stats: { hits: 0, misses: 0 },
      metrics: {
        analysed_loc: 0,
        duplicated_loc: 0,
        duplication_percent: 0,
        clusters_total: 0,
        duplicated_files: 0,
        threshold: { percent: 0, breached: false, source: "none" },
      per_file: [],
      },
      schema_doc: "",
      action_hints: [],
      boilerplate_hints: [],
      embedding_provenance: embedding
        ? {
            attempted_subtrees: 0,
            indexed_subtrees: 0,
            failed_subtrees: 0,
            ...embedding,
          }
        : undefined,
      clusters: [],
    },
    0,
  );
  return store;
}

function model(
  provider_id: string,
  model_id: string,
  overrides: Partial<EmbeddingModelInfo> = {},
): EmbeddingModelInfo {
  return {
    provider_id,
    model_id,
    model_version: "0",
    dimensions: 768,
    recommended: false,
    reachable: true,
    ...overrides,
  };
}

interface FakeQuickPick extends vscode.QuickPick<vscode.QuickPickItem> {
  disposed: boolean;
  hideHandlerCount: number;
  shown: boolean;
  fireHide(): void;
}

function fakeQuickPick(): FakeQuickPick {
  const hideHandlers: Array<() => void> = [];
  const quickPick = {
    activeItems: [] as readonly vscode.QuickPickItem[],
    busy: false,
    disposed: false,
    hideHandlerCount: 0,
    items: [] as readonly vscode.QuickPickItem[],
    selectedItems: [] as readonly vscode.QuickPickItem[],
    shown: false,
    fireHide() {
      for (const handler of hideHandlers) handler();
    },
    hide() {
      quickPick.fireHide();
    },
    dispose() {
      quickPick.disposed = true;
    },
    show() {
      quickPick.shown = true;
    },
    onDidAccept() {
      return { dispose() {} };
    },
    onDidHide(handler: () => void) {
      hideHandlers.push(handler);
      quickPick.hideHandlerCount = hideHandlers.length;
      return { dispose() {} };
    },
  } as unknown as FakeQuickPick;
  return quickPick;
}

function installQuickPick(quickPick: FakeQuickPick): () => void {
  const win = vscode.window as unknown as {
    createQuickPick: typeof vscode.window.createQuickPick;
  };
  const original = win.createQuickPick;
  win.createQuickPick = function createQuickPick<T extends vscode.QuickPickItem>() {
    return quickPick as unknown as vscode.QuickPick<T>;
  };
  return () => {
    win.createQuickPick = original;
  };
}

suite("embeddingPicker helpers", () => {
  test("isActive returns true only when provider + model match", () => {
    const active = { provider_id: "ollama", model_id: "nomic-embed-text" };
    assert.equal(isActive(active, model("ollama", "nomic-embed-text")), true);
    assert.equal(isActive(active, model("ollama", "other")), false);
    assert.equal(isActive(active, model("stub", "nomic-embed-text")), false);
    assert.equal(isActive(null, model("ollama", "nomic-embed-text")), false);
    assert.equal(isActive(active, undefined), false);
  });

  test("buildItems surfaces the 'Ollama not detected' hint when the list is empty", () => {
    const items = buildItems([], newStore());
    assert.ok(items.some((i) => i.label === "Ollama not detected"));
    assert.ok(items.some((i) => i.label?.includes("Embeddings off")));
    assert.ok(items.some((i) => i.label?.includes("Pull a new model")));
    assert.ok(items.some((i) => i.label?.includes("Refresh list")));
    assert.equal(
      items.some((i) => i.label?.includes("stub")),
      false,
      "normal picker must not expose the deterministic test stub",
    );
  });

  test("GH#127 normal picker can disable embeddings and hides deterministic stub", () => {
    const items = buildItems(
      [
        model("ollama", "nomic-embed-text", { recommended: true }),
        model("stub", "stub", { dimensions: 64 }),
      ],
      newStore({
        provider_id: "ollama",
        model_id: "nomic-embed-text",
        model_version: "0",
        dimensions: 768,
      }),
    );
    const text = items.map((item) =>
      `${item.label ?? ""} ${item.description ?? ""} ${item.detail ?? ""}`,
    );
    const offItem = items.find((item) =>
      /turn embeddings off|disable embeddings/i.test(
        `${item.label ?? ""} ${item.description ?? ""} ${item.detail ?? ""}`,
      ),
    );

    assert.deepEqual(
      {
        hasOffItem: Boolean(offItem),
        offItemIsPickable: Boolean(offItem && offItem.entryKind !== "none"),
        hidesStub: !text.some((value) => /\bstub\b|deterministic|CI-friendly/i.test(value)),
      },
      {
        hasOffItem: true,
        offItemIsPickable: true,
        hidesStub: true,
      },
      `normal picker items must expose off and hide test-only stub: ${JSON.stringify(text)}`,
    );
  });

  test("setModel short-circuits when the request throws (covers error branch)", async () => {
    const client = {
      sendRequest: () => Promise.reject(new Error("boom")),
    } as unknown as LanguageClient;
    await setModel(client, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "0",
      dimensions: 768,
      recommended: false,
      reachable: true,
    });
  });

  test("setModel happy path persists the workspace config", async () => {
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    await setModel(client, {
      provider_id: "ollama",
      model_id: "nomic-embed-code",
      model_version: "0",
      dimensions: 768,
      recommended: false,
      reachable: true,
    });
    const cfg = vscode.workspace.getConfiguration("deslop");
    assert.equal(calls.length, 1, `expected one RPC call, got ${JSON.stringify(calls)}`);
    assert.equal(calls[0]?.method, "deslop/embeddingSetModel");
    assert.deepEqual(calls[0]?.params, {
      provider_id: "ollama",
      model_id: "nomic-embed-code",
    });
    assert.equal(cfg.get<string>("embedding.provider"), "ollama");
    assert.equal(cfg.get<string>("embedding.model"), "nomic-embed-code");
    assert.equal(cfg.get<string>("embedding.mode"), "auto");
  });

  test("setModel dispatches deslop/embeddingSetModel with the chosen provider + model", async () => {
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    await setModel(client, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "0",
      dimensions: 768,
      recommended: false,
      reachable: true,
    });
    const swap = calls.find((call) => call.method === "deslop/embeddingSetModel");
    const cfg = vscode.workspace.getConfiguration("deslop");
    assert.equal(calls.length, 1, `setModel must dispatch exactly one RPC: ${JSON.stringify(calls)}`);
    assert.ok(swap, `expected embeddingSetModel request; got ${JSON.stringify(calls)}`);
    assert.deepEqual(swap.params, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
    });
    assert.equal(cfg.get<string>("embedding.provider"), "ollama");
    assert.equal(cfg.get<string>("embedding.model"), "nomic-embed-text");
    assert.equal(cfg.get<string>("embedding.mode"), "auto");
  });

  test("setModelFromPicker marks the store's pending model BEFORE dispatching the RPC", async () => {
    const store = newStore();
    const events: string[] = [];
    const recorded: Array<string | null> = [];
    const calls: Array<{ method: string; params: unknown }> = [];
    store.onDidChange((s) => {
      events.push("change");
      recorded.push(s.pendingEmbeddingModel);
    });
    const client = {
      sendRequest: (method: string, params: unknown) => {
        // Capture the store's pending state at the moment the RPC is issued.
        events.push(`rpc(${store.current.pendingEmbeddingModel ?? "null"})`);
        calls.push({ method, params });
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    await setModelFromPicker(client, store, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "0",
      dimensions: 768,
      recommended: false,
      reachable: true,
    });
    const cfg = vscode.workspace.getConfiguration("deslop");
    assert.equal(calls.length, 1, `expected one RPC call, got ${JSON.stringify(calls)}`);
    assert.equal(calls[0]?.method, "deslop/embeddingSetModel");
    assert.deepEqual(calls[0]?.params, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
    });
    assert.ok(
      events.includes("rpc(nomic-embed-text)"),
      `RPC must fire with pending model already set; got ${JSON.stringify(events)}`,
    );
    assert.deepEqual(
      recorded.filter((value) => value === "nomic-embed-text"),
      ["nomic-embed-text"],
      `pending model must be emitted exactly once before RPC: ${JSON.stringify(recorded)}`,
    );
    assert.equal(
      store.current.pendingEmbeddingModel,
      "nomic-embed-text",
      "pending model stays set until the new report arrives",
    );
    assert.equal(cfg.get<string>("embedding.provider"), "ollama");
    assert.equal(cfg.get<string>("embedding.model"), "nomic-embed-text");
    assert.equal(cfg.get<string>("embedding.mode"), "auto");
  });

  test("buildItems marks the 'Ollama models' header as a non-pickable separator", () => {
    const items = buildItems(
      [model("ollama", "nomic-embed-text"), model("stub", "stub", { dimensions: 64 })],
      newStore(),
    );
    const header = items.find((i) => i.label === "Ollama models");
    assert.ok(header, "header row should exist");
    assert.equal(
      header.kind,
      vscode.QuickPickItemKind.Separator,
      "header must be a Separator so VS Code filters it out of selectedItems",
    );
  });

  test("buildItems groups ollama models + marks the active one", () => {
    const active = {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "0",
      dimensions: 768,
    };
    const items = buildItems(
      [
        model("ollama", "nomic-embed-text", { recommended: true }),
        model("ollama", "unknown-model", { reachable: false }),
        model("stub", "stub", { dimensions: 64 }),
      ],
      newStore(active),
    );
    const nomic = items.find((i) => i.label?.includes("nomic-embed-text"));
    assert.ok(nomic, "nomic entry should exist");
    assert.match(nomic.label, /active/);
    assert.match(nomic.description ?? "", /recommended/);
    // An unreachable model should be labelled 'offline' so the picker
    // surfaces provider-down state to the user.
    const unknown = items.find((i) => i.label?.includes("unknown-model"));
    assert.ok(unknown, "unknown-model entry should exist");
    assert.match(unknown.description ?? "", /offline/);
  });

  test("pickEmbeddingModel reports when the analysis server is absent", async () => {
    await pickEmbeddingModel(newStore(), () => undefined);
  });

  test("pickEmbeddingModel closes the loading picker when model discovery is still pending", async () => {
    const quickPick = fakeQuickPick();
    const restoreQuickPick = installQuickPick(quickPick);
    const client = {
      sendRequest: () => new Promise<EmbeddingModelInfo[]>(() => {
        // Intentionally pending to model an unresponsive provider lookup.
      }),
    } as unknown as LanguageClient;

    try {
      const pendingPicker = pickEmbeddingModel(newStore(), () => client);
      assert.equal(typeof pendingPicker.then, "function");
      await Promise.resolve();
      assert.equal(quickPick.shown, true, "picker must be shown immediately");
      assert.equal(quickPick.busy, true, "picker must show loading while models are queried");
      assert.equal(
        quickPick.hideHandlerCount,
        1,
        "loading picker must register its hide handler before model discovery completes",
      );

      quickPick.fireHide();
      assert.equal(
        quickPick.disposed,
        true,
        "closing the loading picker must dispose it even when model discovery has not returned",
      );
    } finally {
      restoreQuickPick();
    }
  });

  test("buildItems never exposes the deterministic stub row in production", () => {
    // [REMOVE-STUB] Even if the wire payload accidentally carries a
    // stub-provider row, the picker must never surface it to the user.
    const items = buildItems(
      [model("stub", "stub", { dimensions: 64 })],
      newStore({
        provider_id: "stub",
        model_id: "stub",
        model_version: "0",
        dimensions: 64,
      }),
    );
    const stubRow = items.find((item) =>
      /\bstub\b/i.test(`${item.label ?? ""} ${item.description ?? ""} ${item.detail ?? ""}`),
    );
    assert.equal(stubRow, undefined, "picker must hide every stub row from end users");
  });

  test("setModel handles non-Error rejections", async () => {
    const client = {
      sendRequest: () => Promise.reject(new Error("string failure")),
    } as unknown as LanguageClient;
    await setModel(client, model("ollama", "broken-model"));
  });

  test("setModelFromPicker clears pending state after a rejected request", async () => {
    const store = newStore();
    const client = {
      sendRequest: () => Promise.reject(new Error("string failure")),
    } as unknown as LanguageClient;

    await setModelFromPicker(client, store, model("ollama", "broken-model"));
    assert.equal(store.current.pendingEmbeddingModel, null);
  });
});
