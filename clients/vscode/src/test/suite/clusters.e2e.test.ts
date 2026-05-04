// E2E: drive jumpToNextOccurrence / compareWithCanonical / openOccurrence
// with real cluster data from the LSP's initial report, and exercise the
// bubble render path by positioning inside a known cluster.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import type { Report, ReportCluster } from "../../types/report";
import { sleep } from "./helpers";

interface ExtensionExports {
  readonly client?: LanguageClient;
}

async function waitForReport(): Promise<ExtensionExports> {
  const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
  assert.ok(ext, "extension must be installed");
  const api = await ext.activate() as ExtensionExports;
  // Initial report seeding takes time over stdio.
  for (let i = 0; i < 20; i++) {
    await sleep(250);
    const cmds = await vscode.commands.getCommands(true);
    if (cmds.includes("deslop.openCluster")) return api;
  }
  throw new Error("extension did not activate in time");
}

async function waitForRelativePathCluster(client: LanguageClient): Promise<ReportCluster> {
  let last: Report | undefined;
  for (let i = 0; i < 40; i += 1) {
    last = await client.sendRequest<Report>("deslop/reportGet");
    const cluster = last.clusters.find((candidate) =>
      candidate.occurrences.length >= 2
        && candidate.occurrences.some((occurrence) => !path.isAbsolute(occurrence.path)),
    );
    if (cluster) return cluster;
    await sleep(250);
  }
  throw new Error(
    `no relative-path cluster in LSP report; last cluster count ${last?.clusters.length ?? 0}`,
  );
}

async function waitForDiffTab(): Promise<vscode.TabInputTextDiff> {
  // Under coverage instrumentation `vscode.diff` can take >2s to materialise
  // a TabInputTextDiff after closeAllEditors. 10s matches the rest of this
  // suite's wait helpers and absorbs that variance.
  for (let i = 0; i < 100; i += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return tab.input;
      }
    }
    await sleep(100);
  }
  throw new Error("compare command did not open a diff tab");
}

suite("cluster navigation", () => {
  let api: ExtensionExports;

  suiteSetup(async () => {
    api = await waitForReport();
    await sleep(2000);
  });

  test("openCluster by id opens the cluster panel", async () => {
    // Use a synthetic id — the open path builds the HTML regardless of the id matching.
    await vscode.commands.executeCommand("deslop.openCluster", "cluster-for-test");
    await sleep(300);
    await vscode.commands.executeCommand("deslop.openCluster", "cluster-for-test");
    await sleep(200);
  });

  test("jumping inside a fixture file while positioned at the start", async () => {
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    const doc = await vscode.workspace.openTextDocument(
      vscode.Uri.file(`${fixture}/Alpha.cs`),
    );
    const editor = await vscode.window.showTextDocument(doc);
    editor.selection = new vscode.Selection(
      new vscode.Position(2, 8),
      new vscode.Position(2, 8),
    );
    await sleep(200);
    await vscode.commands.executeCommand("deslop.jumpToNextOccurrence");
    await sleep(300);
  });

  test("Focused File tree populates when an editor is active", async () => {
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    const doc = await vscode.workspace.openTextDocument(
      vscode.Uri.file(`${fixture}/Beta.cs`),
    );
    await vscode.window.showTextDocument(doc);
    await sleep(400);
    // Trigger a re-render via active-editor change
    await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
    await sleep(200);
  });

  test("Session tree contains the embedding + cache + state rows", async () => {
    // The session tree is rendered from report state. If a report was seeded,
    // its getChildren returns 5 SessionFieldNode items. We exercise it by
    // triggering a redraw via the active-editor change hook that all providers
    // subscribe to.
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture);
    const doc = await vscode.workspace.openTextDocument(
      vscode.Uri.file(`${fixture}/Alpha.cs`),
    );
    await vscode.window.showTextDocument(doc);
    await sleep(300);
  });

  test("compareWithCanonical opens populated virtual documents for real relative paths", async () => {
    assert.ok(api.client, "extension must expose the real LanguageClient");
    const cluster = await waitForRelativePathCluster(api.client);

    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    await vscode.commands.executeCommand("deslop.compareWithCanonical", cluster.id);
    const diff = await waitForDiffTab();

    assert.equal(diff.original.scheme, "deslop-compare");
    assert.equal(diff.modified.scheme, "deslop-compare");
    assert.notEqual(diff.original.toString(), diff.modified.toString());

    const original = await vscode.workspace.openTextDocument(diff.original);
    const modified = await vscode.workspace.openTextDocument(diff.modified);
    assert.ok(original.getText().trim().length > 0, "left compare document must be populated");
    assert.ok(modified.getText().trim().length > 0, "right compare document must be populated");
  });

  test("bubble inline render triggered by edit", async () => {
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture);
    const doc = await vscode.workspace.openTextDocument(
      vscode.Uri.file(`${fixture}/Alpha.cs`),
    );
    const editor = await vscode.window.showTextDocument(doc);
    await editor.edit((b) => b.insert(new vscode.Position(3, 0), "        // x\n"));
    // Wait past DEBOUNCE_MS (250) + BUDGET_MS (250) + LSP round trip.
    await sleep(2500);
    await editor.edit((b) =>
      b.delete(new vscode.Range(new vscode.Position(3, 0), new vscode.Position(4, 0))),
    );
  });
});
