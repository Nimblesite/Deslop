// Readonly deslop://cluster/<id> documents for cluster links.

import * as vscode from "vscode";

import { log, logWarn } from "./logging";
import { occurrenceDisplayLocation } from "./locations";
import { ReportStore } from "./reportStore";
import { occurrenceCount, ReportCluster, ReportOccurrence } from "./types/report";

export const CLUSTER_DOCUMENT_SCHEME = "deslop";

export function registerClusterDocumentProvider(
  context: vscode.ExtensionContext,
  store: ReportStore,
): void {
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider(
      CLUSTER_DOCUMENT_SCHEME,
      new ClusterDocumentProvider(store),
    ),
  );
}

class ClusterDocumentProvider implements vscode.TextDocumentContentProvider {
  constructor(private readonly store: ReportStore) {}

  provideTextDocumentContent(uri: vscode.Uri): string {
    const clusterId = clusterIdFromUri(uri);
    log("cluster document requested", { uri: uri.toString(), clusterId });
    if (!clusterId) return invalidClusterDocument(uri);
    const cluster = this.store.current.report?.clusters.find((item) => item.id === clusterId);
    if (!cluster) {
      logWarn("cluster document missing", { clusterId });
      return missingClusterDocument(clusterId);
    }
    return renderClusterDocument(cluster);
  }
}

function clusterIdFromUri(uri: vscode.Uri): string | undefined {
  if (uri.scheme !== CLUSTER_DOCUMENT_SCHEME) return undefined;
  if (uri.authority === "cluster") return trimLeadingSlashes(uri.path) || undefined;
  const segments = uri.path.split("/").filter((segment) => segment.length > 0);
  if (segments[0] !== "cluster") return undefined;
  return segments[1] || undefined;
}

function trimLeadingSlashes(value: string): string {
  let out = value;
  while (out.startsWith("/")) out = out.slice(1);
  return out;
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
    `Weight: ${cluster.weight.toFixed(2)}`,
    `Signals: structural ${cluster.signals.structural.toFixed(2)}, ` +
      `jaccard ${cluster.signals.token_jaccard.toFixed(2)}, ` +
      `embedding ${cluster.signals.embedding_cos.toFixed(2)}`,
    "",
    "## Occurrences",
    ...cluster.occurrences.map(renderOccurrence),
  ].join("\n");
}

function renderOccurrence(occurrence: ReportOccurrence, index: number): string {
  const location = occurrenceDisplayLocation(occurrence);
  const label = location?.label ?? occurrence.path;
  const hidden = occurrence.hidden ? " hidden" : "";
  return `${index + 1}. ${label}${hidden}`;
}
