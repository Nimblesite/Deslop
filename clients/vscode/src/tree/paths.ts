// Path helpers shared across the tree-building modules. Kept in their
// own leaf module so `nodes.ts`, `grouping.ts`, `folder.ts`, and
// `language.ts` can all depend on them without an import cycle.

import * as vscode from "vscode";

import { ReportCluster } from "../types/report";

/** The path of a cluster's canonical (first) occurrence, or its id as a
 * last-resort fallback when the cluster somehow has no occurrences. */
export function representativePath(cluster: ReportCluster): string {
  return cluster.occurrences[0]?.path ?? cluster.id;
}

/** Workspace-relative display form of an absolute report path. Falls
 * back to a readable placeholder for empty input. */
export function displayPath(filePath: string): string {
  if (!filePath) return "unknown file";
  return vscode.workspace.asRelativePath(filePath, false);
}

/** Final path segment (file or folder name) of a display path. */
export function baseName(displayed: string): string {
  const segments = displayed.split(/[/\\]/).filter(Boolean);
  return segments[segments.length - 1] ?? displayed;
}
