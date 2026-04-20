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
    vscode.commands.registerCommand("deslop.openOccurrence", (o: ReportOccurrence) =>
      openOccurrence(o),
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
      openSchemaDoc(context, store),
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

export async function openSchemaDoc(ctx: vscode.ExtensionContext, store: ReportStore): Promise<void> {
  const doc = await vscode.workspace.openTextDocument({
    language: "markdown",
    content: store.current.report?.schema_doc ?? "Schema doc unavailable.",
  });
  await vscode.window.showTextDocument(doc, { preview: true });
  void ctx;
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
