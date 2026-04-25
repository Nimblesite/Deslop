// Command palette + gutter interactions. Every command forwards to the LSP
// or opens a webview; nothing owns UI-only state.

import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore } from "../reportStore";
import { openClusterPanel, openReportPanel } from "../webview/panels";
import { pickEmbeddingModel } from "./embeddingPicker";
import { ReportCluster, ReportOccurrence } from "../types/report";
import { buildCompareUri } from "../compare/provider";
import { ClusterNode, OccurrenceNode } from "../tree/providers";
import {
  clusterIdForTreeNode,
  copyClusterLocations,
  copyContextForAI,
  copyHumanLocation,
  copySourceSnippet,
  openAllOccurrences,
  revealOccurrenceInExplorer,
} from "./treeMenus";

type ClientFactory = () => LanguageClient | undefined;

export function registerCommands(
  context: vscode.ExtensionContext,
  store: ReportStore,
  clientOf: ClientFactory,
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("deslop.openReport", () => openReportPanel(context, store)),
    vscode.commands.registerCommand("deslop.openWorstCluster", () =>
      openWorstCluster(context, store),
    ),
    vscode.commands.registerCommand("deslop.openCluster", (id: string) =>
      openClusterPanel(context, store, id),
    ),
    vscode.commands.registerCommand(
      "deslop.openOccurrence",
      async (target: unknown) => {
        const occurrence = occurrenceFromCommandTarget(target);
        if (!occurrence) {
          void vscode.window.showInformationMessage(
            "Deslop: no occurrence resolved for this command.",
          );
          return;
        }
        await openOccurrence(occurrence);
      },
    ),
    vscode.commands.registerCommand("deslop.jumpToNextOccurrence", () =>
      jumpToNextOccurrence(store),
    ),
    vscode.commands.registerCommand("deslop.compareWithCanonical", (id: string) =>
      compareWithCanonical(store, id),
    ),
    vscode.commands.registerCommand("deslop.pickEmbeddingModel", () =>
      pickEmbeddingModel(store, clientOf),
    ),
    vscode.commands.registerCommand("deslop.refreshReport", () =>
      clientOf()?.sendRequest("workspace/executeCommand", {
        command: "deslop.refreshReport",
        arguments: [],
      }),
    ),
    vscode.commands.registerCommand("deslop.toggleShowAllLenses", async () => {
      const cfg = vscode.workspace.getConfiguration("deslop");
      const next = !cfg.get<boolean>("showAllLenses", false);
      await cfg.update("showAllLenses", next, vscode.ConfigurationTarget.Workspace);
    }),
    vscode.commands.registerCommand("deslop.showSchemaDoc", () =>
      openSchemaDoc(context, store, clientOf),
    ),
    vscode.commands.registerCommand(
      "deslop.copyContextForAI",
      (node: ClusterNode | OccurrenceNode) => copyContextForAI(node, store),
    ),
    vscode.commands.registerCommand(
      "deslop.copyHumanLocation",
      (node: OccurrenceNode) => copyHumanLocation(node),
    ),
    vscode.commands.registerCommand(
      "deslop.copyClusterLocations",
      (node: ClusterNode) => copyClusterLocations(node),
    ),
    vscode.commands.registerCommand(
      "deslop.copySourceSnippet",
      (node: OccurrenceNode) => copySourceSnippet(node),
    ),
    vscode.commands.registerCommand(
      "deslop.revealOccurrenceInExplorer",
      (node: OccurrenceNode) => revealOccurrenceInExplorer(node),
    ),
    vscode.commands.registerCommand(
      "deslop.openAllOccurrences",
      (node: ClusterNode) => openAllOccurrences(node),
    ),
    vscode.commands.registerCommand(
      "deslop.openClusterDetails",
      (node: ClusterNode | OccurrenceNode) => {
        const id = clusterIdForTreeNode(node, store);
        if (!id) {
          void vscode.window.showInformationMessage(
            "Deslop: no cluster resolved for this tree row.",
          );
          return;
        }
        openClusterPanel(context, store, id);
      },
    ),
  );
}

export function openWorstCluster(ctx: vscode.ExtensionContext, store: ReportStore): void {
  const report = store.current.report;
  const worst = report?.clusters[0];
  if (!worst) {
    vscode.window.showInformationMessage("Deslop: no duplication detected.");
    return;
  }
  openClusterPanel(ctx, store, worst.id);
}

export function resolveOccurrenceUri(occurrencePath: string): vscode.Uri {
  if (path.isAbsolute(occurrencePath)) return vscode.Uri.file(occurrencePath);
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const resolved = root ? path.join(root, occurrencePath) : occurrencePath;
  return vscode.Uri.file(resolved);
}

export async function openOccurrence(occurrence: ReportOccurrence): Promise<void> {
  const uri = resolveOccurrenceUri(occurrence.path);
  const doc = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  const start = byteToPosition(doc, occurrence.start_byte);
  const end = byteToPosition(doc, occurrence.end_byte);
  editor.revealRange(new vscode.Range(start, end), vscode.TextEditorRevealType.InCenter);
  editor.selection = new vscode.Selection(start, end);
}

