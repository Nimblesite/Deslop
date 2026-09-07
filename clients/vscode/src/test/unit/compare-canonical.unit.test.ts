// [VSIX-PAIR-COMPARE] Canonical comparison preserves the selected occurrence,
// including a third range in the same file and the canonical report during edits.
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as vscode from "vscode";

import { compareWithCanonicalTarget } from "../../commands/register";
import { parseCompareUri } from "../../compare/provider";
import { ReportStore } from "../../reportStore";
import { ClusterNode, OccurrenceNode, StatusTicker, TopOffendersProvider } from "../../tree/providers";
import type { ReportCluster, ReportOccurrence } from "../../types/report";
import { occurrence, wireCluster } from "../cluster.helpers";
import { activateExtension } from "../suite/helpers";
import { reportWithClusters } from "./report.helpers";
import { tempFile } from "./temp-file.helpers";

const FIXTURE_PREFIX = "deslop-canonical-";
const FIXTURE_FILENAME = "ranges.ts";
const CLUSTER_ID = "canonical-comparison";
const UNKNOWN_CLUSTER_ID = "unknown-cluster";
const CLOSE_EDITORS_COMMAND = "workbench.action.closeAllEditors";
const UTF8_ENCODING = "utf8";
const CANONICAL_INDEX = 0;
const FIRST_PEER_INDEX = 1;
const THIRD_OCCURRENCE_INDEX = 2;
const ONE_DIFF = 1;
const NO_DIFFS = 0;
const DIRTY_SOURCE_PATH = "dirty-canonical.ts";
const CANONICAL_CONTEXT = "deslop.occurrenceCanonical";
const PEER_CONTEXT = "deslop.occurrence";
const SOURCE_PARTS = ["const canonical = 1;\n", "const firstPeer = 2;\n", "const selectedPeer = 3;\n"];

interface CompareFixture {
  readonly store: ReportStore;
  readonly cluster: ReportCluster;
  readonly file: string;
}

function fixtureCluster(file: string): ReportCluster {
  let start = CANONICAL_INDEX;
  const occurrences = SOURCE_PARTS.map((text) => {
    const end = start + Buffer.byteLength(text, UTF8_ENCODING);
    const result = occurrence(file, start, end);
    start = end;
    return result;
  });
  return wireCluster({ id: CLUSTER_ID, occurrences });
}

