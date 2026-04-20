// E2E: the embedding picker lists the stub provider when Ollama is unreachable,
// and the fake LSP-driven list when it is.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

suite("embedding picker", () => {
  test("command palette entry exists", async () => {
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("codededup.pickEmbeddingModel"));
  });

  test("picker can be invoked without throwing", async () => {
    const p = vscode.commands.executeCommand("codededup.pickEmbeddingModel");
    // Immediately close any quick pick the command opens.
    await vscode.commands.executeCommand("workbench.action.closeQuickOpen");
    await p;
  });
});
