import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

// Polls `predicate` until it yields a value, so suites wait on an observable
// condition (a report landing, a generation advancing) rather than a fixed
// delay. Shared by every suite that needs the live pipeline to settle.
export async function waitFor<T>(
  predicate: () => T | undefined,
  timeoutMs: number,
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = predicate();
    if (value !== undefined) return value;
    await sleep(100);
  }
  throw new Error(`waitFor timed out after ${timeoutMs}ms`);
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
