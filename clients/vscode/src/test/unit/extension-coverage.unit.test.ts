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
} from "../../extension";
import { wireNotifications } from "../../notifications";
import { LifecyclePhase, ReportStore } from "../../reportStore";
import { AnalysisState, Report } from "../../types/report";
import { emptyReport, repoMetrics } from "./report.helpers";

const OLLAMA_PROVIDER_ID = "ollama";
const DEFAULT_EMBEDDING_MODEL = "nomic-embed-text";
const FAILED_LIFECYCLE_KIND = "failed";
const AUTO_EMBEDDING_MODE = "auto";

function reportWithEmbedding(
  embedding: Report["embedding_provenance"] = undefined,
): Report {
  return emptyReport({
    tool_version: "v",
    metrics: repoMetrics(),
    embedding_provenance: embedding,
  });
}

async function setEmbeddingConfig(values: {
  mode: string;
  provider?: string;
  model?: string;
  endpoint?: string;
}): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update("embedding.mode", values.mode, vscode.ConfigurationTarget.Global);
  await cfg.update("embedding.provider", values.provider ?? OLLAMA_PROVIDER_ID, vscode.ConfigurationTarget.Global);
  await cfg.update("embedding.model", values.model ?? DEFAULT_EMBEDDING_MODEL, vscode.ConfigurationTarget.Global);
  await cfg.update(
    "embedding.endpoint",
    values.endpoint ?? "http://127.0.0.1:11434",
    vscode.ConfigurationTarget.Global,
  );
}

async function resetDeslopConfig(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await setEmbeddingConfig({ mode: "off", provider: OLLAMA_PROVIDER_ID, model: DEFAULT_EMBEDDING_MODEL });
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
      provider: OLLAMA_PROVIDER_ID,
      model: "nomic-embed-code",
      endpoint: "http://127.0.0.1:9000",
    });

    assert.deepEqual(currentInitializationOptions(), {
      minNodes: 42,
      embedding: {
        provider: OLLAMA_PROVIDER_ID,
        model: "nomic-embed-code",
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

    assert.equal(store.current.lifecycle.kind, FAILED_LIFECYCLE_KIND);
    assert.match(
      store.current.lifecycle.kind === FAILED_LIFECYCLE_KIND ? store.current.lifecycle.message : "",
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
    // [VSIX reactivity] The LSP now sends the tagged AnalysisState object
    // ({state:"running",…}); the handler reads `state.state`, so every
    // branch must drive the lifecycle. The previous bare-string payload
    // left `state.state` undefined and silently disabled all of this.
    // Each transition is captured into its own const so the assertions
    // don't collapse the shared discriminant to `never`.
    stateCb?.({ state: "running", started_at_ms: 1 });
    const running: LifecyclePhase = store.current.lifecycle;
    assert.equal(running.kind, "analysing");

    stateCb?.({ state: "idle" });
    const idle: LifecyclePhase = store.current.lifecycle;
    assert.equal(idle.kind, "ready");

    stateCb?.({ state: "errored", message: "Analysis failed: bad fixture" });
    const failed: LifecyclePhase = store.current.lifecycle;
    assert.equal(failed.kind, FAILED_LIFECYCLE_KIND);
    assert.ok(
      failed.kind === FAILED_LIFECYCLE_KIND && /Analysis failed/.test(failed.message),
      "errored analysis state must surface its message on the failed lifecycle",
    );
  });

  test("syncEmbeddingSettingsToLsp skips when no client or embeddings are off", async () => {
    await setEmbeddingConfig({ mode: AUTO_EMBEDDING_MODE, provider: OLLAMA_PROVIDER_ID, model: DEFAULT_EMBEDDING_MODEL });
    await syncEmbeddingSettingsToLsp(new ReportStore(), () => undefined);

    await setEmbeddingConfig({ mode: "off", provider: OLLAMA_PROVIDER_ID, model: DEFAULT_EMBEDDING_MODEL });
    const client = {
      sendRequest: () => {
        throw new Error("must not be called");
      },
    } as unknown as LanguageClient;
    await syncEmbeddingSettingsToLsp(new ReportStore(), () => client);
  });

  test("syncEmbeddingSettingsToLsp skips pending and already-active models", async () => {
    await setEmbeddingConfig({ mode: AUTO_EMBEDDING_MODE, provider: OLLAMA_PROVIDER_ID, model: DEFAULT_EMBEDDING_MODEL });
    const client = {
      sendRequest: () => {
        throw new Error("must not be called");
      },
    } as unknown as LanguageClient;

    const pending = new ReportStore();
    pending.setPendingEmbeddingModel(DEFAULT_EMBEDDING_MODEL);
    await syncEmbeddingSettingsToLsp(pending, () => client);

    const active = new ReportStore();
    active.setSnapshot(
      reportWithEmbedding({
        provider_id: OLLAMA_PROVIDER_ID,
        model_id: DEFAULT_EMBEDDING_MODEL,
        model_version: "0",
        dimensions: 768,
        attempted_subtrees: 0,
        succeeded_subtrees: 0,
        indexed_subtrees: 0,
        failed_subtrees: 0,
      }),
      0,
    );
    await syncEmbeddingSettingsToLsp(active, () => client);
  });

  test("syncEmbeddingSettingsToLsp clears pending model when the LSP rejects", async () => {
    await setEmbeddingConfig({
      mode: AUTO_EMBEDDING_MODE,
      provider: OLLAMA_PROVIDER_ID,
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
