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
  turnEmbeddingsOff,
} from "../../commands/embeddingPicker";
import { ReportStore } from "../../reportStore";
import { EmbeddingModelInfo, ReportDelta } from "../../types/report";
import { emptyReport, repoMetrics } from "./report.helpers";

const OLLAMA_PROVIDER_ID = "ollama";
const NOMIC_MODEL_ID = "nomic-embed-text";
const STUB_PROVIDER_ID = "stub";
const DEFAULT_MODEL_VERSION = "0";
const DEFAULT_MODEL_DIMENSIONS = 768;
const STUB_MODEL_DIMENSIONS = 64;
const DESLOP_CONFIG_SECTION = "deslop";
const EMBEDDING_MODE_SETTING = "embedding.mode";
const EMBEDDING_PROVIDER_SETTING = "embedding.provider";
const EMBEDDING_MODEL_SETTING = "embedding.model";
const EMBEDDING_SET_MODEL_METHOD = "deslop/embeddingSetModel";
const EMBEDDING_LIST_MODELS_METHOD = "deslop/embeddingListModels";
const REPORT_DELTA_METHOD = "deslop/reportDelta";
const EMBEDDINGS_OFF_MODE = "off";
const EMBEDDINGS_AUTO_MODE = "auto";
const CODE_MODEL_ID = "nomic-embed-code";
const UNKNOWN_MODEL_ID = "unknown-model";
const BROKEN_MODEL_ID = "broken-model";
const STRING_FAILURE_MESSAGE = "string failure";
const NONE_ENTRY_KIND = "none";
const REFRESH_ENTRY_KIND = "refresh";

