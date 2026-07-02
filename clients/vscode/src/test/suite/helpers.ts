import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

// Every E2E suite must await activation before executing deslop.* commands.
// The extension activates on onStartupFinished, which races the mocha runner;
// awaiting activate() here makes each suite deterministic regardless of that
// timing. Returns the resolved ExtensionApi so callers can drive it directly.
export async function activateExtension(): Promise<ExtensionApi> {
  const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
  assert.ok(ext, "extension must be installed");
  return (await ext.activate()) as ExtensionApi;
}
