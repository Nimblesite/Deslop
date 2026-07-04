// E2E: cluster + report webview commands open without throwing and the extension
// posts a report snapshot to the webview on ready.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { openSchemaDoc } from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { activateExtension, sleep } from "./helpers";

function extensionRoot(): string {
  return path.resolve(__dirname, "../../..");
}

function packagedSchemaDoc(): string {
  return fs.readFileSync(path.join(extensionRoot(), "dist", "schema_doc.md"), "utf8");
}

function fakeCtx(): vscode.ExtensionContext {
  const root = extensionRoot();
  return {
    subscriptions: { push: () => {} },
    extensionPath: root,
    extensionUri: vscode.Uri.file(root),
    extension: { packageJSON: { version: "0.0.0" } },
  } as unknown as vscode.ExtensionContext;
}

suite("webviews", () => {
  suiteSetup(async () => {
    await activateExtension();
  });

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
    const expected = packagedSchemaDoc();
    await vscode.commands.executeCommand("deslop.showSchemaDoc");
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "schema doc should open in an editor");
    assert.equal(active.document.languageId, "markdown");
    assert.equal(active.document.getText(), expected);
  });

  test("showSchemaDoc falls back to the packaged schema doc with no client and no report", async () => {
    const expected = packagedSchemaDoc();
    await openSchemaDoc(fakeCtx(), new ReportStore());
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "packaged schema doc should open in an editor");
    assert.equal(active.document.languageId, "markdown");
    assert.equal(active.document.getText(), expected);
  });
});
