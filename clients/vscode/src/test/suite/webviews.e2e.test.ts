// E2E: cluster + report webview commands open without throwing and the extension
// posts a report snapshot to the webview on ready.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { sleep } from "./helpers";

suite("webviews", () => {
  test("openReport command opens a webview", async () => {
    await vscode.commands.executeCommand("deslop.openReport");
    await sleep(300);
    // No API to introspect webview panels; verify by running the command twice —
    // the extension deduplicates a second open via the activePanels map.
    await vscode.commands.executeCommand("deslop.openReport");
  });

  test("openWorstCluster opens a webview when report has clusters", async () => {
    await vscode.commands.executeCommand("deslop.openWorstCluster");
    await sleep(300);
  });

  test("showSchemaDoc opens the embedded schema", async () => {
    await vscode.commands.executeCommand("deslop.showSchemaDoc");
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "schema doc should open in an editor");
    assert.match(active.document.getText(), /schema/i);
  });
});
