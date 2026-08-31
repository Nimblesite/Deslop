// Readonly deslop://cluster/<id> documents for cluster links.

import * as vscode from "vscode";

import { log, logWarn } from "./logging";
import { occurrenceDisplayLocation } from "./locations";
import { ReportStore } from "./reportStore";
import { formatScore } from "./types/format";
import { occurrenceCount, Report, ReportCluster, ReportOccurrence } from "./types/report";

export const CLUSTER_DOCUMENT_SCHEME = "deslop";

export function registerClusterDocumentProvider(
  context: vscode.ExtensionContext,
  store: ReportStore,
): void {
  // [VSIX-STATE-DIRTY]: cluster preview documents are a surface — render
  // from the visible projection so an in-progress edit drops the row.
  const provider = { provideTextDocumentContent: (uri: vscode.Uri) => clusterDocumentContent(uri, store.current.visibleReport) };
  context.subscriptions.push(vscode.workspace.registerTextDocumentContentProvider(CLUSTER_DOCUMENT_SCHEME, provider));
}

export function clusterDocumentContent(uri: vscode.Uri, report: Report | null): string {
  const clusterId = clusterIdFromUri(uri);
  log("cluster document requested", { uri: uri.toString(), clusterId });
  if (!clusterId) return invalidClusterDocument(uri);
  const cluster = report?.clusters.find((item) => item.id === clusterId);
  if (!cluster) {
    logWarn("cluster document missing", { clusterId });
    return missingClusterDocument(clusterId);
  }
  return renderClusterDocument(cluster);
}

function clusterIdFromUri(uri: vscode.Uri): string | undefined {
  if (uri.scheme !== CLUSTER_DOCUMENT_SCHEME) return undefined;
  if (uri.authority === "cluster") return nonEmpty(trimLeadingSlashes(uri.path));
  const segments = uri.path.split("/").filter((segment) => segment.length > 0);
  return segments[0] === "cluster" ? nonEmpty(segments[1] ?? "") : undefined;
}

function trimLeadingSlashes(value: string): string {
  let out = value;
  while (out.startsWith("/")) out = out.slice(1);
  return out;
}

function nonEmpty(value: string): string | undefined {
  return value.length > 0 ? value : undefined;
}

function invalidClusterDocument(uri: vscode.Uri): string {
  return [
    "# Deslop cluster",
    "",
    `Unable to parse cluster id from ${uri.toString()}.`,
    "Expected deslop://cluster/<id>.",
  ].join("\n");
}

function missingClusterDocument(clusterId: string): string {
  return [
    `# Deslop cluster ${clusterId}`,
    "",
    "This cluster is not present in the current report snapshot.",
    "Refresh the Deslop report and open the cluster link again.",
  ].join("\n");
}

function renderClusterDocument(cluster: ReportCluster): string {
  return [
    `# Deslop cluster ${cluster.id}`,
    "",
    `Occurrences: ${occurrenceCount(cluster)}`,
    `Weight: ${formatScore(cluster.weight)}`,
    "",
    "## Occurrences",
    ...cluster.occurrences.map(renderOccurrence),
  ].join("\n");
}

function renderOccurrence(occurrence: ReportOccurrence, index: number): string {
  const location = occurrence.displayLocation ?? occurrenceDisplayLocation(occurrence);
  const label = location?.label ?? occurrence.path;
  const hidden = occurrence.hidden ? " hidden" : "";
  return `${index + 1}. ${label}${hidden}`;
}
