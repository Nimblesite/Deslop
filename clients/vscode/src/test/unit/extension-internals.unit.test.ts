// Unit: internal helpers in extension.ts — safe to call under vscode-test.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { notifyingClient, recordingClient, rejectingClient, respondingClient } from "./client.helpers";
import {
  surfaceStartupFailure,
  currentExtensionVersion,
  revealActiveBinary,
  tryResolveOptional,
  seedInitialReport,
  buildServerArgs,
  syncEmbeddingSettingsToLsp,
  resolveWorkspaceRoot,
} from "../../extension";
import { wireNotifications } from "../../notifications";
import { ReportStore } from "../../reportStore";
import { emptyReport, repoMetrics } from "./report.helpers";
import {
  BundledBinaryMissingError,
  UnsupportedPlatformError,
} from "../../binary";

const OLLAMA_PROVIDER_ID = "ollama";
const DEFAULT_EMBEDDING_MODEL = "nomic-embed-text";
const TEST_BINARY_VERSION = "1.0.0";
const MCP_COMPONENT_ID = "deslop-mcp";
const EMBEDDING_MODE_SETTING = "embedding.mode";
const EMBEDDING_PROVIDER_SETTING = "embedding.provider";
const EMBEDDING_MODEL_SETTING = "embedding.model";
const DEFAULT_EMBEDDING_ENDPOINT = "http://127.0.0.1:11434";
const TEST_WORKSPACE_ROOT = "/tmp/deslop-workspace";
const DESLOP_CONFIGURATION_NAMESPACE = "deslop";

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
        version: TEST_BINARY_VERSION,
      },
      {
        kind: "mcp",
        componentId: MCP_COMPONENT_ID,
        source: "bundled",
        path: "/tmp/mcp",
        version: TEST_BINARY_VERSION,
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
        version: TEST_BINARY_VERSION,
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
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
    await cfg.update(EMBEDDING_MODE_SETTING, "off", vscode.ConfigurationTarget.Global);
    await cfg.update(EMBEDDING_PROVIDER_SETTING, OLLAMA_PROVIDER_ID, vscode.ConfigurationTarget.Global);
    await cfg.update(EMBEDDING_MODEL_SETTING, DEFAULT_EMBEDDING_MODEL, vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.endpoint", DEFAULT_EMBEDDING_ENDPOINT, vscode.ConfigurationTarget.Global);
    const args = buildServerArgs("/tmp/deslop-workspace", false);
    assert.deepEqual(args, [TEST_WORKSPACE_ROOT]);
    assertNoLegacyLspFlags(args);
  });

  test("buildServerArgs keeps issue #83 legacy flags out of debug VSIX sessions", async () => {
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
    await cfg.update(EMBEDDING_MODE_SETTING, "auto", vscode.ConfigurationTarget.Global);
    await cfg.update(EMBEDDING_PROVIDER_SETTING, OLLAMA_PROVIDER_ID, vscode.ConfigurationTarget.Global);
    await cfg.update(EMBEDDING_MODEL_SETTING, DEFAULT_EMBEDDING_MODEL, vscode.ConfigurationTarget.Global);
    await cfg.update("embedding.endpoint", DEFAULT_EMBEDDING_ENDPOINT, vscode.ConfigurationTarget.Global);
    const args = buildServerArgs("/tmp/deslop-workspace", true);
    assert.deepEqual(args, [TEST_WORKSPACE_ROOT, "--debug"]);
    assertNoLegacyLspFlags(args);
  });

  test("buildServerArgs forwards issue #28 LSP throttle settings", async () => {
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
    await cfg.update("lsp.workerThreads", 2, vscode.ConfigurationTarget.Global);
    await cfg.update("lsp.nice", 5, vscode.ConfigurationTarget.Global);
    try {
      const args = buildServerArgs("/tmp/deslop-workspace", false);
      assert.deepEqual(args, [
        TEST_WORKSPACE_ROOT,
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
    const { client, registered } = notifyingClient();
    wireNotifications(client, new ReportStore());
    assert.ok(registered("deslop/reportChanged"));
    assert.ok(registered("deslop/analysisState"));
    assert.ok(registered("deslop/embeddingProgress"));
  });

  test("syncEmbeddingSettingsToLsp forwards shared workspace settings", async () => {
    const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
    await cfg.update(EMBEDDING_MODE_SETTING, "auto", vscode.ConfigurationTarget.Global);
    await cfg.update(EMBEDDING_PROVIDER_SETTING, OLLAMA_PROVIDER_ID, vscode.ConfigurationTarget.Global);
    await cfg.update(EMBEDDING_MODEL_SETTING, DEFAULT_EMBEDDING_MODEL, vscode.ConfigurationTarget.Global);
    const { calls, client } = recordingClient(() => null);
    const store = new ReportStore();
    await syncEmbeddingSettingsToLsp(store, () => client);
    assert.deepEqual(calls, [
      {
        method: "deslop/embeddingSetModel",
        params: {
          provider_id: OLLAMA_PROVIDER_ID,
          model_id: DEFAULT_EMBEDDING_MODEL,
          endpoint: DEFAULT_EMBEDDING_ENDPOINT,
        },
      },
    ]);
    assert.equal(store.current.pendingEmbeddingModel, DEFAULT_EMBEDDING_MODEL);
  });

  test("wireNotifications embeddingProgress handler pushes the payload into the store", () => {
    const { client, notify } = notifyingClient();
    const store = new ReportStore();
    wireNotifications(client, store);
    notify("deslop/embeddingProgress", {
      phase: "starting",
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: DEFAULT_EMBEDDING_MODEL,
      done: 0,
      total: 100,
    });
    assert.equal(store.current.embeddingProgress?.total, 100);
    notify("deslop/embeddingProgress", {
      phase: "complete",
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: DEFAULT_EMBEDDING_MODEL,
      done: 100,
      total: 100,
    });
    // After complete, the store clears the progress so the Session panel
    // falls back to the fresh report.
    assert.equal(store.current.embeddingProgress, null);
  });

  test("wireNotifications embeddingProgress complete refreshes the report", async () => {
    const { calls, client, notify } = notifyingClient(() =>
      emptyReport({
        tool_version: "v",
        files_analysed: 7,
        metrics: repoMetrics(),
        embedding_provenance: {
          provider_id: "ollama",
          model_id: "nomic-embed-text",
          model_version: "test",
          dimensions: 768,
          attempted_subtrees: 1,
          succeeded_subtrees: 1,
          indexed_subtrees: 1,
          failed_subtrees: 0,
        },
      }),
    );
    const store = new ReportStore();
    const schedule = wireNotifications(client, store);
    notify("deslop/embeddingProgress", {
      phase: "complete",
      provider_id: OLLAMA_PROVIDER_ID,
      model_id: DEFAULT_EMBEDDING_MODEL,
      done: 1,
      total: 1,
    });
    // The refresh runs on the serialised queue; awaiting the schedule is the
    // deterministic completion point (no microtask counting, no timers).
    await schedule.settled();
    assert.ok(calls.some((call) => call.method === "deslop/reportGet"));
    assert.equal(store.current.report?.files_analysed, 7);
  });

  test("wireNotifications analysisState handler logs without throwing", () => {
    const { client, notify } = notifyingClient();
    wireNotifications(client, new ReportStore());
    notify("deslop/analysisState", "running");
  });

  test("seedInitialReport stores the returned snapshot", async () => {
    const client = respondingClient(() => emptyReport({
          tool_version: "v",
          files_analysed: 2,
          metrics: repoMetrics(),
        }));
    const store = new ReportStore();
    await seedInitialReport(client, store);
    assert.equal(store.current.report?.files_analysed, 2);
  });

  test("seedInitialReport swallows a rejected request", async () => {
    const client = rejectingClient("no backend");
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
    product: { id: DESLOP_CONFIGURATION_NAMESPACE, version: "0.1.0" },
    components: [
      {
        id: MCP_COMPONENT_ID,
        kind: "mcp",
        language: "rust",
        binaryName: MCP_COMPONENT_ID,
        expectedVersion: "0.1.0",
        bundled: { bundlePath: "bin/${platform}/${binaryName}${exe}" },
        required: true,
      },
    ],
    hosts: {},
  };
}
