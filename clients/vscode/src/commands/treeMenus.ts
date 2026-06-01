// VSIX tree context menu commands. Every handler is a thin function
// over existing VSIX primitives (occurrenceDisplayLocation for human
// locations, openOccurrence for navigation). Keep human-visible labels
// line/column only; AI-only payloads are the sole place byte ranges
// surface in copied text.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import { occurrenceDisplayLocation } from "../locations";
import { ReportStore } from "../reportStore";
import {
  ReportCluster,
  ReportOccurrence,
  bucketLabels,
  clusterSlug,
  occurrenceCount,
  resolveBucket,
} from "../types/report";
import { ClusterNode, OccurrenceNode } from "../tree/providers";
import { resolveOccurrenceUri } from "./register";

/// Prompt threshold: confirm before opening more than this many occurrence tabs.
export const OPEN_ALL_THRESHOLD = 5;

/// Copies `path:line:column` for a single occurrence to the clipboard.
/// Issue #12.
export async function copyHumanLocation(node: OccurrenceNode): Promise<void> {
  const text = humanLocation(node.occurrence);
  await vscode.env.clipboard.writeText(text);
  void vscode.window.showInformationMessage(`Copied ${text}`);
}

/// Copies the cluster header + every occurrence as `path:line:column`.
/// Issue #13.
export async function copyClusterLocations(node: ClusterNode): Promise<void> {
  const text = clusterLocationsText(node.cluster);
  await vscode.env.clipboard.writeText(text);
  void vscode.window.showInformationMessage(
    `Copied ${node.cluster.occurrences.length} locations`,
  );
}

/// Copies an AI-readable payload describing the selected cluster or
/// occurrence — byte ranges included, human locations alongside for
/// manual spot-checks. Issue #11.
export async function copyContextForAI(
  target: ClusterNode | OccurrenceNode,
  store: ReportStore,
): Promise<void> {
  const text = isClusterNode(target)
    ? aiPayloadForCluster(target.cluster, target.rank)
    : aiPayloadForOccurrence(target.occurrence, store);
  await vscode.env.clipboard.writeText(text);
  void vscode.window.showInformationMessage("Copied AI context to clipboard");
}

function isClusterNode(
  target: ClusterNode | OccurrenceNode,
): target is ClusterNode {
  return (target as ClusterNode).cluster !== undefined;
}

/// Copies a fenced source snippet for the occurrence byte range with
/// a compact header. Issue #17.
export async function copySourceSnippet(node: OccurrenceNode): Promise<void> {
  const text = sourceSnippetText(node.occurrence);
  await vscode.env.clipboard.writeText(text);
  void vscode.window.showInformationMessage("Copied source snippet to clipboard");
}

/// Resolves the cluster id to open from a tree-node selection. Occurrence
/// rows resolve to their parent cluster via the active report. Issue #15.
export function clusterIdForTreeNode(
  node: ClusterNode | OccurrenceNode,
  store: ReportStore,
): string | undefined {
  if (isClusterNode(node)) return node.cluster.id;
  return findParentCluster(node.occurrence, store)?.id;
}

/// Returns the first occurrence in a cluster, which is the canonical
/// instance used by the tree label, compare command, and report ordering.
export function canonicalOccurrenceForCluster(
  node: ClusterNode,
): ReportOccurrence | undefined {
  return node.cluster.occurrences[0];
}

/// Reveals the occurrence file in the VS Code Explorer. Issue #16.
export async function revealOccurrenceInExplorer(
  node: OccurrenceNode,
): Promise<void> {
  const uri = resolveOccurrenceUri(node.occurrence.path);
  if (!fs.existsSync(uri.fsPath)) {
    void vscode.window.showErrorMessage(
      `Deslop: file not found — ${node.occurrence.path}`,
    );
    return;
  }
  await vscode.commands.executeCommand("revealInExplorer", uri);
}

/// Opens every occurrence in a cluster at its exact line/column. Prompts
/// before opening more than [`OPEN_ALL_THRESHOLD`] files. Issue #19.
///
/// Uses non-preview editors so each occurrence gets a persistent tab —
/// VS Code's preview mode otherwise replaces the previous tab on every
/// subsequent open, leaving only the last occurrence visible.
export async function openAllOccurrences(node: ClusterNode): Promise<void> {
  const unique = dedupeOccurrences(node.cluster.occurrences);
  if (unique.length > OPEN_ALL_THRESHOLD) {
    const answer = await vscode.window.showWarningMessage(
      `Open all ${unique.length} occurrences?`,
      { modal: true },
      "Open all",
    );
    if (answer !== "Open all") return;
  }
  for (const occurrence of unique) {
    await openOccurrenceNonPreview(occurrence);
  }
}

async function openOccurrenceNonPreview(
  occurrence: ReportOccurrence,
): Promise<void> {
  const uri = resolveOccurrenceUri(occurrence.path);
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc, { preview: false });
}

// ---------- text renderers (pure, exported for unit tests) ----------

