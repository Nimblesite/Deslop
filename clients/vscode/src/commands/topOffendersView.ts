// View-axis toggles for the Top Offenders panel
// ([VSIX-TOP-OFFENDERS-GROUPING], [VSIX-TOP-OFFENDERS-SORT],
// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]). Each writes to the workspace
// configuration target so the choice persists per-repo; the title-bar
// toggles and settings stay in sync via the context keys seeded in
// extension.ts.

import * as vscode from "vscode";

async function updateWorkspace(key: string, value: unknown): Promise<void> {
  await vscode.workspace
    .getConfiguration("deslop")
    .update(key, value, vscode.ConfigurationTarget.Workspace);
}

export async function setTopOffendersGroupBy(
  value: "cluster" | "file" | "folder",
): Promise<void> {
  await updateWorkspace("topOffenders.groupBy", value);
}

export async function setTopOffendersSortBy(value: "impact" | "path"): Promise<void> {
  await updateWorkspace("topOffenders.sortBy", value);
}

export async function toggleTopOffendersSplitByLanguage(): Promise<void> {
  const current = vscode.workspace
    .getConfiguration("deslop")
    .get<boolean>("topOffenders.splitByLanguage", false);
  await updateWorkspace("topOffenders.splitByLanguage", !current);
}
