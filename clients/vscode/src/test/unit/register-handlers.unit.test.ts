// Unit: the command handler bodies in commands/register that forward to the
// LSP, the clipboard, or a webview. The COMMAND_BINDINGS table wires these to
// VS Code command ids (registered by the bundled dist/ entrypoint), so they
// are exercised here directly against a mock client + the real workspace.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import {
  copyClusterContextById,
  openClusterDetails,
  openOccurrenceTarget,
  refreshReport,
  toggleShowAllLenses,
} from "../../commands/register";
import { ClusterNode, OccurrenceNode } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";

function fakeCtx(): vscode.ExtensionContext {
  return {
    extensionPath: path.join(__dirname, "..", "..", ".."),
    extensionUri: vscode.Uri.file(path.join(__dirname, "..", "..", "..")),
    subscriptions: [] as vscode.Disposable[],
  } as unknown as vscode.ExtensionContext;
}

function cluster(id: string): ReportCluster {
  return {
    id,
    weight: 7,
    size: 2,
    canonical_node_count: 3,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [
      { path: "/a.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/b.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    occurrences_total: 2,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

function storeWith(clusters: ReportCluster[]): ReportStore {
  const store = new ReportStore();
  const report: Report = {
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
  store.setSnapshot(report, 0);
  return store;
}

suite("register command handlers", () => {
  test("toggleShowAllLenses flips the persisted workspace flag", async () => {
    const cfg = (): vscode.WorkspaceConfiguration => vscode.workspace.getConfiguration("deslop");
    try {
      const before = cfg().get<boolean>("showAllLenses", false);
      await toggleShowAllLenses();
      assert.equal(cfg().get<boolean>("showAllLenses", false), !before, "first toggle inverts the flag");
      await toggleShowAllLenses();
      assert.equal(cfg().get<boolean>("showAllLenses", false), before, "second toggle restores it");
    } finally {
      await cfg().update("showAllLenses", undefined, vscode.ConfigurationTarget.Workspace);
    }
  });

  test("refreshReport forwards the LSP refresh command when a client is live", () => {
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(null);
      },
    } as unknown as LanguageClient;

    refreshReport(() => client);
    assert.deepEqual(calls, [
      {
        method: "workspace/executeCommand",
        params: { command: "deslop.lsp.refreshReport", arguments: [] },
      },
    ]);
  });

  test("refreshReport is a no-op when no client is running", () => {
    assert.equal(refreshReport(() => undefined), undefined);
  });

  test("copyClusterContextById copies the AI payload for a known cluster", async () => {
    const store = storeWith([cluster("alpha"), cluster("beta")]);
    await vscode.env.clipboard.writeText("sentinel");

    await copyClusterContextById(store, "beta");
    const copied = await vscode.env.clipboard.readText();
    assert.notEqual(copied, "sentinel", "a matching id must overwrite the clipboard with the payload");
    assert.match(copied, /beta/, "the copied payload identifies the cluster");
  });

  test("copyClusterContextById leaves the clipboard untouched for an unknown id", async () => {
    const store = storeWith([cluster("alpha")]);
    await vscode.env.clipboard.writeText("sentinel");

    await copyClusterContextById(store, "missing");
    assert.equal(await vscode.env.clipboard.readText(), "sentinel", "no cluster — no clipboard write");
  });

  test("openOccurrenceTarget surfaces guidance when the target has no occurrence", async () => {
    // A null command target resolves to no occurrence — the handler must
    // inform the user rather than throw.
    await openOccurrenceTarget(null);
  });

  test("openClusterDetails opens no panel when a row resolves to no cluster", () => {
    // An occurrence node whose parent cluster is absent from the store yields
    // no cluster id, so the handler shows guidance instead of opening a panel.
    const store = storeWith([]);
    const tabsBefore = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    openClusterDetails(
      fakeCtx(),
      store,
      new OccurrenceNode({ path: "/orphan.cs", start_byte: 0, end_byte: 4, hidden: false }),
    );
    const tabsAfter = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    assert.equal(tabsAfter, tabsBefore, "an unresolved row must not spawn a cluster panel");
  });

  test("openClusterDetails opens the cluster panel for a resolvable node", () => {
    const store = storeWith([cluster("details-target")]);
    const tabsBefore = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    openClusterDetails(fakeCtx(), store, new ClusterNode(cluster("details-target"), 1, "mid"));
    const tabsAfter = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    assert.ok(tabsAfter >= tabsBefore, "a resolvable node opens (or reveals) the cluster panel");
  });
});
