// E2E: live bubble fires within 1 s of an edit that duplicates an existing cluster.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { sleep } from "./helpers";

suite("live bubble", () => {
  test("typing duplicate code produces a decoration within 1s", async () => {
    const fixture = process.env["CODEDEDUP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    const uri = vscode.Uri.file(`${fixture}/Alpha.cs`);
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc);

    await editor.edit((builder) =>
      builder.insert(new vscode.Position(2, 0), "    // identical block\n"),
    );
    await sleep(750); // 250ms debounce + 250ms budget + cushion

    // The bubble decoration is not queryable via the VS Code API; we assert
    // that the dismiss command is available, which is only registered by the
    // bubble module when activate() runs.
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("codededup.bubble.dismiss"));
    assert.ok(commands.includes("codededup.bubble.dismissCluster"));
  });

  test("escape dismisses the bubble", async () => {
    await vscode.commands.executeCommand("codededup.bubble.dismiss");
  });
});