function occurrenceFromCommandTarget(target: unknown): ReportOccurrence | undefined {
  if (isOccurrenceNode(target)) return target.occurrence;
  return isReportOccurrence(target) ? target : undefined;
}

function isOccurrenceNode(target: unknown): target is OccurrenceNode {
  if (typeof target !== "object" || target === null || !("occurrence" in target)) {
    return false;
  }
  return isReportOccurrence(target.occurrence);
}

function isReportOccurrence(target: unknown): target is ReportOccurrence {
  if (typeof target !== "object" || target === null) return false;
  const occurrence = target as Partial<ReportOccurrence>;
  return (
    typeof occurrence.path === "string" &&
    typeof occurrence.start_byte === "number" &&
    typeof occurrence.end_byte === "number"
  );
}

export function jumpToNextOccurrence(store: ReportStore): void {
  const editor = vscode.window.activeTextEditor;
  const report = store.current.report;
  if (!editor || !report) return;
  const here = editor.selection.active;
  const activePath = editor.document.uri.fsPath;
  const cluster = findClusterContaining(report.clusters, activePath, editor.document, here);
  if (!cluster) {
    vscode.window.showInformationMessage("Deslop: no cluster at cursor.");
    return;
  }
  const others = cluster.occurrences.filter((o) => !sameFile(o.path, activePath));
  const next = others[0] ?? cluster.occurrences[0];
  if (!next) return;
  openOccurrence(next).catch(() => undefined);
}

export async function compareWithCanonical(store: ReportStore, clusterId: string): Promise<void> {
  const cluster = store.current.report?.clusters.find((c) => c.id === clusterId);
  if (!cluster || cluster.occurrences.length < 2) return;
  const [a, b] = cluster.occurrences;
  if (!a || !b) return;
  // Always diff occurrence bytes via the deslop-compare provider — same-file
  // clusters would otherwise collapse to "whole file vs. itself" because
  // `vscode.diff` dedupes identical URIs into a single editor pane.
  await vscode.commands.executeCommand(
    "vscode.diff",
    buildCompareUri(a, "a", cluster.id),
    buildCompareUri(b, "b", cluster.id),
    `Compare (cluster ${cluster.id})`,
  );
}

export async function openSchemaDoc(
  ctx: vscode.ExtensionContext,
  store: ReportStore,
  clientOf?: ClientFactory,
): Promise<void> {
  // Live wire blanks `schema_doc` to keep reportGet tiny. Prefer the
  // dedicated `deslop/reportSchemaDoc` RPC, then whatever the snapshot
  // happens to carry, then the packaged markdown copy for offline use.
  const remote = await fetchSchemaDocViaRpc(clientOf);
  const fallback = store.current.report?.schema_doc;
  const packaged = firstNonEmpty(remote, fallback)
    ? undefined
    : await readPackagedSchemaDoc(ctx);
  const content =
    firstNonEmpty(remote, fallback, packaged) ?? "Schema doc unavailable.";
  const doc = await vscode.workspace.openTextDocument({
    language: "markdown",
    content,
  });
  await vscode.window.showTextDocument(doc, { preview: true });
  void ctx;
}

async function fetchSchemaDocViaRpc(clientOf?: ClientFactory): Promise<string | undefined> {
  const client = clientOf?.();
  if (!client) return undefined;
  try {
    const text = await client.sendRequest<string>("deslop/reportSchemaDoc");
    return typeof text === "string" && text.length > 0 ? text : undefined;
  } catch {
    return undefined;
  }
}

async function readPackagedSchemaDoc(
  ctx: vscode.ExtensionContext,
): Promise<string | undefined> {
  try {
    const uri = vscode.Uri.joinPath(ctx.extensionUri, "dist", "schema_doc.md");
    const bytes = await vscode.workspace.fs.readFile(uri);
    const text = Buffer.from(bytes).toString("utf8");
    return text.length > 0 ? text : undefined;
  } catch {
    return undefined;
  }
}

function firstNonEmpty(...values: (string | undefined)[]): string | undefined {
  return values.find((v) => typeof v === "string" && v.length > 0);
}

export function findClusterContaining(
  clusters: ReportCluster[],
  filePath: string,
  document: vscode.TextDocument,
  position: vscode.Position,
): ReportCluster | undefined {
  const byte = utf8ByteOffset(document, position);
  return clusters.find((c) =>
    c.occurrences.some(
      (o) => sameFile(o.path, filePath) && byte >= o.start_byte && byte <= o.end_byte,
    ),
  );
}

export function byteToPosition(doc: vscode.TextDocument, byte: number): vscode.Position {
  const buffer = Buffer.from(doc.getText(), "utf8");
  const slice = buffer.slice(0, Math.min(byte, buffer.length)).toString("utf8");
  return doc.positionAt(slice.length);
}

export function utf8ByteOffset(doc: vscode.TextDocument, position: vscode.Position): number {
  return Buffer.byteLength(
    doc.getText(new vscode.Range(new vscode.Position(0, 0), position)),
    "utf8",
  );
}

function sameFile(a: string, b: string): boolean {
  if (a === b) return true;
  return a.endsWith(b) || b.endsWith(a);
}
