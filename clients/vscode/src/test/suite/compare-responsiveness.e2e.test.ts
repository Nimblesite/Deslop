// [VSIX-PAIR-COMPARE] #607 #608: a compare opens its diff the moment the user
// asks for it. The engine's verdict only labels the diff, so it may never hold
// the diff back, and a report that moved on since the panel was drawn may never
// turn the click into a silent no-op.
//
// Drives the real extension against the real deslop-lsp. The real client's
// `deslop/pairCompare` replies are held — the engine still answers, its answer
// is just not delivered yet — which is exactly what a user sees on a large
// workspace while the engine measures the pair. Every peer of every fixture
// cluster, every way a command is targeted, and both compare routes.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";
import { ClusterNode, OccurrenceNode } from "../../tree/nodes";
import type { ReportCluster, ReportOccurrence } from "../../types/report";
import {
  assertDiffEndpoints,
  CLOSE_ALL_EDITORS,
  COMPARE_PAIR,
  COMPARE_WITH_CANONICAL,
  diffDocuments,
  waitForDiff,
} from "../compare.helpers";
import { diffOpenedWhileVerdictPending, holdPairVerdicts, openDiffTabs, ONE_VERDICT } from "../verdict-hold.helpers";
import { waitForClusterHolding, waitForReport } from "./helpers";

/** The fixture clusters: a renamed copy (Alpha/Beta) and a re-indented copy (Gamma/Delta). */
const FIXTURE_FILES = ["Alpha.cs", "Gamma.cs"] as const;
/** Cluster ids no report holds: what the panel captured before the report moved on. */
const SUPERSEDED_CLUSTER_IDS = ["0000000000000000", "ffffffffffffffff", "stale-id"] as const;
const CANONICAL_INDEX = 0;
const FIRST_PEER_INDEX = 1;
const ONE_DIFF_TAB = 1;
const REPEATED_CLICKS = 3;

/** Every peer of `cluster`, by position. */
function peersOf(cluster: ReportCluster): Array<{ index: number; peer: ReportOccurrence }> {
  return cluster.occurrences.slice(FIRST_PEER_INDEX).map((peer, offset) => ({ index: offset + FIRST_PEER_INDEX, peer }));
}

/** The canonical of `cluster`, asserted present. */
function canonicalOf(cluster: ReportCluster): ReportOccurrence {
  const canonical = cluster.occurrences[CANONICAL_INDEX];
  assert.ok(canonical, `cluster ${cluster.id} has a canonical occurrence`);
  return canonical;
}

/** The first peer of `cluster`, asserted present. */
function firstPeerOf(cluster: ReportCluster): ReportOccurrence {
  const peer = cluster.occurrences[FIRST_PEER_INDEX];
  assert.ok(peer, `cluster ${cluster.id} has a peer`);
  return peer;
}

/** Both sides of the diff hold the source text of their endpoints, not blanks. */
async function assertPopulated(diff: vscode.TabInputTextDiff, left: ReportOccurrence, right: ReportOccurrence): Promise<void> {
  const { original, modified } = await diffDocuments(diff);
  assert.ok(original.trim().length > 0, `the left side shows ${left.path}`);
  assert.ok(modified.trim().length > 0, `the right side shows ${right.path}`);
  const expectedLeft = Buffer.from(await vscode.workspace.fs.readFile(vscode.Uri.file(absolute(left.path))))
    .subarray(left.start_byte, left.end_byte).toString("utf8");
  assert.equal(original, expectedLeft, "the left side is exactly the left endpoint's bytes");
}

function absolute(occurrencePath: string): string {
  if (path.isAbsolute(occurrencePath)) return occurrencePath;
  const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? "";
  return path.join(folder, occurrencePath);
}

