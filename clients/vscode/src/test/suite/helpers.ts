import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import type { ExtensionApi } from "../../extension";
import type { Report, ReportCluster } from "../../types/report";

// The command the extension registers only once the LSP has served a
// report — its presence is the observable "the live pipeline is up".
const OPEN_CLUSTER_COMMAND = "deslop.openCluster";
const REPORT_GET_METHOD = "deslop/reportGet";
// A cluster needs two occurrences before it can be navigated or compared.
const MULTI_OCCURRENCE_MINIMUM = 2;
const POLL_INTERVAL_MS = 250;
const ACTIVATION_POLL_ATTEMPTS = 20;
const REPORT_POLL_ATTEMPTS = 40;

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

// Awaits activation *and* the first report. Initial report seeding takes
// time over stdio, so a suite that only awaits activation races the
// engine and sees an empty report.
export async function waitForReport(): Promise<ExtensionApi> {
  const api = await activateExtension();
  for (let i = 0; i < ACTIVATION_POLL_ATTEMPTS; i += 1) {
    await sleep(POLL_INTERVAL_MS);
    const cmds = await vscode.commands.getCommands(true);
    if (cmds.includes(OPEN_CLUSTER_COMMAND)) return api;
  }
  throw new Error("extension did not activate in time");
}

// Polls the LSP's own report until a multi-occurrence cluster satisfies
// `accept`, so suites assert against real engine output rather than
// staged data. `failure` names what was being waited for.
export async function waitForCluster(
  client: LanguageClient,
  accept: (candidate: ReportCluster) => boolean,
  failure: string,
): Promise<ReportCluster> {
  let last: Report | undefined;
  for (let i = 0; i < REPORT_POLL_ATTEMPTS; i += 1) {
    last = await client.sendRequest<Report>(REPORT_GET_METHOD);
    const cluster = last.clusters.find(
      (candidate) =>
        candidate.occurrences.length >= MULTI_OCCURRENCE_MINIMUM && accept(candidate),
    );
    if (cluster) return cluster;
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(`${failure}; last cluster count ${last?.clusters.length ?? 0}`);
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

/** The workspace the E2E suites run against — the csharp-small fixture the
 * runner opens. Every suite used to read the env var and assert it itself. */
export function fixtureRoot(): string {
  const fixture = process.env["DESLOP_TEST_FIXTURE"];
  assert.ok(fixture, "fixture path must be set");
  return fixture;
}

/** The URI of `name` inside the fixture workspace. */
export function fixtureUri(name: string): vscode.Uri {
  return vscode.Uri.file(`${fixtureRoot()}/${name}`);
}

/** Opens a fixture file and shows it, returning the editor the suite drives. */
export async function openFixture(name: string): Promise<vscode.TextEditor> {
  const doc = await vscode.workspace.openTextDocument(fixtureUri(name));
  return await vscode.window.showTextDocument(doc);
}

/** Deletes a range from `editor` — the undo half of every "edit, then put the
 * buffer back" step, so a suite cannot leak a dirty file into the next one. */
export function deleteRange(
  editor: vscode.TextEditor,
  startLine: number,
  startChar: number,
  endLine: number,
  endChar: number,
): Thenable<boolean> {
  return editor.edit((builder) =>
    builder.delete(
      new vscode.Range(
        new vscode.Position(startLine, startChar),
        new vscode.Position(endLine, endChar),
      ),
    ),
  );
}

/** Inserts a whole line of `text` at `line`, waits `settleMs` for the live
 * pipeline to react, then removes exactly that line so the buffer ends
 * byte-identical to how it started. */
export async function editThenRestore(
  editor: vscode.TextEditor,
  line: number,
  text: string,
  settleMs: number,
): Promise<void> {
  await editor.edit((builder) => builder.insert(new vscode.Position(line, 0), text));
  await sleep(settleMs);
  await deleteRange(editor, line, 0, line + 1, 0);
}
