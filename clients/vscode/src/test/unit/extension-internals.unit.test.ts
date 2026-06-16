// Unit: internal helpers in extension.ts — safe to call under vscode-test.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import {
  surfaceStartupFailure,
  currentExtensionVersion,
  revealActiveBinary,
  tryResolveOptional,
  wireNotifications,
  seedInitialReport,
  buildServerArgs,
  syncEmbeddingSettingsToLsp,
  resolveWorkspaceRoot,
} from "../../extension";
import { ReportStore } from "../../reportStore";
import {
  BundledBinaryMissingError,
  UnsupportedPlatformError,
} from "../../binary";

function fakeCtx(version: unknown): vscode.ExtensionContext {
  return {
    extension: { packageJSON: { version } },
  } as unknown as vscode.ExtensionContext;
}

const LEGACY_LSP_FLAGS = [
  "--min-nodes",
  "--embeddings",
  "--embedding-provider",
  "--embedding-model",
  "--embedding-endpoint",
] as const;

function assertNoLegacyLspFlags(args: string[]): void {
  for (const flag of LEGACY_LSP_FLAGS) {
    assert.equal(
      args.includes(flag),
      false,
      `issue #83: buildServerArgs must not pass legacy ${flag}: ${JSON.stringify(args)}`,
    );
  }
}

