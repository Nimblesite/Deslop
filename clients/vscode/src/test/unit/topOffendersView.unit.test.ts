// Unit: the Top Offenders view-axis toggles
// ([VSIX-TOP-OFFENDERS-GROUPING] / [VSIX-TOP-OFFENDERS-SORT] /
// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]). Each command writes the persisted
// workspace configuration that the tree provider and the title-bar `when`
// clauses read back. Runs under vscode-test so the real workspace
// configuration store round-trips the writes.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import {
  clearTopOffendersFilter,
  isTopOffendersFilterActive,
  readTopOffendersFilter,
  setTopOffendersGroupBy,
  setTopOffendersSortBy,
} from "../../commands/topOffendersView";

function cfg(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration("deslop");
}

async function resetViewAxes(): Promise<void> {
  const c = cfg();
  await c.update("topOffenders.groupBy", undefined, vscode.ConfigurationTarget.Workspace);
  await c.update("topOffenders.sortBy", undefined, vscode.ConfigurationTarget.Workspace);
  await c.update(
    "topOffenders.filterSeverities",
    undefined,
    vscode.ConfigurationTarget.Workspace,
  );
}

suite("top offenders view-axis toggles", () => {
  teardown(async () => {
    await resetViewAxes();
  });

  test("setTopOffendersGroupBy persists each of the four grouping modes", async () => {
    for (const mode of ["cluster", "file", "folder", "severity"] as const) {
      await setTopOffendersGroupBy(mode);
      assert.equal(
        cfg().get<string>("topOffenders.groupBy"),
        mode,
        `groupBy must persist ${mode} so the tree provider and toggle render it`,
      );
    }
  });

  // [FACET-TOP-OFFENDERS-FILTER] The persisted facet-filter arrays
  // round-trip through the workspace configuration, unknown values are
  // dropped on read, and the clear action resets both axes.
  test("facet filter persists, sanitizes unknown values, and clears", async () => {
    assert.equal(isTopOffendersFilterActive(), false, "filter defaults to inactive");

    await cfg().update(
      "topOffenders.filterSeverities",
      ["worst", "not_a_severity"],
      vscode.ConfigurationTarget.Workspace,
    );
    assert.deepEqual(
      readTopOffendersFilter(),
      { severities: ["worst"] },
      "unknown values are dropped on read — a typo never empties the tree",
    );
    assert.equal(isTopOffendersFilterActive(), true);

    await clearTopOffendersFilter();
    assert.deepEqual(readTopOffendersFilter(), { severities: [] });
    assert.equal(isTopOffendersFilterActive(), false, "clear resets both axes");
  });

  test("setTopOffendersSortBy persists impact and path", async () => {
    await setTopOffendersSortBy("path");
    assert.equal(cfg().get<string>("topOffenders.sortBy"), "path");

    await setTopOffendersSortBy("impact");
    assert.equal(cfg().get<string>("topOffenders.sortBy"), "impact");
  });

});
