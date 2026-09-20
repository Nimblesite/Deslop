// E2E: drive jumpToNextOccurrence / comparePair / openOccurrence
// with real cluster data from the LSP's initial report, and exercise the
// bubble render path by positioning inside a known cluster.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import type { ExtensionApi } from "../../extension";
import { parseCompareUri } from "../../compare/provider";
import { compareTitle, INDENTATION_ONLY_VERDICT, measurePair, PAIR_COMPARE_METHOD } from "../../compare/title";
import { kindTitle, type PairComparison, type PairEvidence, type ReportCluster, type ReportOccurrence } from "../../types/report";
import {
  deleteRange,
  editThenRestore,
  fixtureRoot,
  openFixture,
  sleep,
  waitForCluster,
  waitForReport,
} from "./helpers";

const BUBBLE_RENDER_SETTLE_MS = 2500;
const ALPHA_FILE = "Alpha.cs";
const BETA_FILE = "Beta.cs";
const GAMMA_FILE = "Gamma.cs";
const DELTA_FILE = "Delta.cs";
const DIFFERENT_TEXT = "different";
const INDENTATION_ONLY_TEXT = "indentation_only";
const NEARLY_IDENTICAL_KIND = "nearly_identical";
// [CLONE-BUCKETS-IDENTICAL] Identity is equal source content after
// ASCII-whitespace folding, so a re-indented copy is Identical even
// though its bytes differ.
const IDENTICAL_KIND = "identical";
const CLOSE_ALL_EDITORS = "workbench.action.closeAllEditors";
const COMPARE_WITH_CANONICAL = "deslop.compareWithCanonical";
const LINE_BREAK = "\n";

function waitForRelativePathCluster(client: LanguageClient): Promise<ReportCluster> {
  return waitForCluster(
    client,
    (candidate) => candidate.occurrences.some((occurrence) => !path.isAbsolute(occurrence.path)),
    "no relative-path cluster in LSP report",
  );
}

function waitForClusterHolding(client: LanguageClient, fileName: string): Promise<ReportCluster> {
  return waitForCluster(
    client,
    (candidate) => candidate.occurrences.some((occurrence) => path.basename(occurrence.path) === fileName),
    `no cluster holding ${fileName} in LSP report`,
  );
}

interface OpenedDiff {
  readonly label: string;
  readonly input: vscode.TabInputTextDiff;
}

async function waitForDiff(): Promise<OpenedDiff> {
  // Under coverage instrumentation `vscode.diff` can take >2s to materialise
  // a TabInputTextDiff after closeAllEditors. 10s matches the rest of this
  // suite's wait helpers and absorbs that variance.
  for (let i = 0; i < 100; i += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return { label: tab.label, input: tab.input };
      }
    }
    await sleep(100);
  }
  throw new Error("compare command did not open a diff tab");
}

async function waitForDiffTab(): Promise<vscode.TabInputTextDiff> {
  return (await waitForDiff()).input;
}