async function withCompareFixture(run: (fixture: CompareFixture) => Promise<void>): Promise<void> {
  const { dir, file } = tempFile(FIXTURE_PREFIX, FIXTURE_FILENAME);
  fs.writeFileSync(file, SOURCE_PARTS.join(""), UTF8_ENCODING);
  const cluster = fixtureCluster(file);
  const store = new ReportStore();
  store.setSnapshot(reportWithClusters([cluster]), CANONICAL_INDEX);
  try {
    await closeEditors();
    await run({ store, cluster, file });
  } finally {
    await closeEditors();
    store.dispose();
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

async function closeEditors(): Promise<void> {
  await vscode.commands.executeCommand(CLOSE_EDITORS_COMMAND);
}

function diffTabs(): vscode.TabInputTextDiff[] {
  return vscode.window.tabGroups.all.flatMap((group) => group.tabs.map((tab) => tab.input))
    .filter((input): input is vscode.TabInputTextDiff => input instanceof vscode.TabInputTextDiff);
}

function assertTreeContexts(store: ReportStore, cluster: ReportCluster, expected: readonly string[]): void {
  const ticker = new StatusTicker();
  const provider = new TopOffendersProvider(store, ticker);
  try {
    const rows = provider.getChildren(new ClusterNode(cluster));
    assert.deepEqual(rows.map((row) => row.contextValue), expected);
    assert.deepEqual(rows.map((row) => (row as OccurrenceNode).occurrence), cluster.occurrences);
  } finally {
    provider.dispose();
    ticker.dispose();
  }
}

async function assertComparedRange(selected: ReportOccurrence, selectedIndex: number): Promise<void> {
  const tabs = diffTabs();
  assert.equal(tabs.length, ONE_DIFF, "comparison must open exactly one native diff");
  const diff = tabs[CANONICAL_INDEX];
  assert.ok(diff);
  const left = parseCompareUri(diff.original);
  const right = parseCompareUri(diff.modified);
  assert.equal(left.sourcePath, selected.path);
  assert.equal(left.startByte, CANONICAL_INDEX);
  assert.equal(right.sourcePath, selected.path);
  assert.equal(right.startByte, selected.start_byte);
  assert.equal(right.endByte, selected.end_byte);
  assert.notEqual(diff.original.toString(), diff.modified.toString());
  assert.equal((await vscode.workspace.openTextDocument(diff.original)).getText(), SOURCE_PARTS[CANONICAL_INDEX]);
  assert.equal((await vscode.workspace.openTextDocument(diff.modified)).getText(), SOURCE_PARTS[selectedIndex]);
}

suite("canonical comparison", () => {
  suiteSetup(async () => { await activateExtension(); });

  test("tree peers retain their comparison actions when the original canonical becomes dirty", () => {
    const source = fixtureCluster(FIXTURE_FILENAME);
    const [canonical, ...peers] = source.occurrences;
    assert.ok(canonical);
    const cluster = { ...source, occurrences: [{ ...canonical, path: DIRTY_SOURCE_PATH }, ...peers] };
    const store = new ReportStore();
    store.setSnapshot(reportWithClusters([cluster]), CANONICAL_INDEX);
    assertTreeContexts(store, cluster, [CANONICAL_CONTEXT, PEER_CONTEXT, PEER_CONTEXT]);
    store.markFileDirty(DIRTY_SOURCE_PATH);
    const projected = store.current.visibleReport?.clusters[CANONICAL_INDEX];
    assert.ok(projected);
    assert.deepEqual(projected.occurrences, peers);
    assertTreeContexts(store, projected, [PEER_CONTEXT, PEER_CONTEXT]);
    store.clearFileDirty(DIRTY_SOURCE_PATH);
    assertTreeContexts(store, cluster, [CANONICAL_CONTEXT, PEER_CONTEXT, PEER_CONTEXT]);
    store.dispose();
  });

  test("tree and cluster targets preserve the peer range before and during an unsaved edit", async () => {
    await withCompareFixture(async ({ store, cluster, file }) => {
      const selected = cluster.occurrences[THIRD_OCCURRENCE_INDEX];
      const firstPeer = cluster.occurrences[FIRST_PEER_INDEX];
      assert.ok(selected && firstPeer);
      await compareWithCanonicalTarget(store, new OccurrenceNode(selected));
      await assertComparedRange(selected, THIRD_OCCURRENCE_INDEX);
      await closeEditors();
      await compareWithCanonicalTarget(store, new ClusterNode(cluster));
      await assertComparedRange(firstPeer, FIRST_PEER_INDEX);
      await closeEditors();
      store.markFileDirty(file);
      await compareWithCanonicalTarget(store, cluster.id, selected);
      await assertComparedRange(selected, THIRD_OCCURRENCE_INDEX);
    });
  });

  test("canonical, missing and foreign targets never substitute another peer", async () => {
    await withCompareFixture(async ({ store, cluster }) => {
      const canonical = cluster.occurrences[CANONICAL_INDEX];
      assert.ok(canonical);
      const requests: readonly [unknown, unknown?][] = [
        [new OccurrenceNode(canonical)], [UNKNOWN_CLUSTER_ID], [undefined],
        [cluster.id, canonical], [cluster.id, UNKNOWN_CLUSTER_ID],
        [cluster.id, { ...canonical, path: UNKNOWN_CLUSTER_ID }],
      ];
      for (const [target, selected] of requests) {
        await compareWithCanonicalTarget(store, target, selected);
        assert.equal(diffTabs().length, NO_DIFFS, "a rejected target must leave the editor unchanged");
      }
    });
  });
});