function newStore(embedding?: {
  provider_id: string;
  model_id: string;
  model_version: string;
  dimensions: number;
}): ReportStore {
  const store = new ReportStore();
  store.setSnapshot(
    emptyReport({
      tool_version: "x",
      metrics: repoMetrics(),
      embedding_provenance: embedding
        ? {
            attempted_subtrees: 0,
            succeeded_subtrees: 0,
            indexed_subtrees: 0,
            failed_subtrees: 0,
            ...embedding,
          }
        : undefined,
    }),
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
    model_version: DEFAULT_MODEL_VERSION,
    dimensions: DEFAULT_MODEL_DIMENSIONS,
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
  fireAccept(): Promise<void>;
}

function fakeQuickPick(): FakeQuickPick {
  const hideHandlers: Array<() => void> = [];
  const acceptHandlers: Array<() => unknown> = [];
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
    async fireAccept() {
      // Fire only the most recently registered handler so a `refresh`
      // re-entry into pickEmbeddingModel does not re-trigger stale handlers.
      const handler = acceptHandlers[acceptHandlers.length - 1];
      if (handler) await handler();
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
    onDidAccept(handler: () => unknown) {
      acceptHandlers.push(handler);
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
    const active = { provider_id: OLLAMA_PROVIDER_ID, model_id: NOMIC_MODEL_ID };
    assert.equal(isActive(active, model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID)), true);
    assert.equal(isActive(active, model(OLLAMA_PROVIDER_ID, "other")), false);
    assert.equal(isActive(active, model(STUB_PROVIDER_ID, NOMIC_MODEL_ID)), false);
    assert.equal(isActive(null, model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID)), false);
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
        model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID, { recommended: true }),
        model(STUB_PROVIDER_ID, STUB_PROVIDER_ID, { dimensions: STUB_MODEL_DIMENSIONS }),
      ],
      newStore({
        provider_id: OLLAMA_PROVIDER_ID,
        model_id: NOMIC_MODEL_ID,
        model_version: DEFAULT_MODEL_VERSION,
        dimensions: DEFAULT_MODEL_DIMENSIONS,
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
        offItemIsPickable: Boolean(offItem && offItem.entryKind !== NONE_ENTRY_KIND),
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
    await setModel(client, model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID));
  });

  test("setModel happy path persists the workspace config", async () => {
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    await setModel(client, model(OLLAMA_PROVIDER_ID, CODE_MODEL_ID));
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIG_SECTION);
    assert.equal(calls.length, 1, `expected one RPC call, got ${JSON.stringify(calls)}`);
    assert.equal(calls[0]?.method, EMBEDDING_SET_MODEL_METHOD);
    assert.deepEqual(calls[0]?.params, {
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: CODE_MODEL_ID,
    });
    assert.equal(cfg.get<string>(EMBEDDING_PROVIDER_SETTING), OLLAMA_PROVIDER_ID);
    assert.equal(cfg.get<string>(EMBEDDING_MODEL_SETTING), CODE_MODEL_ID);
    assert.equal(cfg.get<string>(EMBEDDING_MODE_SETTING), EMBEDDINGS_AUTO_MODE);
  });

  test("setModel dispatches deslop/embeddingSetModel with the chosen provider + model", async () => {
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    await setModel(client, model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID));
    const swap = calls.find((call) => call.method === EMBEDDING_SET_MODEL_METHOD);
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIG_SECTION);
    assert.equal(calls.length, 1, `setModel must dispatch exactly one RPC: ${JSON.stringify(calls)}`);
    assert.ok(swap, `expected embeddingSetModel request; got ${JSON.stringify(calls)}`);
    assert.deepEqual(swap.params, {
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: NOMIC_MODEL_ID,
    });
    assert.equal(cfg.get<string>(EMBEDDING_PROVIDER_SETTING), OLLAMA_PROVIDER_ID);
    assert.equal(cfg.get<string>(EMBEDDING_MODEL_SETTING), NOMIC_MODEL_ID);
    assert.equal(cfg.get<string>(EMBEDDING_MODE_SETTING), EMBEDDINGS_AUTO_MODE);
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
    await setModelFromPicker(client, store, model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID));
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIG_SECTION);
    assert.equal(calls.length, 1, `expected one RPC call, got ${JSON.stringify(calls)}`);
    assert.equal(calls[0]?.method, EMBEDDING_SET_MODEL_METHOD);
    assert.deepEqual(calls[0]?.params, {
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: NOMIC_MODEL_ID,
    });
    assert.ok(
      events.includes(`rpc(${NOMIC_MODEL_ID})`),
      `RPC must fire with pending model already set; got ${JSON.stringify(events)}`,
    );
    assert.deepEqual(
      recorded.filter((value) => value === NOMIC_MODEL_ID),
      [NOMIC_MODEL_ID],
      `pending model must be emitted exactly once before RPC: ${JSON.stringify(recorded)}`,
    );
    assert.equal(
      store.current.pendingEmbeddingModel,
      NOMIC_MODEL_ID,
      "pending model stays set until the new report arrives",
    );
    assert.equal(cfg.get<string>(EMBEDDING_PROVIDER_SETTING), OLLAMA_PROVIDER_ID);
    assert.equal(cfg.get<string>(EMBEDDING_MODEL_SETTING), NOMIC_MODEL_ID);
    assert.equal(cfg.get<string>(EMBEDDING_MODE_SETTING), EMBEDDINGS_AUTO_MODE);
  });

  test("buildItems marks the 'Ollama models' header as a non-pickable separator", () => {
    const items = buildItems(
      [
        model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID),
        model(STUB_PROVIDER_ID, STUB_PROVIDER_ID, { dimensions: STUB_MODEL_DIMENSIONS }),
      ],
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
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: NOMIC_MODEL_ID,
      model_version: DEFAULT_MODEL_VERSION,
      dimensions: DEFAULT_MODEL_DIMENSIONS,
    };
    const items = buildItems(
      [
        model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID, { recommended: true }),
        model(OLLAMA_PROVIDER_ID, UNKNOWN_MODEL_ID, { reachable: false }),
        model(STUB_PROVIDER_ID, STUB_PROVIDER_ID, { dimensions: STUB_MODEL_DIMENSIONS }),
      ],
      newStore(active),
    );
    const nomic = items.find((i) => i.label?.includes("nomic-embed-text"));
    assert.ok(nomic, "nomic entry should exist");
    assert.match(nomic.label, /active/);
    assert.match(nomic.description ?? "", /recommended/);
    // An unreachable model should be labelled 'offline' so the picker
    // surfaces provider-down state to the user.
    const unknown = items.find((i) => i.label?.includes(UNKNOWN_MODEL_ID));
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

  test("pickEmbeddingModel dispatches on the accepted entry kind", async () => {
    const quickPick = fakeQuickPick();
    const restoreQuickPick = installQuickPick(quickPick);
    const requests: string[] = [];
    const client = {
      sendRequest: (method: string) => {
        requests.push(method);
        if (method === EMBEDDING_LIST_MODELS_METHOD) {
          return Promise.resolve([model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID)]);
        }
        return Promise.resolve(null);
      },
    } as unknown as LanguageClient;
    const accept = (entry: Record<string, unknown>): Promise<void> => {
      quickPick.selectedItems = [entry as unknown as vscode.QuickPickItem];
      return quickPick.fireAccept();
    };

    try {
      const store = newStore();
      await pickEmbeddingModel(store, () => client);
      assert.ok(quickPick.items.length > 0, "picker is populated from the live model list");

      // No selection, and the non-actionable separator, both short-circuit.
      quickPick.selectedItems = [];
      quickPick.activeItems = [];
      await quickPick.fireAccept();
      await accept({ entryKind: NONE_ENTRY_KIND, label: "info row" });

      // Selecting a model switches it through the LSP.
      await accept({
        entryKind: "model",
        label: "m",
        model: model(OLLAMA_PROVIDER_ID, NOMIC_MODEL_ID),
      });
      assert.ok(
        requests.includes(EMBEDDING_SET_MODEL_METHOD),
        "model selection switches the model",
      );

      // The off row turns embeddings off and asks for the post-switch delta.
      await accept({ entryKind: EMBEDDINGS_OFF_MODE, label: EMBEDDINGS_OFF_MODE });
      assert.ok(
        requests.includes(REPORT_DELTA_METHOD),
        "the off path requests the post-switch delta",
      );

      // Refresh re-enters the picker (covers the recursion branch).
      await accept({ entryKind: REFRESH_ENTRY_KIND, label: REFRESH_ENTRY_KIND });
    } finally {
      restoreQuickPick();
      const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIG_SECTION);
      await cfg.update(EMBEDDING_MODE_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
      await cfg.update(EMBEDDING_PROVIDER_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
      await cfg.update(EMBEDDING_MODEL_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
    }
  });

  test("buildItems never exposes the deterministic stub row in production", () => {
    // [REMOVE-STUB] Even if the wire payload accidentally carries a
    // stub-provider row, the picker must never surface it to the user.
    const items = buildItems(
      [model(STUB_PROVIDER_ID, STUB_PROVIDER_ID, { dimensions: STUB_MODEL_DIMENSIONS })],
      newStore({
        provider_id: STUB_PROVIDER_ID,
        model_id: STUB_PROVIDER_ID,
        model_version: DEFAULT_MODEL_VERSION,
        dimensions: STUB_MODEL_DIMENSIONS,
      }),
    );
    const stubRow = items.find((item) =>
      /\bstub\b/i.test(`${item.label ?? ""} ${item.description ?? ""} ${item.detail ?? ""}`),
    );
    assert.equal(stubRow, undefined, "picker must hide every stub row from end users");
  });

  test("setModel handles non-Error rejections", async () => {
    const client = {
      sendRequest: () => Promise.reject(new Error(STRING_FAILURE_MESSAGE)),
    } as unknown as LanguageClient;
    await setModel(client, model(OLLAMA_PROVIDER_ID, BROKEN_MODEL_ID));
  });

  test("setModelFromPicker clears pending state after a rejected request", async () => {
    const store = newStore();
    const client = {
      sendRequest: () => Promise.reject(new Error(STRING_FAILURE_MESSAGE)),
    } as unknown as LanguageClient;

    await setModelFromPicker(client, store, model(OLLAMA_PROVIDER_ID, BROKEN_MODEL_ID));
    assert.equal(store.current.pendingEmbeddingModel, null);
  });
});

