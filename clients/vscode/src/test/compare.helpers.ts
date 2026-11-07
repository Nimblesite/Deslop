// [VSIX-PAIR-COMPARE] Shared drivers for the native compare diff: open it the
// way a user does, read back the tab VS Code shows, and check it against the
// engine's own answer for the same two endpoints over the real LSP.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { parseCompareUri } from "../compare/provider";
import { compareTitle, measurePair, PAIR_COMPARE_METHOD } from "../compare/title";
import type { PairComparison, PairEvidence, ReportCluster, ReportOccurrence } from "../types/report";
import { sleep } from "./suite/helpers";

/** The custom URI scheme both sides of every compare diff live under. */
export const COMPARE_SCHEME = "deslop-compare";
export const CLOSE_ALL_EDITORS = "workbench.action.closeAllEditors";
export const COMPARE_WITH_CANONICAL = "deslop.compareWithCanonical";
export const COMPARE_PAIR = "deslop.comparePair";
const LINE_BREAK = "\n";
// Under coverage instrumentation `vscode.diff` can take >2s to materialise a
// TabInputTextDiff after closeAllEditors; 100 × 100ms absorbs that variance.
const DIFF_POLL_ATTEMPTS = 100;
const DIFF_POLL_INTERVAL_MS = 100;

/** The diff tab a compare command opened: its title and both URIs. */
export interface OpenedDiff {
  readonly label: string;
  readonly input: vscode.TabInputTextDiff;
}

/** Both documents of an opened diff, as VS Code renders them. */
export interface DiffDocuments {
  readonly original: string;
  readonly modified: string;
}

/** What one-click comparison opened and what the engine says about it. */
export interface CanonicalComparison {
  readonly diff: OpenedDiff;
  readonly canonical: ReportOccurrence;
  readonly peer: ReportOccurrence;
  readonly evidence: PairEvidence;
}

export async function waitForDiff(): Promise<OpenedDiff> {
  for (let attempt = 0; attempt < DIFF_POLL_ATTEMPTS; attempt += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return { label: tab.label, input: tab.input };
      }
    }
    await sleep(DIFF_POLL_INTERVAL_MS);
  }
  throw new Error("compare command did not open a diff tab");
}

export async function waitForDiffTab(): Promise<vscode.TabInputTextDiff> {
  return (await waitForDiff()).input;
}

/** The text of both sides of `input`. */
export async function diffDocuments(input: vscode.TabInputTextDiff): Promise<DiffDocuments> {
  const original = (await vscode.workspace.openTextDocument(input.original)).getText();
  const modified = (await vscode.workspace.openTextDocument(input.modified)).getText();
  return { original, modified };
}

/** The engine's own answer for exactly these two endpoints, over the real LSP. */
export async function pairEvidence(
  client: LanguageClient,
  left: ReportOccurrence,
  right: ReportOccurrence,
): Promise<PairEvidence> {
  const comparison = await client.sendRequest<PairComparison>(PAIR_COMPARE_METHOD, {
    left: { path: left.path, start_byte: left.start_byte, end_byte: left.end_byte },
    right: { path: right.path, start_byte: right.start_byte, end_byte: right.end_byte },
  });
  const measured = await measurePair(client, left, right);
  assert.deepEqual(measured, comparison.evidence, "measurePair preserves the real engine's evidence for these exact endpoints");
  return comparison.evidence;
}

export function assertDiffEndpoints(input: vscode.TabInputTextDiff, left: ReportOccurrence, right: ReportOccurrence): void {
  assert.equal(input.original.scheme, COMPARE_SCHEME);
  assert.equal(input.modified.scheme, COMPARE_SCHEME);
  const original = parseCompareUri(input.original);
  const modified = parseCompareUri(input.modified);
  assert.equal(original.sourcePath, left.path);
  assert.equal(original.startByte, left.start_byte);
  assert.equal(original.endByte, left.end_byte);
  assert.equal(modified.sourcePath, right.path);
  assert.equal(modified.startByte, right.start_byte);
  assert.equal(modified.endByte, right.end_byte);
  assert.notEqual(input.original.toString(), input.modified.toString());
}

/** Opens `left` vs `right` through `deslop.comparePair`, checks the endpoints and returns the diff. */
export async function comparePairExplicitly(left: ReportOccurrence, right: ReportOccurrence): Promise<OpenedDiff> {
  await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
  await vscode.commands.executeCommand(COMPARE_PAIR, left, right);
  const diff = await waitForDiff();
  assertDiffEndpoints(diff.input, left, right);
  return diff;
}

// One click on a peer: the canonical range lands on the left, the clicked
// occurrence on the right, and the title names both files and repeats the
// engine's verdict on that exact pair ([VSIX-PAIR-COMPARE]).
export async function compareWithCanonicalOneClick(
  client: LanguageClient,
  cluster: ReportCluster,
): Promise<CanonicalComparison> {
  const [canonical, peer] = cluster.occurrences;
  assert.ok(canonical && peer, "cluster must expose a canonical occurrence and a peer");
  await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
  await vscode.commands.executeCommand(COMPARE_WITH_CANONICAL, cluster.id, peer);
  const diff = await waitForDiff();
  assertDiffEndpoints(diff.input, canonical, peer);
  const evidence = await pairEvidence(client, canonical, peer);
  assert.equal(diff.label, compareTitle(canonical, peer, evidence), "the title is the engine's verdict on this pair");
  assert.ok(diff.label.includes(path.basename(canonical.path)), `title names the canonical file: ${diff.label}`);
  assert.ok(diff.label.includes(path.basename(peer.path)), `title names the peer file: ${diff.label}`);
  return { diff, canonical, peer, evidence };
}

export function linesWithoutIndentation(text: string): string[] {
  return text.split(LINE_BREAK).map((line) => line.trimStart());
}
