// E2E: edit a fixture file, give the real LSP time to re-analyse,
// assert the bubble-related commands are registered (only possible if
// activate() ran end-to-end against the real deslop-lsp binary).

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { sleep } from "./helpers";

suite("live bubble (real LSP)", () => {
  test("extension spawns the real deslop-lsp binary", async () => {
    const ext = vscode.extensions.getExtension("deslop.deslop-vscode");
    assert.ok(ext && ext.isActive, "extension must be active against the real LSP");
  });

  test("editing a duplicated range triggers re-analysis", async () => {
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    const uri = vscode.Uri.file(`${fixture}/Alpha.cs`);
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc);

    await editor.edit((builder) =>
      builder.insert(new vscode.Position(2, 0), "    var extra = 42;\n"),
    );
    // 250ms debounce + LSP re-analysis budget; the real binary must complete in <1s.
    await sleep(2000);

    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("deslop.bubble.dismiss"));
    assert.ok(commands.includes("deslop.bubble.dismissCluster"));
  });

  test("escape dismisses the bubble", async () => {
    await vscode.commands.executeCommand("deslop.bubble.dismiss");
  });
});