suite("compare responsiveness (#607 #608)", () => {
  let api: ExtensionApi;
  const clusters: ReportCluster[] = [];

  suiteSetup(async () => {
    api = await waitForReport();
    assert.ok(api.client, "extension must expose the real LanguageClient");
    for (const file of FIXTURE_FILES) clusters.push(await waitForClusterHolding(api.client, file));
    for (const cluster of clusters) assert.ok(cluster.occurrences.length >= 2, `${cluster.id} has a canonical and a peer`);
    assert.notEqual(clusters[0]?.id, clusters[1]?.id, "the fixture clusters are distinct");
  });

  test("Compare To Canonical opens every peer's diff while the engine's verdict is still pending (#607)", async () => {
    assert.ok(api.client);
    for (const cluster of clusters) {
      const canonical = canonicalOf(cluster);
      for (const { index, peer } of peersOf(cluster)) {
        const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_WITH_CANONICAL, cluster.id, peer);
        assertDiffEndpoints(diff, canonical, peer);
        await assertPopulated(diff, canonical, peer);
        assert.equal(openDiffTabs(), ONE_DIFF_TAB, `cluster ${cluster.id} peer ${index}: exactly one diff opened`);
      }
    }
  });

  test("Compare To Canonical opens while pending from every command target the UI sends (#607)", async () => {
    assert.ok(api.client);
    for (const cluster of clusters) {
      const canonical = canonicalOf(cluster);
      const peer = firstPeerOf(cluster);
      const targets: Array<[string, unknown[]]> = [
        ["cluster id + row (cluster panel button)", [cluster.id, peer]],
        ["tree occurrence row", [new OccurrenceNode(peer, cluster)]],
        ["bare occurrence (code lens / hover)", [peer]],
        ["tree cluster row (first peer)", [new ClusterNode(cluster)]],
      ];
      for (const [how, args] of targets) {
        const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_WITH_CANONICAL, ...args);
        assertDiffEndpoints(diff, canonical, peer);
        assert.equal(openDiffTabs(), ONE_DIFF_TAB, `${how}: exactly one diff opened`);
      }
    }
  });

  test("Compare with Selected opens the diff of both picked rows, in either order, while pending (#608)", async () => {
    assert.ok(api.client);
    for (const cluster of clusters) {
      for (const left of cluster.occurrences) {
        for (const right of cluster.occurrences) {
          if (left === right) continue;
          const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_PAIR, left, right);
          assertDiffEndpoints(diff, left, right);
          await assertPopulated(diff, left, right);
          assert.equal(openDiffTabs(), ONE_DIFF_TAB, "exactly one diff opened");
        }
      }
    }
  });

  test("Compare with Selected across two different clusters opens while pending (#608)", async () => {
    assert.ok(api.client);
    const [first, second] = clusters.map(canonicalOf);
    assert.ok(first && second);
    for (const [left, right] of [[first, second], [second, first]] as const) {
      const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_PAIR, left, right);
      assertDiffEndpoints(diff, left, right);
      await assertPopulated(diff, left, right);
    }
  });

  test("Compare To Canonical on a row whose cluster id the report has since replaced still compares that row (#607)", async () => {
    assert.ok(api.client);
    for (const cluster of clusters) {
      const canonical = canonicalOf(cluster);
      for (const supersededId of SUPERSEDED_CLUSTER_IDS) {
        for (const { index, peer } of peersOf(cluster)) {
          const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_WITH_CANONICAL, supersededId, peer);
          assertDiffEndpoints(diff, canonical, peer);
          assert.equal(openDiffTabs(), ONE_DIFF_TAB, `${supersededId} peer ${index}: exactly one diff opened`);
        }
      }
    }
  });

  test("clicking compare again while a verdict is pending opens one diff and asks the engine once (#607 #608)", async () => {
    assert.ok(api.client);
    const [cluster] = clusters;
    assert.ok(cluster);
    const canonical = canonicalOf(cluster);
    const peer = firstPeerOf(cluster);
    for (const [command, args] of [
      [COMPARE_WITH_CANONICAL, [cluster.id, peer]],
      [COMPARE_PAIR, [canonical, peer]],
    ] as const) {
      await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
      const verdicts = holdPairVerdicts(api.client);
      const clicks = Array.from({ length: REPEATED_CLICKS }, () => vscode.commands.executeCommand(command, ...args));
      try {
        const diff = await waitForDiff();
        assertDiffEndpoints(diff.input, canonical, peer);
        assert.equal(openDiffTabs(), ONE_DIFF_TAB, `${command}: ${REPEATED_CLICKS} clicks on one pair show one diff`);
        assert.equal(verdicts.count(), ONE_VERDICT, `${command}: ${REPEATED_CLICKS} clicks on one pair ask the engine once`);
      } finally {
        verdicts.release();
        await Promise.all(clicks);
      }
    }
  });

  test("a verdict request that fails still leaves the diff open on both endpoints (#607 #608)", async () => {
    assert.ok(api.client);
    const [cluster] = clusters;
    assert.ok(cluster);
    const canonical = canonicalOf(cluster);
    const peer = firstPeerOf(cluster);
    for (const [command, args] of [
      [COMPARE_WITH_CANONICAL, [cluster.id, peer]],
      [COMPARE_PAIR, [canonical, peer]],
    ] as const) {
      await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
      const verdicts = holdPairVerdicts(api.client, { fail: true });
      const running = vscode.commands.executeCommand(command, ...args);
      try {
        const diff = await waitForDiff();
        assertDiffEndpoints(diff.input, canonical, peer);
      } finally {
        verdicts.release();
        await running;
      }
      assert.equal(openDiffTabs(), ONE_DIFF_TAB, `${command}: the diff stays open after the engine failed`);
    }
  });
});
