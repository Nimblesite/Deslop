// [VSIX-TOP-OFFENDERS-GROUPING] One grouping picker and severity filter, persisted per workspace.

import * as vscode from "vscode";

import { ReportStore } from "../reportStore";
import { GroupBy, normalizeGroupBy } from "../tree/grouping";
import {
  FacetFilter,
  ReportCluster,
  SEVERITIES,
  sanitizeFacetFilter,
  severityLabel,
  Severity,
  clusterSeverity,
} from "../types/report";

const FILTER_SEVERITIES_SETTING = "topOffenders.filterSeverities";
const DESLOP_CONFIGURATION_NAMESPACE = "deslop";

async function updateWorkspace(key: string, value: unknown): Promise<void> {
  await vscode.workspace
    .getConfiguration(DESLOP_CONFIGURATION_NAMESPACE)
    .update(key, value, vscode.ConfigurationTarget.Workspace);
}

export async function setTopOffendersGroupBy(
  value: GroupBy,
): Promise<void> {
  await updateWorkspace("topOffenders.groupBy", value);
}

// [VSIX-TOP-OFFENDERS-GROUPING] All grouping choices share one discoverable menu.
export const GROUPING_OPTIONS: readonly { label: string; value: GroupBy }[] = [
  { label: "Clone Category", value: "kind" },
  { label: "Folder", value: "folder" },
  { label: "Language", value: "language" },
  { label: "File", value: "file" },
  { label: "No Grouping", value: "cluster" },
];

export async function chooseTopOffendersGrouping(): Promise<void> {
  const current = normalizeGroupBy(vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE).get<string>("topOffenders.groupBy"));
  const picked = await vscode.window.showQuickPick(
    GROUPING_OPTIONS.map((item) => ({ ...item, description: item.value === current ? "Current grouping" : "" })),
    { title: "Group Top Offenders", placeHolder: "Choose a grouping — highest weight first" },
  );
  if (picked) await setTopOffendersGroupBy(picked.value);
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


function facetPickItems(clusters: ReportCluster[], current: FacetFilter): FacetPickItem[] {
  const noun = (count: number): string => (count === 1 ? "cluster" : "clusters");
  return SEVERITIES.map((severity) => ({
    severity,
    count: clusters.filter((cluster) => clusterSeverity(cluster) === severity).length,
  }))
    .filter(({ count }) => count > 0)
    .map(({ severity, count }) => ({
      label: severityLabel(severity),
      description: `${count} ${noun(count)}`,
      wire: severity,
      picked: current.severities.includes(severity),
    }));
}

// [FACET-TOP-OFFENDERS-FILTER] Selecting no levels clears the filter;
// cancellation keeps the persisted workspace selection.
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
    placeHolder: "Show only the selected diagnostic levels (empty selection shows all)",
  });
  if (!picked) return;
  await updateWorkspace(
    FILTER_SEVERITIES_SETTING,
    picked.map((item) => item.wire),
  );
}
