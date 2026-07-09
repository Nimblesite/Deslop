// E2E (issue #201): activation in an EMPTY window — no workspace folder.
//
// The showstopper: opening VS Code with no folder crash-looped the LSP.
// vscode-languageclient appends its `--stdio` transport flag, the Rust binary
// read that flag as the workspace root, and the file watcher died on the bogus
// path ("The Deslop server crashed 5 times"). The fix is two moves: (1)
// startLanguageClient returns undefined when there is no root, and (2)
// activate() guards `client.start()` behind `if (client)`, settling the
// lifecycle to "ready" in the else branch.
//
// Every other suite runs against the csharp fixture (a folder is always open),
// so only THIS no-folder launch config exercises the guard and the else branch
// — the load-bearing half of the fix. Without this test, a refactor that drops
// the `if (client)` wrapper would throw `TypeError: Cannot read properties of
// undefined (reading 'start')` during activation — #201 again — while every
// other test stayed green.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { ExtensionApi, resolveWorkspaceRoot } from "../../extension";

suite("activation without a workspace folder (issue #201)", () => {
  test("activate() does not launch the LSP and settles into a ready idle state", async () => {
    // Precondition: this launch config must open an empty window. If a folder
    // leaked in, the test would silently exercise the happy path instead.
    assert.ok(
      !vscode.workspace.workspaceFolders ||
        vscode.workspace.workspaceFolders.length === 0,
      "the no-folder launch config must open an empty window",
    );
    assert.equal(
      resolveWorkspaceRoot(),
      undefined,
      "no folder ⇒ resolveWorkspaceRoot() is undefined — the top of the #201 chain",
    );

    const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
    assert.ok(ext, "extension must be installed");

    // Must NOT throw. Pre-fix this either built a rootless stdio client that
    // crash-looped, or (after a bad refactor) threw on `undefined.start()`.
    const api = (await ext.activate()) as ExtensionApi;
    assert.ok(ext.isActive, "extension must activate cleanly with no folder open");

    assert.equal(
      api.client,
      undefined,
      "no folder ⇒ the LSP client is never constructed or started (#201)",
    );
    assert.equal(
      api.reportStore?.current.lifecycle.kind,
      "ready",
      "no folder ⇒ lifecycle settles to a ready idle state, not a perpetual scan spinner",
    );
  });
});
