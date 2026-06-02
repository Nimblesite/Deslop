// Unit: the activation-adjacent glue in extension.ts that the E2E suite only
// reaches through the bundled dist/ entrypoint (so it never lands in the
// instrumented out/ coverage). Each helper is exported for direct exercise
// under vscode-test against the real workspace + document APIs.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import {
  currentApi,
  currentBinarySettings,
  deactivate,
  requireResolved,
  startLanguageClient,
  syncTopOffendersContext,
  wireDirtyDocuments,
} from "../../extension";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";
import { ResolvedBinary } from "../../binary";

function resolvedLsp(): ResolvedBinary {
  return {
    kind: "lsp",
    componentId: "deslop-lsp",
    source: "bundled",
    path: "/tmp/deslop-lsp",
    version: "1.0.0",
  };
}

function clusterAcross(dirtyPath: string, otherPath: string): ReportCluster {
  return {
    id: "c1",
    weight: 5,
    size: 2,
    canonical_node_count: 3,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [
      { path: dirtyPath, start_byte: 0, end_byte: 4, hidden: false },
      { path: otherPath, start_byte: 0, end_byte: 4, hidden: false },
    ],
    occurrences_total: 2,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

function reportWith(clusters: ReportCluster[]): Report {
  return {
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 2,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 0,
      duplicated_loc: 0,
      duplication_percent: 0,
      clusters_total: clusters.length,
      duplicated_files: 0,
      threshold: { percent: 0, breached: false, source: "none" },
      per_file: [],
    },
    schema_doc: "",
    action_hints: [],
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters,
  };
}

suite("extension activation glue", () => {
  test("currentApi reflects the pre-activation module state", () => {
    const api = currentApi();
    // No activate() has run in this unit context, so the live handles are
    // empty — but the read-through getters must still be wired.
    assert.equal(api.client, undefined);
    assert.equal(api.resolvedLsp, undefined);
    assert.equal(api.resolvedMcp, undefined);
    assert.equal(api.reportStore, undefined);
    assert.ok(!("then" in api), "currentApi returns a plain snapshot, not a thenable");
  });

  test("syncTopOffendersContext normalises every view axis without throwing", async () => {
    // Re-fetch the configuration after each update — a captured
    // WorkspaceConfiguration is a snapshot and would not observe the writes.
    const read = (): vscode.WorkspaceConfiguration =>
      vscode.workspace.getConfiguration("deslop");
    try {
      // Explicit known values exercise the folder/path/split-on arms.
      await read().update("topOffenders.groupBy", "folder", vscode.ConfigurationTarget.Workspace);
      await read().update("topOffenders.sortBy", "path", vscode.ConfigurationTarget.Workspace);
      await read().update("topOffenders.splitByLanguage", true, vscode.ConfigurationTarget.Workspace);
      assert.equal(read().get<string>("topOffenders.groupBy"), "folder");
      syncTopOffendersContext();

      // Unknown grouping/sort fall back to the spec defaults — the function
      // must coerce rather than propagate the bad value.
      await read().update("topOffenders.groupBy", "nonsense", vscode.ConfigurationTarget.Workspace);
      await read().update("topOffenders.sortBy", "nonsense", vscode.ConfigurationTarget.Workspace);
      await read().update("topOffenders.splitByLanguage", false, vscode.ConfigurationTarget.Workspace);
      syncTopOffendersContext();
      assert.equal(read().get<string>("topOffenders.groupBy"), "nonsense", "raw config value is untouched");
    } finally {
      await read().update("topOffenders.groupBy", undefined, vscode.ConfigurationTarget.Workspace);
      await read().update("topOffenders.sortBy", undefined, vscode.ConfigurationTarget.Workspace);
      await read().update("topOffenders.splitByLanguage", undefined, vscode.ConfigurationTarget.Workspace);
    }
  });

  test("startLanguageClient builds a configured LanguageClient without spawning", () => {
    const client = startLanguageClient(resolvedLsp());
    assert.ok(client instanceof LanguageClient, "must return a real LanguageClient instance");
    // Constructed but never started — no server process is spawned.
    assert.equal(client.name, "Deslop");
  });

  test("currentBinarySettings mirrors the configured override paths", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    try {
      await cfg.update("lspPath", "/custom/lsp", vscode.ConfigurationTarget.Workspace);
      await cfg.update("mcpPath", "/custom/mcp", vscode.ConfigurationTarget.Workspace);
      assert.deepEqual(currentBinarySettings(), {
        lspPath: "/custom/lsp",
        mcpPath: "/custom/mcp",
      });
    } finally {
      await cfg.update("lspPath", undefined, vscode.ConfigurationTarget.Workspace);
      await cfg.update("mcpPath", undefined, vscode.ConfigurationTarget.Workspace);
    }
  });

  test("requireResolved returns the binary when present", () => {
    const lsp = resolvedLsp();
    assert.equal(requireResolved({ "deslop-lsp": lsp }, "deslop-lsp"), lsp);
  });

  test("requireResolved throws when the component is missing", () => {
    assert.throws(
      () => requireResolved({}, "deslop-lsp"),
      /deslop-lsp did not resolve/,
    );
  });

  test("deactivate is a no-op when no client is running", async () => {
    // The module-level client is undefined in this unit context, so the
    // early-return path runs without touching a server.
    await deactivate();
  });

  test("wireDirtyDocuments hides a file's occurrences on edit and restores them on save", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-dirty-"));
    const dirtyFile = path.join(dir, "Edited.cs");
    fs.writeFileSync(dirtyFile, "code\n", "utf8");

    const store = new ReportStore();
    store.setSnapshot(reportWith([clusterAcross(dirtyFile, "/other/Stable.cs")]), 0);
    assert.equal(store.current.visibleReport?.clusters.length, 1, "cluster visible while clean");

    const subscription = wireDirtyDocuments(store);
    try {
      const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(dirtyFile));
      const editor = await vscode.window.showTextDocument(doc);
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "x"));
      assert.equal(
        store.current.visibleReport?.clusters.length,
        0,
        "editing the file drops it below two occurrences — the cluster is elided",
      );

      await doc.save();
      assert.equal(
        store.current.visibleReport?.clusters.length,
        1,
        "saving clears the dirty marker so the cluster reappears",
      );
    } finally {
      subscription.dispose();
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });
});
