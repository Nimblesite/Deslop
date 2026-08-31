// View-axis toggles for the Top Offenders panel
// ([VSIX-TOP-OFFENDERS-GROUPING], [VSIX-TOP-OFFENDERS-SORT],
// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]) and the facet filter
// ([FACET-TOP-OFFENDERS-FILTER]). Each writes to the workspace
// configuration target so the choice persists per-repo; the title-bar
// toggles and settings stay in sync via the context keys seeded in
// extension.ts.

import * as vscode from "vscode";

import { ReportStore } from "../reportStore";
import {
  FacetFilter,
  ReportCluster,
  SEVERITIES,
  sanitizeFacetFilter,
  severityLabel,
  Severity,
  clusterBand,
} from "../types/report";

const FILTER_SEVERITIES_SETTING = "topOffenders.filterSeverities";
const DESLOP_CONFIGURATION_NAMESPACE = "deslop";

async function updateWorkspace(key: string, value: unknown): Promise<void> {
  await vscode.workspace
    .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
    .update(key, value, vscode.ConfigurationTarget.Workspace);
}

export async function setTopOffendersGroupBy(
  value: "cluster" | "file" | "folder" | "severity",
): Promise<void> {
  await updateWorkspace("topOffenders.groupBy", value);
}

export async function setTopOffendersSortBy(value: "impact" | "path"): Promise<void> {
  await updateWorkspace("topOffenders.sortBy", value);
}

// [FACET-TOP-OFFENDERS-FILTER] Reads the persisted severity facet array,
// dropping unknown values (the typo fallback — a bad value must never
// yield an empty tree).
export function readTopOffendersFilter(): FacetFilter {
  const config = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
  return sanitizeFacetFilter(
    config.get<string[]>(FILTER_SEVERITIES_SETTING, []) ?? [],
  );
}

/** True when the facet filter is active. Drives the
 * `deslop.topOffendersFiltered` context key and toolbar icon state. */
export function isTopOffendersFilterActive(): boolean {
  const { severities } = readTopOffendersFilter();
  return severities.length > 0;
}

// [FACET-TOP-OFFENDERS-FILTER] Clears the persisted filter — the
// action bound to the filtered status row and the active-filter button.
export async function clearTopOffendersFilter(): Promise<void> {
  await updateWorkspace(FILTER_SEVERITIES_SETTING, []);
}

/** One row of the Choose Filter QuickPick, remembering the wire value it
 * stands for. */
interface FacetPickItem extends vscode.QuickPickItem {
  wire: Severity;
}

/** Rows for every severity band present in `clusters`, each with the
 * shared label and its live cluster count. Only present values are
 * offered. */
function facetPickItems(clusters: ReportCluster[], current: FacetFilter): FacetPickItem[] {
  const noun = (count: number): string => (count === 1 ? "cluster" : "clusters");
  return SEVERITIES.map((severity) => ({
    severity,
    count: clusters.filter((cluster) => clusterBand(cluster) === severity).length,
  }))
    .filter(({ count }) => count > 0)
    .map(({ severity, count }) => ({
      label: severityLabel(severity),
      description: `${count} ${noun(count)}`,
      wire: severity,
      picked: current.severities.includes(severity),
    }));
}

// [FACET-TOP-OFFENDERS-FILTER] Choose Filter: a multi-select QuickPick
// over the severity bands present in the current report. Selecting
// nothing (and confirming) clears the filter; cancelling leaves it
// untouched. Writes the persisted, workspace-scoped array.
export async function chooseTopOffendersFilter(store: ReportStore): Promise<void> {
  const clusters = store.current.visibleReport?.clusters ?? [];
  const items = facetPickItems(clusters, readTopOffendersFilter());
  if (items.length === 0) {
    void vscode.window.showInformationMessage("Deslop: no clusters to filter yet.");
    return;
  }
  const picked = await vscode.window.showQuickPick(items, {
    canPickMany: true,
    title: "Filter Top Offenders",
    placeHolder: "Show only the selected severity bands (empty selection shows all)",
  });
  if (!picked) return;
  await updateWorkspace(
    FILTER_SEVERITIES_SETTING,
    picked.map((item) => item.wire),
  );
}
