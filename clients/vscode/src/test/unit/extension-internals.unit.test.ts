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
        source: "bundled",
        path: "/tmp/lsp",
        version: "1.0.0",
      },
      {
        kind: "mcp",
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
        source: "env",
        path: "/tmp/lsp",
        version: null,
      },
      undefined,
    );
  });

  test("revealActiveBinary handles a missing lsp binary", () => {
    revealActiveBinary(undefined, undefined);
  });

  test("tryResolveOptional swallows failure and returns undefined", () => {
    const result = tryResolveOptional("/nonexistent/extension", "mcp", "0.1.0");
    assert.equal(result, undefined);
  });

  test("wireNotifications registers handlers without throwing", () => {
    const handlers = new Map<string, Function>();
    const client = {
      onNotification: (name: string, cb: Function) => handlers.set(name, cb),
      sendRequest: () => Promise.resolve(null),
    } as unknown as LanguageClient;
    wireNotifications(client, new ReportStore());
    assert.ok(handlers.has("codededup/reportChanged"));
    assert.ok(handlers.has("codededup/analysisState"));
  });

  test("wireNotifications analysisState handler logs without throwing", () => {
    let stateCb: ((s: string) => void) | undefined;
    const client = {
      onNotification: (name: string, cb: (s: string) => void) => {
        if (name === "codededup/analysisState") stateCb = cb;
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
        if (name === "codededup/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        if (name === "codededup/reportDelta") {
          return Promise.resolve({
            from_generation: 0,
            to_generation: 1,
            clusters_added: [],
            clusters_removed: [],
            clusters_updated: [],
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
        report_schema_version: 3,
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
        },
        schema_doc: "",
        action_hints: [],
        embedding_provenance: null,
        clusters: [],
      },
      0,
    );
    wireNotifications(client, store);
    await changedCb?.({ generation: 1, summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0, worst_weight: 0 } });
    assert.ok(requests.includes("codededup/reportDelta"));
  });

  test("wireNotifications reportChanged falls back to reportGet when delta is null", async () => {
    let changedCb: ((p: unknown) => Promise<void>) | undefined;
    const requests: string[] = [];
    const client = {
      onNotification: (name: string, cb: (p: unknown) => Promise<void>) => {
        if (name === "codededup/reportChanged") changedCb = cb;
      },
      sendRequest: (name: string) => {
        requests.push(name);
        if (name === "codededup/reportDelta") return Promise.resolve(null);
        return Promise.resolve({
          report_schema_version: 3,
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
          embedding_provenance: null,
          clusters: [],
        });
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();
    wireNotifications(client, store);
    await changedCb?.({ generation: 5, summary: { clusters_added: 0, clusters_removed: 0, clusters_updated: 0, worst_weight: 0 } });
    assert.ok(requests.includes("codededup/reportGet"));
  });

  test("seedInitialReport stores the returned snapshot", async () => {
    const client = {
      sendRequest: () =>
        Promise.resolve({
          report_schema_version: 3,
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
          },
          schema_doc: "",
          action_hints: [],
          embedding_provenance: null,
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
});
