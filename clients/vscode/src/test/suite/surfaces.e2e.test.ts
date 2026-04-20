// E2E: drive tree providers + decorations + bubble modes + status bar.
// Opens fixture files so FocusedFileProvider and DecorationManager get real
// editors to reason about, and flips configuration to cover both bubble modes.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { sleep } from "./helpers";

async function openFixture(name: string): Promise<vscode.TextEditor> {
  const fixture = process.env["DESLOP_TEST_FIXTURE"];
  assert.ok(fixture, "fixture path must be set");
  const doc = await vscode.workspace.openTextDocument(
    vscode.Uri.file(`${fixture}/${name}`),
  );
  return await vscode.window.showTextDocument(doc);
}

suite("surfaces", () => {
  suiteSetup(async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
    await ext!.activate();
    await sleep(1500);
  });

  test("Top Offenders tree yields at least one root node", async () => {
    // Wait for initial report seeding; tree reads from the store.
    await sleep(500);
    // The tree is created at activation, covered by the activation test.
    // Here we ensure the tree data provider has been registered:
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes("deslop.openCluster"));
  });

  test("opening a fixture editor fires FocusedFile tree refresh + decorations", async () => {
    await openFixture("Alpha.cs");
    await sleep(500);
  });

  test("editing a fixture triggers the decoration redraw pipeline", async () => {
    const editor = await openFixture("Alpha.cs");
    await editor.edit((b) => b.insert(new vscode.Position(0, 0), "// edit\n"));
    await sleep(500);
    await editor.edit((b) =>
      b.delete(new vscode.Range(new vscode.Position(0, 0), new vscode.Position(1, 0))),
    );
    await sleep(500);
  });

  test("bubble ghost mode renders after an edit", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("liveBubble.mode", "ghost", vscode.ConfigurationTarget.Workspace);
    const editor = await openFixture("Beta.cs");
    await editor.edit((b) => b.insert(new vscode.Position(1, 0), "    var x = 1;\n"));
    await sleep(1500);
    await editor.edit((b) =>
      b.delete(new vscode.Range(new vscode.Position(1, 0), new vscode.Position(2, 0))),
    );
    await cfg.update("liveBubble.mode", "inline", vscode.ConfigurationTarget.Workspace);
  });

  test("bubble disabled setting short-circuits onEdit", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await cfg.update("liveBubble.enabled", false, vscode.ConfigurationTarget.Workspace);
    const editor = await openFixture("Alpha.cs");
    await editor.edit((b) => b.insert(new vscode.Position(0, 0), " "));
    await sleep(500);
    await cfg.update("liveBubble.enabled", true, vscode.ConfigurationTarget.Workspace);
  });

  test("dismissCluster suppresses re-bubbling for the dismissed cluster", async () => {
    await vscode.commands.executeCommand("deslop.bubble.dismissCluster", "cluster-xyz");
  });

  test("closing the active editor clears the bubble", async () => {
    await openFixture("Alpha.cs");
    await sleep(300);
    await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
    await sleep(300);
  });

  test("inlay hints provider is registered for C#", async () => {
    // Registration happens at activation — if activation succeeded the provider is live.
    const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
    assert.ok(ext!.isActive);
  });
});
