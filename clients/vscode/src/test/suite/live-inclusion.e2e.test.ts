// [METRICS-TOTAL-INCLUDE] [VSIX-TOP-OFFENDERS-EXCLUDED] Real config watcher → LSP → report store → UI.
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { DidChangeConfigurationNotification, DocumentDiagnosticRequest, type FullDocumentDiagnosticReport } from "vscode-languageclient/node";
import type { ReportStore } from "../../reportStore";
import type { Report } from "../../types/report";
import { decoratableClusters } from "../../decorations/manager";
import { fixtureRoot, fixtureUri, waitForReport } from "./helpers";

const CONFIG_NAME = ".deslop.toml";
const EMPTY_CONFIG = "[total]\ninclude = []\n";
const DEFAULT_INCLUDES = ["identical", "nearly_identical"];
const FIRST_FILE = "Alpha.cs";
const EXPECTED_CLUSTER_COUNT = 3;
const DIAGNOSTICS_ENABLED = "diagnostics.enabled";

function settledReport(store: ReportStore): Promise<Report> {
  return new Promise((resolve) => {
    const choose = () => {
      const { visibleReport, lifecycle } = store.current;
      if (lifecycle.kind === "ready" && visibleReport?.clusters.length === EXPECTED_CLUSTER_COUNT) {
        subscription.dispose(); resolve(visibleReport);
      }
    };
    const subscription = store.onDidChange(choose);
    choose();
  });
}

function changeConfig(store: ReportStore, configPath: string, content: string | null, includes: readonly string[]): Promise<Report> {
  const before = store.current.generation;
  return new Promise((resolve, reject) => {
    const subscription = store.onDidChange((state) => {
      const report = state.visibleReport;
      if (state.generation > before && report && JSON.stringify(report.metrics.total_includes) === JSON.stringify(includes)) {
        subscription.dispose(); resolve(report);
      }
    });
    try {
      if (content === null) fs.unlinkSync(configPath);
      else fs.writeFileSync(configPath, content);
    } catch (error) { subscription.dispose(); reject(error); }
  });
}

suite("live include configuration", () => {
  test("excluding and restoring kinds preserves every clone through the real live loop", async () => {
    const api = await waitForReport();
    const store = api.reportStore;
    const client = api.client;
    assert.ok(store && client);
    const original = await settledReport(store);
    assert.deepEqual(original.metrics.total_includes, DEFAULT_INCLUDES);
    assert.ok(original.clusters.length > 0);
    const configPath = path.join(fixtureRoot(), CONFIG_NAME);
    const saved = fs.existsSync(configPath) ? fs.readFileSync(configPath, "utf8") : null;
    const settings = vscode.workspace.getConfiguration("deslop");
    const diagnosticSetting = settings.inspect<boolean>(DIAGNOSTICS_ENABLED)?.workspaceValue;
    await settings.update(DIAGNOSTICS_ENABLED, true, vscode.ConfigurationTarget.Workspace);
    await client.sendNotification(DidChangeConfigurationNotification.method, { settings: { deslop: { diagnostics: { enabled: true } } } });
    try {
      const initialDiagnostic = await client.sendRequest<FullDocumentDiagnosticReport>(DocumentDiagnosticRequest.method, { textDocument: { uri: fixtureUri(FIRST_FILE).toString() } });
      assert.equal(initialDiagnostic.items.length, 1, "enabled diagnostics must be visible before exclusion");
      const excluded = await changeConfig(store, configPath, EMPTY_CONFIG, []);
      assert.deepEqual(excluded.clusters, original.clusters, "cluster IDs, paths, kinds, copies, ranks and mass survive");
      assert.equal(excluded.metrics.duplicated_loc, 0);
      assert.equal(excluded.metrics.duplication_percent, 0);
      assert.ok(excluded.metrics.per_file.every((row) => row.duplicated_loc === 0));
      assert.ok(excluded.metrics.per_kind.every((row) => !row.included_in_total));
      assert.deepEqual(decoratableClusters(excluded), []);
      const diagnostics = await client.sendRequest<FullDocumentDiagnosticReport>(DocumentDiagnosticRequest.method, { textDocument: { uri: fixtureUri(FIRST_FILE).toString() } });
      assert.ok(diagnostics.kind === "full");
      assert.deepEqual(diagnostics.items, []);
    } finally {
      try {
      const restored = await changeConfig(store, configPath, saved, DEFAULT_INCLUDES);
      assert.deepEqual(restored.clusters, original.clusters);
      assert.equal(restored.metrics.duplicated_loc, original.metrics.duplicated_loc);
      assert.equal(restored.metrics.duplication_percent, original.metrics.duplication_percent);
      assert.ok(decoratableClusters(restored).length > 0);
      const diagnostics = await client.sendRequest<FullDocumentDiagnosticReport>(DocumentDiagnosticRequest.method, { textDocument: { uri: fixtureUri(FIRST_FILE).toString() } });
      assert.ok(diagnostics.kind === "full");
      assert.equal(diagnostics.items.length, 1, "restoring the kind restores the real editor diagnostic");
      } finally {
        await settings.update(DIAGNOSTICS_ENABLED, diagnosticSetting, vscode.ConfigurationTarget.Workspace);
      }
    }
  });
});
