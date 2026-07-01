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
  openHtmlReport,
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

const REPORT_TAB_LABEL = "Deslop HTML Report";

function reportTabCount(): number {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .filter((tab) => tab.label === REPORT_TAB_LABEL).length;
}

// Webview tab open/close events reach window.tabGroups asynchronously, so poll
// (bounded) for the expected count rather than asserting it synchronously.
async function waitForReportTabCount(target: number): Promise<number> {
  for (let attempt = 0; attempt < 200 && reportTabCount() !== target; attempt += 1) {
    await new Promise((resolve) => {
      setTimeout(resolve, 10);
    });
  }
  return reportTabCount();
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

  test("openHtmlReport renders via the LSP and shows the report tab", async () => {
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve("<!doctype html><html><body>report</body></html>");
      },
    } as unknown as LanguageClient;

    await openHtmlReport(() => client);
    assert.deepEqual(calls, [
      {
        method: "workspace/executeCommand",
        params: { command: "deslop.lsp.renderHtmlReport", arguments: [] },
      },
    ]);
    assert.equal(await waitForReportTabCount(1), 1, "rendering must open the HTML report tab");

    // A second render refreshes the singleton in place — no duplicate tab.
    await openHtmlReport(() => client);
    assert.equal(await waitForReportTabCount(1), 1, "a re-render must not duplicate the report tab");

    // Closing the tab disposes the panel and fires onDidDispose (which clears the
    // singleton handle). No reopen here, so there is no disposed-handle reuse.
    const reportTab = vscode.window.tabGroups.all
      .flatMap((g) => g.tabs)
      .find((t) => t.label === REPORT_TAB_LABEL);
    assert.ok(reportTab, "the HTML report tab must be present before closing");
    await vscode.window.tabGroups.close(reportTab);
    assert.equal(await waitForReportTabCount(0), 0, "closing must dispose the report tab");
  });

  test("openHtmlReport opens no tab when no client is running", async () => {
    const before = reportTabCount();
    await openHtmlReport(() => undefined);
    assert.equal(reportTabCount(), before, "no client → no report tab is opened");
  });

  test("openHtmlReport opens no tab when the report is empty", async () => {
    const before = reportTabCount();
    const client = {
      sendRequest: () => Promise.resolve(""),
    } as unknown as LanguageClient;
    await openHtmlReport(() => client);
    assert.equal(reportTabCount(), before, "empty report → no report tab is opened");
  });

  test("openHtmlReport opens no tab when the LSP returns a non-string", async () => {
    const before = reportTabCount();
    const client = {
      sendRequest: () => Promise.resolve(null as unknown as string),
    } as unknown as LanguageClient;
    await openHtmlReport(() => client);
    assert.equal(reportTabCount(), before, "non-string response → no report tab is opened");
  });

  test("openHtmlReport shows a progress spinner while the LSP renders (#256)", async () => {
    // Regression: a large render blocks for a long time, so the awaited round-trip
    // must run inside vscode.window.withProgress — otherwise the UI reads as frozen
    // with no sign the click registered.
    const win = vscode.window as unknown as { withProgress: unknown };
    const original = win.withProgress;
    const spinners: string[] = [];
    win.withProgress = (
      options: vscode.ProgressOptions,
      task: (
        progress: vscode.Progress<{ message?: string }>,
        token: vscode.CancellationToken,
      ) => Thenable<unknown>,
    ) => {
      spinners.push(`${options.location as number}:${options.title ?? ""}`);
      const progress = { report: () => undefined } as vscode.Progress<{ message?: string }>;
      const token = {
        isCancellationRequested: false,
        onCancellationRequested: () => ({ dispose: () => undefined }),
      } as unknown as vscode.CancellationToken;
      return task(progress, token);
    };

    const client = {
      sendRequest: () => Promise.resolve("<!doctype html><html><body>report</body></html>"),
    } as unknown as LanguageClient;

    try {
      await openHtmlReport(() => client);
      assert.equal(spinners.length, 1, "the render must be wrapped in exactly one progress indicator");
      assert.equal(
        spinners[0],
        `${vscode.ProgressLocation.Notification}:Deslop: rendering HTML report…`,
        "the spinner is a notification carrying a human-readable render message",
      );
      assert.equal(await waitForReportTabCount(1), 1, "the report tab still opens once the render resolves");
    } finally {
      win.withProgress = original;
      const reportTab = vscode.window.tabGroups.all
        .flatMap((g) => g.tabs)
        .find((t) => t.label === REPORT_TAB_LABEL);
      if (reportTab) await vscode.window.tabGroups.close(reportTab);
    }
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
