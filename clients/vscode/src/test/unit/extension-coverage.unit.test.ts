// Unit: focused coverage for extension branches that are awkward to reach
// through full activation.

import * as assert from "node:assert/strict";
import type { LanguageClient } from "vscode-languageclient/node";
import * as vscode from "vscode";
import {
  buildServerArgs,
  currentInitializationOptions,
  resolveWorkspaceRoot,
  surfaceStartupFailure,
  syncEmbeddingSettingsToLsp,
  wireNotifications,
} from "../../extension";
import { ReportStore } from "../../reportStore";
import { AnalysisState, Report } from "../../types/report";

function reportWithEmbedding(
  embedding: Report["embedding_provenance"] = null,
): Report {
  return {
    report_schema_version: 1,
    tool_version: "v",
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
    embedding_provenance: embedding,
    clusters: [],
  };
}

async function setEmbeddingConfig(values: {
  mode: string;
  provider?: string;
  model?: string;
  endpoint?: string;
}): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update("embedding.mode", values.mode, vscode.ConfigurationTarget.Global);
  await cfg.update("embedding.provider", values.provider ?? "ollama", vscode.ConfigurationTarget.Global);
  await cfg.update("embedding.model", values.model ?? "nomic-embed-text", vscode.ConfigurationTarget.Global);
  await cfg.update(
    "embedding.endpoint",
    values.endpoint ?? "http://127.0.0.1:11434",
    vscode.ConfigurationTarget.Global,
  );
}

async function resetDeslopConfig(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await setEmbeddingConfig({ mode: "off", provider: "ollama", model: "nomic-embed-text" });
  await cfg.update("minNodes", 30, vscode.ConfigurationTarget.Global);
  await cfg.update("incremental", true, vscode.ConfigurationTarget.Global);
  await cfg.update("configPath", "", vscode.ConfigurationTarget.Global);
}

suite("extension coverage branches", () => {
  teardown(async () => {
    await resetDeslopConfig();
  });

  test("buildServerArgs handles missing workspace roots", () => {
    assert.deepEqual(buildServerArgs(undefined, false), []);
    assert.deepEqual(buildServerArgs(undefined, true), ["--debug"]);
  });

  test("resolveWorkspaceRoot returns the first VS Code workspace folder", () => {
    assert.equal(resolveWorkspaceRoot(), vscode.workspace.workspaceFolders?.[0]?.uri.fsPath);
  });

  test("currentInitializationOptions mirrors the deslop configuration", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("minNodes", 42, vscode.ConfigurationTarget.Global);
    await cfg.update("incremental", false, vscode.ConfigurationTarget.Global);
    await cfg.update("configPath", "/tmp/deslop.toml", vscode.ConfigurationTarget.Global);
    await setEmbeddingConfig({
      mode: "required",
      provider: "stub",
      model: "stub",
      endpoint: "http://127.0.0.1:9000",
    });

    assert.deepEqual(currentInitializationOptions(), {
      minNodes: 42,
      embedding: {
        provider: "stub",
        model: "stub",
        endpoint: "http://127.0.0.1:9000",
        mode: "required",
      },
      incremental: false,
      configPath: "/tmp/deslop.toml",
    });
  });

  test("surfaceStartupFailure records the failed lifecycle when a store is supplied", () => {
    const store = new ReportStore();
    surfaceStartupFailure(new Error("boom"), store);

    assert.equal(store.current.lifecycle.kind, "failed");
    assert.match(
      store.current.lifecycle.kind === "failed" ? store.current.lifecycle.message : "",
      /failed to start/i,
    );
  });

  test("wireNotifications maps idle and errored analysis states into lifecycle", () => {
    let stateCb: ((state: AnalysisState) => void) | undefined;
    const client = {
      onNotification: (name: string, cb: (state: AnalysisState) => void) => {
        if (name === "deslop/analysisState") stateCb = cb;
      },
      sendRequest: () => Promise.resolve(null),
    } as unknown as LanguageClient;
    const store = new ReportStore();

    wireNotifications(client, store);
    stateCb?.("idle");
    assert.equal(store.current.lifecycle.kind, "ready");

    stateCb?.("errored");
    const failedLifecycle = store.current.lifecycle;
    assert.equal(failedLifecycle.kind, "failed");
    assert.ok("message" in failedLifecycle);
    assert.match(String(failedLifecycle.message), /Analysis failed/);
  });

  test("syncEmbeddingSettingsToLsp skips when no client or embeddings are off", async () => {
    await setEmbeddingConfig({ mode: "auto", provider: "stub", model: "stub" });
    await syncEmbeddingSettingsToLsp(new ReportStore(), () => undefined);

    await setEmbeddingConfig({ mode: "off", provider: "stub", model: "stub" });
    const client = {
      sendRequest: () => {
        throw new Error("must not be called");
      },
    } as unknown as LanguageClient;
    await syncEmbeddingSettingsToLsp(new ReportStore(), () => client);
  });

  test("syncEmbeddingSettingsToLsp skips pending and already-active models", async () => {
    await setEmbeddingConfig({ mode: "auto", provider: "stub", model: "stub" });
    const client = {
      sendRequest: () => {
        throw new Error("must not be called");
      },
    } as unknown as LanguageClient;

    const pending = new ReportStore();
    pending.setPendingEmbeddingModel("stub");
    await syncEmbeddingSettingsToLsp(pending, () => client);

    const active = new ReportStore();
    active.setSnapshot(
      reportWithEmbedding({
        provider_id: "stub",
        model_id: "stub",
        model_version: "0",
        dimensions: 64,
      }),
      0,
    );
    await syncEmbeddingSettingsToLsp(active, () => client);
  });

  test("syncEmbeddingSettingsToLsp clears pending model when the LSP rejects", async () => {
    await setEmbeddingConfig({
      mode: "auto",
      provider: "ollama",
      model: "broken-model",
      endpoint: "http://127.0.0.1:11434",
    });
    const store = new ReportStore();
    const client = {
      sendRequest: () => Promise.reject(new Error("backend unavailable")),
    } as unknown as LanguageClient;

    await assert.rejects(
      () => syncEmbeddingSettingsToLsp(store, () => client),
      /backend unavailable/,
    );
    assert.equal(store.current.pendingEmbeddingModel, null);
  });
});