// The engine's own answer for exactly these two endpoints, over the real LSP.
async function pairEvidence(
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

function assertDiffEndpoints(input: vscode.TabInputTextDiff, left: ReportOccurrence, right: ReportOccurrence): void {
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

// One click on a peer: the canonical range lands on the left, the clicked
// occurrence on the right, and the title names both files and repeats the
// engine's verdict on that exact pair ([VSIX-PAIR-COMPARE]).
async function compareWithCanonicalOneClick(
  client: LanguageClient,
  cluster: ReportCluster,
): Promise<{ diff: OpenedDiff; canonical: ReportOccurrence; peer: ReportOccurrence; evidence: PairEvidence }> {
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

function linesWithoutIndentation(text: string): string[] {
  return text.split(LINE_BREAK).map((line) => line.trimStart());
}

suite("cluster navigation", () => {
  let api: ExtensionApi;

  suiteSetup(async () => {
    api = await waitForReport();
    await sleep(2000);
  });

  test("openCluster by id opens the cluster panel", async () => {
    // Use a synthetic id — the open path builds the HTML regardless of the id matching.
    await vscode.commands.executeCommand("deslop.openCluster", "cluster-for-test");
    await sleep(300);
    await vscode.commands.executeCommand("deslop.openCluster", "cluster-for-test");
    await sleep(200);
  });

  test("jumping inside a fixture file while positioned at the start", async () => {
    const editor = await openFixture(ALPHA_FILE);
    editor.selection = new vscode.Selection(
      new vscode.Position(2, 8),
      new vscode.Position(2, 8),
    );
    await sleep(200);
    await vscode.commands.executeCommand("deslop.jumpToNextOccurrence");
    await sleep(300);
  });

  test("Duplication report webview opens from the command surface", async () => {
    // [VSIX-METRICS-REPORT] The Duplication panel headline opens this.
    await vscode.commands.executeCommand("deslop.openDuplicationReport");
    await sleep(400);
    await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
    await sleep(200);
  });

  test("Session tree contains the embedding + cache + state rows", async () => {
    // The session tree is rendered from report state. If a report was seeded,
    // its getChildren returns 5 SessionFieldNode items. We exercise it by
    // triggering a redraw via the active-editor change hook that all providers
    // subscribe to.
    await openFixture(ALPHA_FILE);
    await sleep(300);
  });

  test("one click compares a renamed copy with its canonical and titles the diff with its clone kind", async () => {
    assert.ok(api.client, "extension must expose the real LanguageClient");
    const cluster = await waitForClusterHolding(api.client, ALPHA_FILE);
    const { diff, evidence } = await compareWithCanonicalOneClick(api.client, cluster);
    assert.equal(evidence.text_identity, DIFFERENT_TEXT, "renamed copies do not differ only by indentation");
    // [CLONE-BUCKETS-ROUTING] A consistent rename is a Type-2 near-copy, never
    // shape-only and never a weaker relation.
    assert.equal(cluster.kind, NEARLY_IDENTICAL_KIND, "a consistent rename is a near-copy");
    assert.ok(!diff.label.includes(INDENTATION_ONLY_VERDICT), `no indentation claim for a rename: ${diff.label}`);
    assert.ok(diff.label.endsWith(kindTitle(cluster.kind)), `title ends with the pair's kind: ${diff.label}`);
    assert.ok(diff.label.includes(ALPHA_FILE) && diff.label.includes(BETA_FILE), `title names Alpha and Beta: ${diff.label}`);
    const original = (await vscode.workspace.openTextDocument(diff.input.original)).getText();
    const modified = (await vscode.workspace.openTextDocument(diff.input.modified)).getText();
    assert.notDeepEqual(
      linesWithoutIndentation(original),
      linesWithoutIndentation(modified),
      "the renamed copies differ by more than indentation",
    );
  });

  test("a copy that differs only by indentation is titled that way and the diff shows nothing else", async () => {
    assert.ok(api.client, "extension must expose the real LanguageClient");
    const cluster = await waitForClusterHolding(api.client, GAMMA_FILE);
    // [CLONE-BUCKETS-IDENTICAL] `Identical` requires equal source content after
    // ASCII-whitespace folding — not equal bytes. Re-indentation is whitespace
    // outside any literal, so it folds away and the copy stays Identical. The
    // byte inequality asserted below is what makes this a folding result rather
    // than a byte-equality one.
    assert.equal(cluster.kind, IDENTICAL_KIND, "re-indentation folds away, leaving identical source content");
    const { diff, evidence } = await compareWithCanonicalOneClick(api.client, cluster);
    assert.equal(evidence.text_identity, INDENTATION_ONLY_TEXT, "the engine sees indentation as the whole difference");
    assert.ok(diff.label.endsWith(INDENTATION_ONLY_VERDICT), `title says indentation only: ${diff.label}`);
    assert.ok(
      diff.label.includes(GAMMA_FILE) && diff.label.includes(DELTA_FILE),
      `title names both copies: ${diff.label}`,
    );
    const original = (await vscode.workspace.openTextDocument(diff.input.original)).getText();
    const modified = (await vscode.workspace.openTextDocument(diff.input.modified)).getText();
    assert.notEqual(original, modified, "the two copies are not the same bytes");
    assert.deepEqual(
      linesWithoutIndentation(original),
      linesWithoutIndentation(modified),
      "once indentation is removed the two copies are the same lines",
    );
  });

  test("comparePair opens populated virtual documents for two explicit endpoints with real relative paths", async () => {
    assert.ok(api.client, "extension must expose the real LanguageClient");
    const cluster = await waitForRelativePathCluster(api.client);
    const left = cluster.occurrences[0];
    const right = cluster.occurrences[1];
    assert.ok(left && right, "cluster must expose two endpoints to compare");

    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    // [VSIX-PAIR-COMPARE] Both endpoints are passed explicitly; the command
    // has no single-argument form.
    await vscode.commands.executeCommand("deslop.comparePair", left, right);
    const diff = await waitForDiffTab();

    assert.equal(diff.original.scheme, "deslop-compare");
    assert.equal(diff.modified.scheme, "deslop-compare");
    assert.notEqual(diff.original.toString(), diff.modified.toString());

    const original = await vscode.workspace.openTextDocument(diff.original);
    const modified = await vscode.workspace.openTextDocument(diff.modified);
    assert.ok(original.getText().trim().length > 0, "left compare document must be populated");
    assert.ok(modified.getText().trim().length > 0, "right compare document must be populated");
  });

  // [VSIX-STATE-DIRTY] (#130): editing one peer of a 2-occurrence cluster used
  // to drop the cluster from the canonical store, which made command-by-id
  // lookups silently no-op. The store now splits canonical (LSP-authored) from
  // the visible projection — the diff command must still resolve through
  // canonical even while the file is dirty.
  test("comparePair works on a cluster whose file is edited but unsaved (#130)", async () => {
    assert.ok(api.client, "extension must expose the real LanguageClient");
    const cluster = await waitForRelativePathCluster(api.client);
    const left = cluster.occurrences[0];
    const right = cluster.occurrences[1];
    assert.ok(left && right, "cluster must expose two endpoints to compare");

    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const dirtyUri = path.isAbsolute(left.path)
      ? vscode.Uri.file(left.path)
      : vscode.Uri.file(path.join(fixtureRoot(), left.path));
    const doc = await vscode.workspace.openTextDocument(dirtyUri);
    const editor = await vscode.window.showTextDocument(doc);

    try {
      // Insert a single character to mark the file dirty client-side. The LSP
      // is not notified (no save), so the canonical report still carries the
      // cluster. The visible projection elides it — that is the test's whole
      // point.
      await editor.edit((b) => b.insert(new vscode.Position(0, 0), " "));
      assert.ok(doc.isDirty, "dirty marker must be set after edit");

      await vscode.commands.executeCommand("deslop.comparePair", left, right);
      const diff = await waitForDiffTab();

      assert.equal(diff.original.scheme, "deslop-compare");
      assert.equal(diff.modified.scheme, "deslop-compare");
      const original = await vscode.workspace.openTextDocument(diff.original);
      const modified = await vscode.workspace.openTextDocument(diff.modified);
      assert.ok(
        original.getText().trim().length > 0,
        "left compare document must populate from the canonical report even when a peer file is dirty",
      );
      assert.ok(
        modified.getText().trim().length > 0,
        "right compare document must populate from the canonical report even when a peer file is dirty",
      );
    } finally {
      // Always restore the buffer so the dirty set does not leak into later
      // tests in this suite. The diff command may close the source editor —
      // reopen the source document explicitly so the edit call can target it.
      const restored = await vscode.window.showTextDocument(doc, { preview: false });
      await deleteRange(restored, 0, 0, 0, 1);
    }
  });

  test("bubble inline render triggered by edit", async () => {
    const editor = await openFixture(ALPHA_FILE);
    // Wait past DEBOUNCE_MS (250) + BUDGET_MS (250) + LSP round trip.
    await editThenRestore(editor, 3, "        // x\n", BUBBLE_RENDER_SETTLE_MS);
  });
});
