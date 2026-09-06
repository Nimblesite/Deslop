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
  BUCKETS,
  bucketLabels,
  CATEGORIES,
  categoryLabels,
  FacetFilter,
  ReportCluster,
  resolveBucket,
  resolveCategory,
  sanitizeFacetFilter,
} from "../types/report";

async function updateWorkspace(key: string, value: unknown): Promise<void> {
  await vscode.workspace
    .getConfiguration("deslop")
    .update(key, value, vscode.ConfigurationTarget.Workspace);
}

export async function setTopOffendersGroupBy(
  value: "cluster" | "file" | "folder" | "type",
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

// [FACET-TOP-OFFENDERS-FILTER] Reads the persisted facet filter arrays,
// dropping unknown values (the typo fallback — a bad value must never
// yield an empty tree).
export function readTopOffendersFilter(): FacetFilter {
  const config = vscode.workspace.getConfiguration("deslop");
  return sanitizeFacetFilter(
    config.get<string[]>("topOffenders.filterBuckets", []) ?? [],
    config.get<string[]>("topOffenders.filterCategories", []) ?? [],
  );
}

/** True when any facet-filter axis is active. Drives the
 * `deslop.topOffendersFiltered` context key and toolbar icon state. */
export function isTopOffendersFilterActive(): boolean {
  const { buckets, categories } = readTopOffendersFilter();
  return buckets.length > 0 || categories.length > 0;
}

// [FACET-TOP-OFFENDERS-FILTER] Clears both persisted filter axes — the
// action bound to the filtered status row and the active-filter button.
export async function clearTopOffendersFilter(): Promise<void> {
  await updateWorkspace("topOffenders.filterBuckets", []);
  await updateWorkspace("topOffenders.filterCategories", []);
}

/** One row of the Choose Filter QuickPick, remembering which axis and
 * wire value it stands for. */
interface FacetPickItem extends vscode.QuickPickItem {
  axis: "bucket" | "category";
  wire: string;
}

/** Rows for every bucket and category present in `clusters`, each with
 * the shared plain label and its live cluster count. Only present
 * values are offered (`same_behavior` appears only when the embedding
 * pass ran, #195). */
function facetPickItems(clusters: ReportCluster[], current: FacetFilter): FacetPickItem[] {
  const noun = (count: number): string => (count === 1 ? "cluster" : "clusters");
  const buckets: FacetPickItem[] = BUCKETS.map((bucket) => ({
    bucket,
    count: clusters.filter((cluster) => resolveBucket(cluster) === bucket).length,
  }))
    .filter(({ count }) => count > 0)
    .map(({ bucket, count }) => ({
      label: bucketLabels(bucket).plainTitle,
      description: `${count} ${noun(count)}`,
      axis: "bucket",
      wire: bucket,
      picked: current.buckets.includes(bucket),
    }));
  const categories: FacetPickItem[] = CATEGORIES.map((category) => ({
    category,
    count: clusters.filter((cluster) => resolveCategory(cluster) === category).length,
  }))
    .filter(({ count }) => count > 0)
    .map(({ category, count }) => ({
      label: categoryLabels(category).groupTitle,
      description: `${count} ${noun(count)}`,
      axis: "category",
      wire: category,
      picked: current.categories.includes(category),
    }));
  return [...buckets, ...categories];
}

// [FACET-TOP-OFFENDERS-FILTER] Choose Filter: a multi-select QuickPick
// over the buckets and categories present in the current report.
// Selecting nothing (and confirming) clears the filter; cancelling
// leaves it untouched. Writes both persisted, workspace-scoped arrays.
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
    placeHolder: "Show only the selected clone types (empty selection shows all)",
  });
  if (!picked) return;
  await updateWorkspace(
    "topOffenders.filterBuckets",
    picked.filter((item) => item.axis === "bucket").map((item) => item.wire),
  );
  await updateWorkspace(
    "topOffenders.filterCategories",
    picked.filter((item) => item.axis === "category").map((item) => item.wire),
  );
}