function emptyDelta(toGeneration: number): ReportDelta {
  return {
    from_generation: 0,
    to_generation: toGeneration,
    clusters_added: [],
    clusters_removed: [],
    clusters_updated: [],
    metrics: repoMetrics(),
    cache_stats: { hits: 0, misses: 0 },
    tool_version: "x",
  };
}

suite("turn embeddings off", () => {
  teardown(async () => {
    await vscode.workspace
      .getConfiguration(DESLOP_CONFIG_SECTION)
      .update(EMBEDDING_MODE_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
  });

  test("turnEmbeddingsOff sends the off request, persists mode=off, and applies the returned delta", async () => {
    const store = newStore();
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        if (method === REPORT_DELTA_METHOD) return Promise.resolve(emptyDelta(7));
        return Promise.resolve(null);
      },
    } as unknown as LanguageClient;

    await turnEmbeddingsOff(client, store);

    assert.deepEqual(
      calls.find((c) => c.method === EMBEDDING_SET_MODEL_METHOD)?.params,
      { provider_id: EMBEDDINGS_OFF_MODE, model_id: EMBEDDINGS_OFF_MODE },
      "the LSP must be told to switch the provider off",
    );
    assert.equal(
      vscode.workspace
        .getConfiguration(DESLOP_CONFIG_SECTION)
        .get<string>(EMBEDDING_MODE_SETTING),
      EMBEDDINGS_OFF_MODE,
      "embedding.mode must persist as off so the next session stays off",
    );
    assert.equal(store.current.generation, 7, "the returned delta settles the new generation");
  });

  test("turnEmbeddingsOff clears the pending marker when the LSP returns no delta", async () => {
    const store = newStore();
    const client = {
      sendRequest: (method: string) =>
        method === REPORT_DELTA_METHOD ? Promise.resolve(null) : Promise.resolve(null),
    } as unknown as LanguageClient;

    await turnEmbeddingsOff(client, store);
    assert.equal(
      store.current.pendingEmbeddingModel,
      null,
      "no delta means nothing to apply — the optimistic pending marker is cleared",
    );
  });

  test("turnEmbeddingsOff reverts the pending marker when the LSP rejects", async () => {
    const store = newStore();
    const client = {
      sendRequest: () => Promise.reject(new Error("backend unavailable")),
    } as unknown as LanguageClient;

    await turnEmbeddingsOff(client, store);
    assert.equal(
      store.current.pendingEmbeddingModel,
      null,
      "a failed switch must not strand the UI on a half-applied off state",
    );
  });
});
