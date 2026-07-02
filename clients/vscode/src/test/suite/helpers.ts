import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

// Every E2E suite must await activation before executing deslop.* commands.
// The extension activates on onStartupFinished, which races the mocha runner:
// the single esbuild bundle usually wins that race, the multi-module out/
// tree (used by the coverage run — [VSIX-TESTING-COVERAGE]) does not.
export async function activateExtension(): Promise<ExtensionApi> {
  const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
  assert.ok(ext, "extension must be installed");
  return (await ext.activate()) as ExtensionApi;
}
