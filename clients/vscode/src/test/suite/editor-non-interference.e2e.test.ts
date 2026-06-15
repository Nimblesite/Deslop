// E2E: [LSP-NON-INTERFERENCE] — Deslop must never touch the editor's own
// Go To Definition. It registers no definition provider, so
// `vscode.executeDefinitionProvider` returns nothing from Deslop and F12
// stays entirely the editor's own language server.
//
// Regression guard for #231: a `definitionProvider` overload made F12 spin
// on large Flutter/Windows projects (VS Code waits for every provider, and
// Deslop blocked on its in-flight analysis). Driven against the real
// bundled LSP binary, per CLAUDE.md (no fake LSP).

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { sleep } from "./helpers";

async function codeLenses(uri: vscode.Uri): Promise<vscode.CodeLens[]> {
  return (
    (await vscode.commands.executeCommand<vscode.CodeLens[]>(
      "vscode.executeCodeLensProvider",
      uri,
    )) ?? []
  );
}

suite("editor non-interference (#231)", () => {
  let alpha: vscode.Uri;

  suiteSetup(async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
    assert.ok(ext, "extension must be installed");
    await ext.activate();
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    alpha = vscode.Uri.file(`${fixture}/Alpha.cs`);
    const doc = await vscode.workspace.openTextDocument(alpha);
    await vscode.window.showTextDocument(doc);
    // Wait until Deslop's additive code lens appears. This proves the LSP
    // is connected and contributing, so the empty-definition assertion is
    // meaningful rather than vacuously true.
    for (let attempt = 0; attempt < 40; attempt++) {
      if ((await codeLenses(alpha)).length > 0) return;
      await sleep(500);
    }
  });

  test("Deslop contributes its additive clone code lens (LSP is live)", async () => {
    const lenses = await codeLenses(alpha);
    assert.ok(
      lenses.length > 0,
      "Deslop's additive clone code lens must be present on a file with a duplicate",
    );
  });

  test("Deslop answers no Go To Definition — F12 stays the editor's own", async () => {
    // A position inside the duplicated method body of Alpha.cs.
    const position = new vscode.Position(8, 16);
    const definitions = await vscode.commands.executeCommand<
      (vscode.Location | vscode.LocationLink)[] | undefined
    >("vscode.executeDefinitionProvider", alpha, position);
    assert.ok(
      !definitions || definitions.length === 0,
      `Deslop must contribute no Go To Definition result; got ${JSON.stringify(definitions)}`,
    );
  });
});
