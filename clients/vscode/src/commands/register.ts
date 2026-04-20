// Command palette + gutter interactions. Every command forwards to the LSP
// or opens a webview; nothing owns UI-only state.

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore } from "../reportStore";
import { openClusterPanel, openReportPanel } from "../webview/panels";
import { pickEmbeddingModel } from "./embeddingPicker";
import { ReportCluster, ReportOccurrence } from "../types/report";

type ClientFactory = () => LanguageClient | undefined;

export function registerCommands(
  context: vscode.ExtensionContext,
  store: ReportStore,
  clientOf: ClientFactory,
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("codededup.openReport", () => openReportPanel(context, store)),
    vscode.commands.registerCommand("codededup.openWorstCluster", () =>
      openWorstCluster(context, store),
    ),
    vscode.commands.registerCommand("codededup.openCluster", (id: string) =>
      openClusterPanel(context, store, id),
    ),
    vscode.commands.registerCommand("codededup.openOccurrence", (o: ReportOccurrence) =>
      openOccurrence(o),
    ),
    vscode.commands.registerCommand("codededup.jumpToNextOccurrence", () =>
      jumpToNextOccurrence(store),
    ),
    vscode.commands.registerCommand("codededup.compareWithCanonical", (id: string) =>
      compareWithCanonical(store, id),
    ),
    vscode.commands.registerCommand("codededup.pickEmbeddingModel", () =>
      pickEmbeddingModel(store, clientOf),
    ),
    vscode.commands.registerCommand("codededup.refreshReport", () =>
      clientOf()?.sendRequest("workspace/executeCommand", {
        command: "codededup.refreshReport",
        arguments: [],
      }),
    ),
    vscode.commands.registerCommand("codededup.toggleShowAllLenses", async () => {
      const cfg = vscode.workspace.getConfiguration("codededup");
      const next = !cfg.get<boolean>("showAllLenses", false);
      await cfg.update("showAllLenses", next, vscode.ConfigurationTarget.Workspace);
    }),
    vscode.commands.registerCommand("codededup.showSchemaDoc", () =>
      openSchemaDoc(context, store),
    ),
  );
}

function openWorstCluster(ctx: vscode.ExtensionContext, store: ReportStore): void {
  const report = store.current.report;
  const worst = report?.clusters[0];
  if (!worst) {
    vscode.window.showInformationMessage("CodeDedup: no duplication detected.");
    return;
  }
  openClusterPanel(ctx, store, worst.id);
}

async function openOccurrence(occurrence: ReportOccurrence): Promise<void> {
  const uri = vscode.Uri.file(occurrence.path);
  const doc = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  const start = byteToPosition(doc, occurrence.start_byte);
  const end = byteToPosition(doc, occurrence.end_byte);
  editor.revealRange(new vscode.Range(start, end), vscode.TextEditorRevealType.InCenter);
  editor.selection = new vscode.Selection(start, end);
}

function jumpToNextOccurrence(store: ReportStore): void {
  const editor = vscode.window.activeTextEditor;
  const report = store.current.report;
  if (!editor || !report) return;
  const here = editor.selection.active;
  const activePath = editor.document.uri.fsPath;
  const cluster = findClusterContaining(report.clusters, activePath, editor.document, here);
  if (!cluster) {
    vscode.window.showInformationMessage("CodeDedup: no cluster at cursor.");
    return;
  }
  const others = cluster.occurrences.filter((o) => !sameFile(o.path, activePath));
  const next = others[0] ?? cluster.occurrences[0];
  if (!next) return;
  openOccurrence(next).catch(() => undefined);
}

async function compareWithCanonical(store: ReportStore, clusterId: string): Promise<void> {
  const cluster = store.current.report?.clusters.find((c) => c.id === clusterId);
  if (!cluster || cluster.occurrences.length < 2) return;
  const [a, b] = cluster.occurrences;
  if (!a || !b) return;
  await vscode.commands.executeCommand(
    "vscode.diff",
    vscode.Uri.file(a.path),
    vscode.Uri.file(b.path),
    `Compare (cluster ${cluster.id})`,
  );
}

async function openSchemaDoc(ctx: vscode.ExtensionContext, store: ReportStore): Promise<void> {
  const doc = await vscode.workspace.openTextDocument({
    language: "markdown",
    content: store.current.report?.schema_doc ?? "Schema doc unavailable.",
  });
  await vscode.window.showTextDocument(doc, { preview: true });
  void ctx;
}

function findClusterContaining(
  clusters: ReportCluster[],
  path: string,
  document: vscode.TextDocument,
  position: vscode.Position,
): ReportCluster | undefined {
  const byte = utf8ByteOffset(document, position);
  return clusters.find((c) =>
    c.occurrences.some(
      (o) => sameFile(o.path, path) && byte >= o.start_byte && byte <= o.end_byte,
    ),
  );
}

function byteToPosition(doc: vscode.TextDocument, byte: number): vscode.Position {
  const buffer = Buffer.from(doc.getText(), "utf8");
  const slice = buffer.slice(0, Math.min(byte, buffer.length)).toString("utf8");
  return doc.positionAt(slice.length);
}

function utf8ByteOffset(doc: vscode.TextDocument, position: vscode.Position): number {
  return Buffer.byteLength(
    doc.getText(new vscode.Range(new vscode.Position(0, 0), position)),
    "utf8",
  );
}

function sameFile(a: string, b: string): boolean {
  if (a === b) return true;
  return a.endsWith(b) || b.endsWith(a);
}
