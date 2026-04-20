// Unit: pure helper logic inside embeddingPicker. Runs under vscode-test so
// the transitive `vscode` import resolves against the real host.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import {
  buildItems,
  formatSize,
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
      report_schema_version: 1,
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
      },
      schema_doc: "",
      action_hints: [],
      embedding_provenance: embedding ?? null,
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
    size_bytes: 1024 * 1024 * 42,
    is_embedding_model: true,
    ...overrides,
  };
}

suite("embeddingPicker helpers", () => {
  test("formatSize grows through B/KiB/MiB/GiB", () => {
    assert.match(formatSize(100), /B$/);
    assert.match(formatSize(2048), /KiB$/);
    assert.match(formatSize(5 * 1024 * 1024), /MiB$/);
    assert.match(formatSize(10 * 1024 * 1024 * 1024), /GiB$/);
  });

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
    // There's still the stub entry + 'Pull new' + 'Refresh list'.
    assert.ok(items.some((i) => i.label?.includes("stub")));
    assert.ok(items.some((i) => i.label?.includes("Pull a new model")));
    assert.ok(items.some((i) => i.label?.includes("Refresh list")));
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
      size_bytes: null,
      is_embedding_model: true,
    });
  });

  test("setModel happy path persists the workspace config", async () => {
    const client = {
      sendRequest: () => Promise.resolve(undefined),
    } as unknown as LanguageClient;
    await setModel(client, {
      provider_id: "ollama",
      model_id: "nomic-embed-code",
      model_version: "0",
      dimensions: 768,
      size_bytes: null,
      is_embedding_model: true,
    });
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
      size_bytes: null,
      is_embedding_model: true,
    });
    const swap = calls.find((call) => call.method === "deslop/embeddingSetModel");
    assert.ok(swap, `expected embeddingSetModel request; got ${JSON.stringify(calls)}`);
    assert.deepEqual(swap.params, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
    });
  });

  test("setModelFromPicker marks the store's pending model BEFORE dispatching the RPC", async () => {
    const store = newStore();
    const events: string[] = [];
    const recorded: Array<string | null> = [];
    store.onDidChange((s) => {
      events.push("change");
      recorded.push(s.pendingEmbeddingModel);
    });
    const client = {
      sendRequest: () => {
        // Capture the store's pending state at the moment the RPC is issued.
        events.push(`rpc(${store.current.pendingEmbeddingModel ?? "null"})`);
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    await setModelFromPicker(client, store, {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "0",
      dimensions: 768,
      size_bytes: null,
      is_embedding_model: true,
    });
    assert.ok(
      events.includes("rpc(nomic-embed-text)"),
      `RPC must fire with pending model already set; got ${JSON.stringify(events)}`,
    );
    assert.equal(
      store.current.pendingEmbeddingModel,
      "nomic-embed-text",
      "pending model stays set until the new report arrives",
    );
  });

  test("buildItems marks the 'Ollama models' header as a non-pickable separator", () => {
    const items = buildItems(
      [model("ollama", "nomic-embed-text"), model("stub", "stub", { dimensions: 64, size_bytes: null })],
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
        model("ollama", "nomic-embed-text"),
        model("ollama", "unknown-model", { is_embedding_model: false, size_bytes: null }),
        model("stub", "stub", { dimensions: 64, size_bytes: null }),
      ],
      newStore(active),
    );
    const nomic = items.find((i) => i.label?.includes("nomic-embed-text"));
    assert.ok(nomic, "nomic entry should exist");
    assert.match(nomic.label, /active/);
    // The unknown model should be labelled 'may not embed' since is_embedding_model=false.
    const unknown = items.find((i) => i.label?.includes("unknown-model"));
    assert.ok(unknown, "unknown-model entry should exist");
    assert.match(unknown.description ?? "", /may not embed/);
  });
});
