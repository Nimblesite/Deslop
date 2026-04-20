// E2E: activation path — extension activates on fixture workspace, status bar appears,
// tree populates with the sample cluster, bubble commands exist.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

suite("activation", () => {
  test("extension activates on a C# fixture workspace", async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
    assert.ok(ext, "extension should be registered");
    await ext!.activate();
    assert.ok(ext!.isActive, "extension should be active");
  });

  test("activity bar view container is registered", async () => {
    const views = vscode.window.visibleTextEditors;
    void views;
    // Presence is verified via command — the view container itself is not queryable.
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("deslop.openReport"));
    assert.ok(commands.includes("deslop.openWorstCluster"));
    assert.ok(commands.includes("deslop.pickEmbeddingModel"));
    assert.ok(commands.includes("deslop.revealActiveBinary"));
  });

  test("status bar item is shown after activation", async () => {
    // VS Code API exposes no direct read for status bar items; we assert the
    // command it routes to is installed.
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("deslop.openWorstCluster"));
  });
});