suite("extension internals", () => {
  test("currentExtensionVersion reads packageJSON.version", () => {
    assert.equal(currentExtensionVersion(fakeCtx("1.2.3")), "1.2.3");
  });

  test("currentExtensionVersion falls back to 0.0.0 when absent", () => {
    assert.equal(currentExtensionVersion(fakeCtx(undefined)), "0.0.0");
    assert.equal(currentExtensionVersion(fakeCtx(42)), "0.0.0");
  });

  test("surfaceStartupFailure handles a BundledBinaryMissingError", () => {
    surfaceStartupFailure(new BundledBinaryMissingError("/nowhere"));
  });

  test("surfaceStartupFailure handles an UnsupportedPlatformError", () => {
    surfaceStartupFailure(new UnsupportedPlatformError("plan9", "arm64"));
  });

  test("surfaceStartupFailure handles a generic error", () => {
    surfaceStartupFailure(new Error("boom"));
  });

  test("revealActiveBinary with both resolved", () => {
    revealActiveBinary(
      {
        kind: "lsp",
        componentId: "deslop-lsp",
        source: "bundled",
        path: "/tmp/lsp",
        version: "1.0.0",
      },
      {
        kind: "mcp",
        componentId: "deslop-mcp",
        source: "bundled",
        path: "/tmp/mcp",
        version: "1.0.0",
      },
    );
  });

  test("revealActiveBinary handles a missing mcp binary", () => {
    revealActiveBinary(
      {
        kind: "lsp",
        componentId: "deslop-lsp",
        source: "env-dir",
        path: "/tmp/lsp",
        version: "1.0.0",
      },
      undefined,
    );
  });

  test("revealActiveBinary handles a missing lsp binary", () => {
    revealActiveBinary(undefined, undefined);
  });

  test("tryResolveOptional swallows failure and returns undefined", () => {
    const saved = { ...process.env };
    delete process.env["DESLOP_BINARY_DIR"];
    process.env["PATH"] = "/nope";
    try {
      const result = tryResolveOptional("/nonexistent/extension", "mcp", optionalManifest());
      assert.equal(result, undefined);
    } finally {
      process.env = saved;
    }
  });

  test("buildServerArgs keeps issue #83 legacy flags out of fresh VSIX sessions", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("embedding.mode", "off", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.provider", "ollama", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.model", "nomic-embed-text", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.endpoint", "http://127.0.0.1:11434", vscode.ConfigurationTarget.Global);
    const args = buildServerArgs("/tmp/deslop-workspace", false);
    assert.deepEqual(args, ["/tmp/deslop-workspace"]);
    assertNoLegacyLspFlags(args);
  });

  test("buildServerArgs keeps issue #83 legacy flags out of debug VSIX sessions", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("embedding.mode", "auto", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.provider", "ollama", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.model", "nomic-embed-text", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.endpoint", "http://127.0.0.1:11434", vscode.ConfigurationTarget.Global);
    const args = buildServerArgs("/tmp/deslop-workspace", true);
    assert.deepEqual(args, ["/tmp/deslop-workspace", "--debug"]);
    assertNoLegacyLspFlags(args);
  });

  test("buildServerArgs forwards issue #28 LSP throttle settings", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("lsp.workerThreads", 2, vscode.ConfigurationTarget.Global);
    await cfg.update("lsp.nice", 5, vscode.ConfigurationTarget.Global);
    try {
      const args = buildServerArgs("/tmp/deslop-workspace", false);
      assert.deepEqual(args, [
        "/tmp/deslop-workspace",
        "--worker-threads",
        "2",
        "--nice",
        "5",
      ]);
      assertNoLegacyLspFlags(args);
    } finally {
      await cfg.update("lsp.workerThreads", 0, vscode.ConfigurationTarget.Global);
      await cfg.update("lsp.nice", 0, vscode.ConfigurationTarget.Global);
    }
  });

  test("wireNotifications registers handlers without throwing", () => {
    const handlers = new Map<string, (...args: unknown[]) => unknown>();
    const client = {
      onNotification: (name: string, cb: (...args: unknown[]) => unknown) => handlers.set(name, cb),
      sendRequest: () => Promise.resolve(null),
    } as unknown as LanguageClient;
    wireNotifications(client, new ReportStore());
    assert.ok(handlers.has("deslop/reportChanged"));
    assert.ok(handlers.has("deslop/analysisState"));
    assert.ok(handlers.has("deslop/embeddingProgress"));
  });

  test("syncEmbeddingSettingsToLsp forwards shared workspace settings", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("embedding.mode", "auto", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.provider", "ollama", vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.model", "nomic-embed-text", vscode.ConfigurationTarget.Global);
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(null);
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    await syncEmbeddingSettingsToLsp(store, () => client);
    assert.deepEqual(calls, [
      {
        method: "deslop/embeddingSetModel",
        params: {
          provider_id: "ollama",
          model_id: "nomic-embed-text",
          endpoint: "http://127.0.0.1:11434",
        },
      },
    ]);
    assert.equal(store.current.pendingEmbeddingModel, "nomic-embed-text");
  });

  test("wireNotifications embeddingProgress handler pushes the payload into the store", () => {
    let progressCb: ((p: unknown) => void) | undefined;
    const client = {
      onNotification: (name: string, cb: (p: unknown) => void) => {
        if (name === "deslop/embeddingProgress") progressCb = cb;
      },
      sendRequest: () => Promise.resolve(null),
    } as unknown as LanguageClient;
    const store = new ReportStore();
    wireNotifications(client, store);
    progressCb?.({
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 100,
    });
    assert.equal(store.current.embeddingProgress?.total, 100);
    progressCb?.({
      phase: "complete",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 100,
      total: 100,
    });
    // After complete, the store clears the progress so the Session panel
    // falls back to the fresh report.
    assert.equal(store.current.embeddingProgress, null);
  });

  test("wireNotifications embeddingProgress complete refreshes the report", async () => {
    let progressCb: ((p: unknown) => void) | undefined;
    const requests: string[] = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => void) => {
        if (name === "deslop/embeddingProgress") progressCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        return Promise.resolve({
          tool_version: "v",
          min_nodes: 30,
          files_analysed: 7,
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
          embedding_provenance: {
            provider_id: "ollama",
            model_id: "nomic-embed-text",
            model_version: "test",
            dimensions: 768,
            attempted_subtrees: 1,
            indexed_subtrees: 1,
            failed_subtrees: 0,
          },
          clusters: [],
        });
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    wireNotifications(client, store);
    progressCb?.({
      phase: "complete",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 1,
      total: 1,
    });
    await Promise.resolve();
    assert.ok(requests.includes("deslop/reportGet"));
    assert.equal(store.current.report?.files_analysed, 7);
  });

  test("wireNotifications analysisState handler logs without throwing", () => {
    let stateCb: ((s: string) => void) | undefined;
    const client = {
      onNotification: (name: string, cb: (s: string) => void) => {
        if (name === "deslop/analysisState") stateCb = cb;
      },
      sendRequest: () => Promise.resolve(null),
    } as unknown as LanguageClient;
    wireNotifications(client, new ReportStore());
    stateCb?.("running");
  });

  test("wireNotifications reportChanged applies a delta", async () => {
    let changedCb: ((p: unknown) => Promise<void>) | undefined;
    const requests: string[] = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => Promise<void>) => {
        if (name === "deslop/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        if (name === "deslop/reportDelta") {
          return Promise.resolve({
            from_generation: 0,
            to_generation: 1,
            clusters_added: [],
            clusters_removed: [],
            clusters_updated: [],
            metrics: {
              analysed_loc: 0,
              duplicated_loc: 0,
              duplication_percent: 0,
              clusters_total: 0,
              duplicated_files: 0,
              threshold: { percent: 0, breached: false, source: "none" },
              per_file: [],
            },
            cache_stats: { hits: 0, misses: 0 },
            tool_version: "v",
          });
        }
        return Promise.resolve({});
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    store.setSnapshot(
      {
        tool_version: "v0",
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
        embedding_provenance: undefined,
        clusters: [],
      },
      0,
    );
    wireNotifications(client, store);
    await changedCb?.({ generation: 1, summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0, worst_weight: 0 } });
    assert.ok(requests.includes("deslop/reportDelta"));
  });

  test("wireNotifications reportChanged falls back to reportGet when delta is null", async () => {
    let changedCb: ((p: unknown) => Promise<void>) | undefined;
    const requests: string[] = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => Promise<void>) => {
        if (name === "deslop/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        if (name === "deslop/reportDelta") return Promise.resolve(null);
        return Promise.resolve({
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
          embedding_provenance: undefined,
          clusters: [],
        });
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    wireNotifications(client, store);
    await changedCb?.({ generation: 5, summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0, worst_weight: 0 } });
    assert.ok(requests.includes("deslop/reportGet"));
  });

  test("seedInitialReport stores the returned snapshot", async () => {
    const client = {
      sendRequest: () =>
        Promise.resolve({
          tool_version: "v",
          min_nodes: 30,
          files_analysed: 2,
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
          embedding_provenance: undefined,
          clusters: [],
        }),
    } as unknown as LanguageClient;
    const store = new ReportStore();
    await seedInitialReport(client, store);
    assert.equal(store.current.report?.files_analysed, 2);
  });

  test("seedInitialReport swallows a rejected request", async () => {
    const client = {
      sendRequest: () => Promise.reject(new Error("no backend")),
    } as unknown as LanguageClient;
    await seedInitialReport(client, new ReportStore());
  });

  test("resolveWorkspaceRoot returns the fsPath of the first workspace folder when present", () => {
    // Under the test runner the workspace always points at the csharp-small
    // fixture (configured via DESLOP_TEST_FIXTURE / vscode-test.mjs), so
    // resolveWorkspaceRoot must hand back that directory's fsPath.
    const folders = vscode.workspace.workspaceFolders;
    assert.ok(
      folders && folders.length > 0,
      "vscode-test runner must open a workspace folder — resolveWorkspaceRoot cannot be exercised otherwise",
    );
    const first = folders[0];
    assert.ok(first, "workspaceFolders[0] must exist");
    assert.equal(resolveWorkspaceRoot(), first.uri.fsPath);
  });
});

function optionalManifest() {
  return {
    manifestVersion: 1,
    product: { id: "deslop", version: "0.1.0" },
    components: [
      {
        id: "deslop-mcp",
        kind: "mcp",
        language: "rust",
        binaryName: "deslop-mcp",
        expectedVersion: "0.1.0",
        bundled: { bundlePath: "bin/${platform}/${binaryName}${exe}" },
        required: true,
      },
    ],
    hosts: {},
  };
}
