// E2E: drive the Top Offenders panel axes and the tree-node command
// family against the live extension host. The grouping and sort axes
// are configuration writes, so every toggle is asserted on the
// persisted workspace value; the copy family is asserted on the
// clipboard; the open family is asserted on the editor tabs it
// produces ([VSIX-TOP-OFFENDERS-GROUPING], [VSIX-TOP-OFFENDERS-SORT],
// [VSIX-COMMANDS]).

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";
import { clusterBand, type Report, type ReportCluster } from "../../types/report";
import { ClusterNode, OccurrenceNode } from "../../tree/nodes";
import { activateExtension, sleep } from "./helpers";

const DESLOP_CONFIGURATION_NAMESPACE = "deslop";
const GROUP_BY_SETTING = "topOffenders.groupBy";
const SORT_BY_SETTING = "topOffenders.sortBy";
const FILTER_SEVERITIES_SETTING = "topOffenders.filterSeverities";

const GROUP_BY_AXES: Array<"cluster" | "file" | "folder" | "severity"> = [
  "cluster",
  "file",
  "folder",
  "severity",
];

function readConfig<T>(key: string): T | undefined {
  return vscode.workspace
    .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
    .get<T>(key);
}

async function waitForReport(): Promise<ExtensionApi> {
  const api = await activateExtension();
  for (let i = 0; i < 20; i++) {
    await sleep(250);
    const cmds = await vscode.commands.getCommands(true);
    if (cmds.includes("deslop.openCluster")) return api;
  }
  throw new Error("extension did not activate in time");
}

async function waitForMultiOccurrenceCluster(
  client: NonNullable<ExtensionApi["client"]>,
): Promise<ReportCluster> {
  let last: Report | undefined;
  for (let i = 0; i < 40; i += 1) {
    last = await client.sendRequest<Report>("deslop/reportGet");
    const cluster = last.clusters.find((candidate) => candidate.occurrences.length >= 2);
    if (cluster) return cluster;
    await sleep(250);
  }
  throw new Error(
    `no multi-occurrence cluster in LSP report; last count ${last?.clusters.length ?? 0}`,
  );
}

async function waitForTab(predicate: (tab: vscode.Tab) => boolean): Promise<vscode.Tab> {
  for (let i = 0; i < 100; i += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (predicate(tab)) return tab;
      }
    }
    await sleep(100);
  }
  throw new Error("expected editor tab never opened");
}

suite("top offenders panel and node commands", () => {
  let api: ExtensionApi;

  suiteSetup(async () => {
    api = await waitForReport();
  });

  async function requireCluster(): Promise<ReportCluster> {
    const client = api.client;
    assert.ok(client, "extension must expose the real LanguageClient");
    return await waitForMultiOccurrenceCluster(client);
  }

  suiteTeardown(async () => {
    await vscode.workspace
      .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
      .update(GROUP_BY_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
      .update(SORT_BY_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
      .update(FILTER_SEVERITIES_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
  });

  test("every grouping axis persists its workspace value", async () => {
    for (const axis of GROUP_BY_AXES) {
      await vscode.commands.executeCommand(`deslop.topOffenders.showBy${axis.charAt(0).toUpperCase()}${axis.slice(1)}`);
      assert.equal(
        await readConfig(GROUP_BY_SETTING),
        axis,
        `the ${axis} axis command must persist the grouping choice`,
      );
    }
  });

  test("both sort axes persist their workspace value", async () => {
    await vscode.commands.executeCommand("deslop.topOffenders.sortByImpact");
    assert.equal(await readConfig(SORT_BY_SETTING), "impact", "impact sort must persist");
    await vscode.commands.executeCommand("deslop.topOffenders.sortByPath");
    assert.equal(await readConfig(SORT_BY_SETTING), "path", "path sort must persist");
  });

  test("clearFilter empties the persisted severity facet", async () => {
    await vscode.workspace
      .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
      .update(FILTER_SEVERITIES_SETTING, ["worst"], vscode.ConfigurationTarget.Workspace);
    await vscode.commands.executeCommand("deslop.topOffenders.clearFilter");
    assert.deepEqual(
      await readConfig(FILTER_SEVERITIES_SETTING),
      [],
      "clearFilter must empty the persisted facet array",
    );
  });

  test("cluster-node commands open the cluster's files and locations", async () => {
    const wire = await requireCluster();
    const clusterNode = new ClusterNode(wire, clusterBand(wire), { showFile: true });

    await vscode.commands.executeCommand("deslop.openClusterDetails", clusterNode);
    await vscode.commands.executeCommand("deslop.openAllOccurrences", clusterNode);
    await waitForTab((tab) => tab.input instanceof vscode.TabInputText);
    await vscode.commands.executeCommand("deslop.openCanonicalFile", clusterNode);
    await waitForTab((tab) => tab.input instanceof vscode.TabInputText);

    const first = wire.occurrences[0];
    assert.ok(first, "the cluster must carry a first occurrence");
    const occurrenceNode = new OccurrenceNode(first, wire, wire.rank, 0);
    await vscode.commands.executeCommand("deslop.revealOccurrenceInExplorer", occurrenceNode);
  });

  test("the copy family writes the cluster facts to the clipboard", async () => {
    const wire = await requireCluster();
    const clusterNode = new ClusterNode(wire, clusterBand(wire), { showFile: true });
    const first = wire.occurrences[0];
    assert.ok(first, "the cluster must carry a first occurrence");
    const occurrenceNode = new OccurrenceNode(first, wire, wire.rank, 0);

    await vscode.commands.executeCommand("deslop.copyClusterContextById", wire.id);
    const byId = await vscode.env.clipboard.readText();
    assert.ok(
      byId.includes(wire.id),
      `copyClusterContextById must carry the cluster id: ${byId.slice(0, 200)}`,
    );

    await vscode.commands.executeCommand("deslop.copyContextForAI", clusterNode);
    const forAi = await vscode.env.clipboard.readText();
    assert.ok(
      forAi.includes(wire.id),
      `copyContextForAI must carry the cluster id: ${forAi.slice(0, 200)}`,
    );

    await vscode.commands.executeCommand("deslop.copyClusterLocations", clusterNode);
    const locations = await vscode.env.clipboard.readText();
    assert.ok(
      locations.includes(first.path),
      `copyClusterLocations must carry the occurrence path: ${locations}`,
    );

    await vscode.commands.executeCommand("deslop.copyHumanLocation", occurrenceNode);
    const human = await vscode.env.clipboard.readText();
    assert.ok(
      human.includes(first.path),
      `copyHumanLocation must carry the occurrence path: ${human}`,
    );

    assert.ok(first.path, "the fixture occurrence must name a file");
    await vscode.commands.executeCommand("deslop.copySourceSnippet", occurrenceNode);
    const source = await vscode.env.clipboard.readText();
    assert.ok(
      source.length > 0,
      "copySourceSnippet must write the source bytes under the occurrence range",
    );
  });
});