/// Builds the clipboard text for [`copyClusterLocations`].
export function clusterLocationsText(cluster: ReportCluster): string {
  const bucket = bucketLabels(resolveBucket(cluster)).plainTitle;
  const header = `cluster ${cluster.id} · ${bucket} · ${occurrenceCount(cluster)} occurrences`;
  const rows = cluster.occurrences.map(humanLocation);
  return [header, ...rows].join("\n");
}

/// Builds the AI payload for a cluster tree-node selection.
export function aiPayloadForCluster(
  cluster: ReportCluster,
  rank: number,
): string {
  const bucket = resolveBucket(cluster);
  const labels = bucketLabels(bucket);
  const header = [
    `slug: ${clusterSlug(cluster)}`,
    `cluster_id: ${cluster.id}`,
    `rank: ${rank}`,
    `bucket: ${bucket} (${labels.taxonomyLabel})`,
    `weight: ${cluster.weight.toFixed(4)}`,
    `size: ${cluster.size}`,
    `canonical_node_count: ${cluster.canonical_node_count}`,
    `occurrences: ${occurrenceCount(cluster)}`,
    signalsLine(cluster),
  ];
  const rows = cluster.occurrences.map(
    (o) => `- ${o.path} | ${humanLocation(o)} | ${o.start_byte}..${o.end_byte}`,
  );
  return [
    ...header,
    "",
    "occurrences (path | line:column | bytes):",
    ...rows,
    "",
    "Use these byte ranges as precise edit anchors for deduplication.",
  ].join("\n");
}

/// Builds the AI payload for an occurrence tree-node selection.
export function aiPayloadForOccurrence(
  occurrence: ReportOccurrence,
  store: ReportStore,
): string {
  const parent = findParentCluster(occurrence, store);
  const head: string[] = [
    `occurrence_path: ${occurrence.path}`,
    `human_location: ${humanLocation(occurrence)}`,
    `bytes: ${occurrence.start_byte}..${occurrence.end_byte}`,
  ];
  if (parent) head.push("", ...parentClusterLines(parent, store));
  head.push(
    "",
    "Use these byte ranges as precise edit anchors for deduplication.",
  );
  return head.join("\n");
}

/// Builds the fenced code block + header for [`copySourceSnippet`].
export function sourceSnippetText(occurrence: ReportOccurrence): string {
  const snippet = readOccurrenceBytes(occurrence);
  const language = languageForPath(occurrence.path);
  return [
    humanLocation(occurrence),
    "```" + language,
    snippet,
    "```",
  ].join("\n");
}

// ---------- helpers ----------

function humanLocation(occurrence: ReportOccurrence): string {
  return occurrenceDisplayLocation(occurrence)?.label ?? occurrence.path;
}

function signalsLine(cluster: ReportCluster): string {
  const s = cluster.signals;
  return `signals: structural=${s.structural.toFixed(4)} token=${s.token_jaccard.toFixed(4)} embed=${s.embedding_cos.toFixed(4)} fused=${s.fused.toFixed(4)}`;
}

function parentClusterLines(
  parent: ReportCluster,
  store: ReportStore,
): string[] {
  const bucket = resolveBucket(parent);
  const labels = bucketLabels(bucket);
  const all = store.current.report?.clusters ?? [];
  const rankIndex = all.findIndex((c) => c.id === parent.id);
  return [
    `cluster_id: ${parent.id}`,
    `rank: ${rankIndex >= 0 ? rankIndex + 1 : "?"}`,
    `bucket: ${bucket} (${labels.taxonomyLabel})`,
    `weight: ${parent.weight.toFixed(4)}`,
    `size: ${parent.size}`,
    signalsLine(parent),
    `sibling_occurrences: ${Math.max(parent.occurrences.length - 1, 0)}`,
  ];
}

function findParentCluster(
  occurrence: ReportOccurrence,
  store: ReportStore,
): ReportCluster | undefined {
  return store.current.report?.clusters.find((c) =>
    c.occurrences.some(
      (o) => o.path === occurrence.path && o.start_byte === occurrence.start_byte,
    ),
  );
}

function dedupeOccurrences(
  occurrences: ReportOccurrence[],
): ReportOccurrence[] {
  const seen = new Set<string>();
  const out: ReportOccurrence[] = [];
  for (const o of occurrences) {
    const key = `${o.path}:${o.start_byte}:${o.end_byte}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(o);
  }
  return out;
}

function readOccurrenceBytes(occurrence: ReportOccurrence): string {
  try {
    const uri = resolveOccurrenceUri(occurrence.path);
    const content = fs.readFileSync(uri.fsPath, "utf8");
    const buffer = Buffer.from(content, "utf8");
    const clamp = (n: number): number => Math.max(0, Math.min(n, buffer.length));
    return buffer
      .slice(clamp(occurrence.start_byte), clamp(occurrence.end_byte))
      .toString("utf8");
  } catch {
    return "";
  }
}

const LANGUAGE_BY_EXT: Record<string, string> = {
  ".cs": "csharp",
  ".rs": "rust",
  ".py": "python",
  ".ts": "typescript",
  ".tsx": "typescript",
  ".js": "javascript",
  ".dart": "dart",
};

function languageForPath(filePath: string): string {
  const ext = path.extname(filePath).toLowerCase();
  return LANGUAGE_BY_EXT[ext] ?? "";
}
