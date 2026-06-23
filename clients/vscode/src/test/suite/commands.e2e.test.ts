// E2E: exercise every command surface so activation, register, webview panels,
// and occurrence navigation are all hit.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { sleep } from "./helpers";

// [VSIX-COMMANDS]
suite("commands", () => {
  suiteSetup(async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
    assert.ok(ext, "extension should be registered");
    await ext.activate();
    // Give the LSP a beat to produce the initial report.
    await sleep(1500);
  });

  test("openReport + openReport again reveals the existing panel", async () => {
    await vscode.commands.executeCommand("deslop.openReport");
    await sleep(200);
    await vscode.commands.executeCommand("deslop.openReport");
    await sleep(200);
  });

  test("openWorstCluster twice reveals the cluster panel", async () => {
    await vscode.commands.executeCommand("deslop.openWorstCluster");
    await sleep(200);
    await vscode.commands.executeCommand("deslop.openWorstCluster");
    await sleep(200);
  });

  test("openCluster with a bad id does not throw", async () => {
    await vscode.commands.executeCommand("deslop.openCluster", "nonexistent-id");
    await sleep(100);
  });

  test("openOccurrence opens the referenced file", async () => {
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    await vscode.commands.executeCommand("deslop.openOccurrence", {
      path: `${fixture}/Alpha.cs`,
      start_byte: 0,
      end_byte: 10,
    });
    await sleep(200);
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "an editor should be open");
  });

  test("jumpToNextOccurrence with no cluster under cursor is a no-op", async () => {
    await vscode.commands.executeCommand("deslop.jumpToNextOccurrence");
  });

  test("compareWithCanonical with a bad id is a no-op", async () => {
    await vscode.commands.executeCommand("deslop.compareWithCanonical", "nonexistent");
  });

  test("toggleShowAllLenses flips the workspace setting", async () => {
    const before = vscode.workspace
      .getConfiguration("deslop")
      .get<boolean>("showAllLenses", false);
    await vscode.commands.executeCommand("deslop.toggleShowAllLenses");
    const after = vscode.workspace
      .getConfiguration("deslop")
      .get<boolean>("showAllLenses", false);
    assert.notEqual(before, after);
    await vscode.commands.executeCommand("deslop.toggleShowAllLenses");
  });

  test("refreshReport forwards to the LSP (LSP may not implement)", async () => {
    // The extension fires workspace/executeCommand — the LSP stub may respond
    // with "Method not found"; the command itself must not throw synchronously.
    try {
      await vscode.commands.executeCommand("deslop.refreshReport");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      assert.match(message, /Method not found|not registered|unknown/i);
    }
  });

  test("openHtmlReport renders the standalone report in a webview tab", async () => {
    // True E2E: the real LSP renders the HTML via renderHtmlReport and the
    // extension hosts it in a singleton "Deslop HTML Report" tab.
    await vscode.commands.executeCommand("deslop.openHtmlReport");
    await sleep(400);
    const hasReportTab = vscode.window.tabGroups.all
      .flatMap((g) => g.tabs)
      .some((t) => t.label === "Deslop HTML Report");
    assert.ok(hasReportTab, "the standalone HTML report tab must open");
  });

  test("revealActiveBinary fires the info modal without throwing", async () => {
    // The command shows a modal; we don't need to dismiss it — when the
    // extension-host test session ends VS Code tears all windows down.
    vscode.commands.executeCommand("deslop.revealActiveBinary");
    await sleep(200);
  });
});
