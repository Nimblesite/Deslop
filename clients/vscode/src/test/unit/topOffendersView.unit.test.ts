// Unit: the Top Offenders view-axis toggles
// ([VSIX-TOP-OFFENDERS-GROUPING] / [VSIX-TOP-OFFENDERS-SORT] /
// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]). Each command writes the persisted
// workspace configuration that the tree provider and the title-bar `when`
// clauses read back. Runs under vscode-test so the real workspace
// configuration store round-trips the writes.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import {
  setTopOffendersGroupBy,
  setTopOffendersSortBy,
  toggleTopOffendersSplitByLanguage,
} from "../../commands/topOffendersView";

function cfg(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration("deslop");
}

async function resetViewAxes(): Promise<void> {
  const c = cfg();
  await c.update("topOffenders.groupBy", undefined, vscode.ConfigurationTarget.Workspace);
  await c.update("topOffenders.sortBy", undefined, vscode.ConfigurationTarget.Workspace);
  await c.update("topOffenders.splitByLanguage", undefined, vscode.ConfigurationTarget.Workspace);
}

suite("top offenders view-axis toggles", () => {
  teardown(async () => {
    await resetViewAxes();
  });

  test("setTopOffendersGroupBy persists each of the three grouping modes", async () => {
    for (const mode of ["cluster", "file", "folder"] as const) {
      await setTopOffendersGroupBy(mode);
      assert.equal(
        cfg().get<string>("topOffenders.groupBy"),
        mode,
        `groupBy must persist ${mode} so the tree provider and toggle render it`,
      );
    }
  });

  test("setTopOffendersSortBy persists impact and path", async () => {
    await setTopOffendersSortBy("path");
    assert.equal(cfg().get<string>("topOffenders.sortBy"), "path");

    await setTopOffendersSortBy("impact");
    assert.equal(cfg().get<string>("topOffenders.sortBy"), "impact");
  });

  test("toggleTopOffendersSplitByLanguage flips the persisted flag from its default", async () => {
    assert.equal(
      cfg().get<boolean>("topOffenders.splitByLanguage", false),
      false,
      "split-by-language must default to off so folder mode does not double-nest languages",
    );

    await toggleTopOffendersSplitByLanguage();
    assert.equal(
      cfg().get<boolean>("topOffenders.splitByLanguage", false),
      true,
      "first toggle turns the language split on",
    );

    await toggleTopOffendersSplitByLanguage();
    assert.equal(
      cfg().get<boolean>("topOffenders.splitByLanguage", false),
      false,
      "second toggle turns the language split back off",
    );
  });
});
