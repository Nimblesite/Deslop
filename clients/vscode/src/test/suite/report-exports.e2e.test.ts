// [OUTPUT-REPORT-REUSE] Drive the actual HTML button command with the real LSP.
import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import type { Report, ReportDelta } from "../../types/report";
import { activateExtension } from "./helpers";

const REPORT_GET = "deslop/reportGet";
const REPORT_DELTA = "deslop/reportDelta";
const OPEN_HTML = "deslop.openHtmlReport";
const HTML_TITLE = "Deslop HTML Report";
const REPEATED_CLICKS = 3;
const SINGLE_REPORT_TAB = 1;
const MINIMUM_OCCURRENCES = 2;

suite("report exports", () => {
  test("HTML button reuses the live calculations and one report tab", async () => {
    const { client } = await activateExtension();
    assert.ok(client, "real language client must be running");
    const expected = await client.sendRequest<Report>(REPORT_GET);
    assert.ok(expected.clusters.some((cluster) => cluster.occurrence_count >= MINIMUM_OCCURRENCES));
    const before = await generation(client);
    for (let click = 0; click < REPEATED_CLICKS; click += 1) {
      await openHtmlTab();
      assert.deepEqual(await client.sendRequest<Report>(REPORT_GET), expected);
      assert.equal(await generation(client), before, "HTML export must not refresh analysis");
      const tabs = vscode.window.tabGroups.all.flatMap((group) => group.tabs);
      assert.equal(tabs.filter((tab) => tab.label === HTML_TITLE).length, SINGLE_REPORT_TAB);
    }
  });
});

async function generation(client: LanguageClient): Promise<number> {
  const delta = await client.sendRequest<ReportDelta | null>(REPORT_DELTA, { since_generation: 0 });
  assert.ok(delta, "real LSP must return the current generation");
  return delta.to_generation;
}

async function openHtmlTab(): Promise<void> {
  await vscode.commands.executeCommand(OPEN_HTML);
  if (hasHtmlTab()) return;
  await new Promise<void>((resolve) => {
    const changed = vscode.window.tabGroups.onDidChangeTabs(() => {
      if (!hasHtmlTab()) return;
      changed.dispose();
      resolve();
    });
  });
}

function hasHtmlTab(): boolean {
  return vscode.window.tabGroups.all.some((group) => group.tabs.some((tab) => tab.label === HTML_TITLE));
}
