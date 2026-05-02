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
  aiPayloadForCluster,
  canonicalOccurrenceForCluster,
  clusterIdForTreeNode,
  copyClusterLocations,
  copyContextForAI,
  copyHumanLocation,
  copySourceSnippet,
  openAllOccurrences,
  revealOccurrenceInExplorer,
} from "./treeMenus";

type ClientFactory = () => LanguageClient | undefined;

interface CommandDeps {
  readonly context: vscode.ExtensionContext;
  readonly store: ReportStore;
  readonly clientOf: ClientFactory;
}

interface CommandBinding {
  readonly id: string;
  readonly run: (deps: CommandDeps, ...args: unknown[]) => unknown;
}

const COMMAND_BINDINGS: readonly CommandBinding[] = [
  { id: "deslop.openReport", run: ({ context, store }) => openReportPanel(context, store) },
  { id: "deslop.openWorstCluster", run: ({ context, store }) => openWorstCluster(context, store) },
  { id: "deslop.openCluster", run: ({ context, store }, id) => openClusterPanel(context, store, id as string) },
  { id: "deslop.openOccurrence", run: (_deps, target) => openOccurrenceTarget(target) },
  { id: "deslop.pickEmbeddingModel", run: ({ store, clientOf }) => pickEmbeddingModel(store, clientOf) },
  { id: "deslop.refreshReport", run: ({ clientOf }) => refreshReport(clientOf) },
  { id: "deslop.toggleShowAllLenses", run: toggleShowAllLenses },
  { id: "deslop.showSchemaDoc", run: ({ context, store, clientOf }) => openSchemaDoc(context, store, clientOf) },
  { id: "deslop.jumpToNextOccurrence", run: ({ store }) => jumpToNextOccurrence(store) },
  { id: "deslop.compareWithCanonical", run: ({ store }, target) => compareWithCanonicalTarget(store, target) },
  { id: "deslop.compareOccurrenceWithCanonical", run: ({ store }, target) => compareWithCanonicalTarget(store, target) },
  { id: "deslop.openAllOccurrences", run: (_deps, node) => openAllOccurrences(node as ClusterNode) },
  { id: "deslop.openCanonicalFile", run: (_deps, node) => openCanonicalOccurrence(node as ClusterNode) },
  { id: "deslop.openClusterDetails", run: ({ context, store }, node) => openClusterDetails(context, store, node as ClusterNode | OccurrenceNode) },
  { id: "deslop.topOffenders.showByCluster", run: () => setTopOffendersGroupBy("cluster") },
  { id: "deslop.topOffenders.showByFile", run: () => setTopOffendersGroupBy("file") },
  { id: "deslop.copyContextForAI", run: ({ store }, node) => copyContextForAI(node as ClusterNode | OccurrenceNode, store) },
  { id: "deslop.copyClusterContextById", run: ({ store }, id) => copyClusterContextById(store, id) },
  { id: "deslop.copyHumanLocation", run: (_deps, node) => copyHumanLocation(node as OccurrenceNode) },
  { id: "deslop.copyClusterLocations", run: (_deps, node) => copyClusterLocations(node as ClusterNode) },
  { id: "deslop.copySourceSnippet", run: (_deps, node) => copySourceSnippet(node as OccurrenceNode) },
  { id: "deslop.revealOccurrenceInExplorer", run: (_deps, node) => revealOccurrenceInExplorer(node as OccurrenceNode) },
];

export function registerCommands(
  context: vscode.ExtensionContext,
  store: ReportStore,
  clientOf: ClientFactory,
): void {
  const deps = { context, store, clientOf };
  for (const binding of COMMAND_BINDINGS) {
    context.subscriptions.push(
      vscode.commands.registerCommand(binding.id, (...args: unknown[]) =>
        binding.run(deps, ...args),
      ),
    );
  }
}

function refreshReport(clientOf: ClientFactory): Thenable<unknown> | undefined {
  return clientOf()?.sendRequest("workspace/executeCommand", {
    command: "deslop.refreshReport",
    arguments: [],
  });
}

async function toggleShowAllLenses(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  const next = !cfg.get<boolean>("showAllLenses", false);
  await cfg.update("showAllLenses", next, vscode.ConfigurationTarget.Workspace);
}

async function openOccurrenceTarget(target: unknown): Promise<void> {
  const occurrence = occurrenceFromCommandTarget(target);
  if (occurrence) await openOccurrence(occurrence);
  else void vscode.window.showInformationMessage("Deslop: no occurrence resolved for this command.");
}

async function copyClusterContextById(store: ReportStore, id: unknown): Promise<void> {
  const clusterId = typeof id === "string" ? id : String(id);
  const cluster = store.current.report?.clusters.find((c) => c.id === clusterId);
  if (!cluster) return;
  const rank = (store.current.report?.clusters.indexOf(cluster) ?? -1) + 1;
  await vscode.env.clipboard.writeText(aiPayloadForCluster(cluster, rank));
  void vscode.window.showInformationMessage("Copied AI context to clipboard");
}

function openClusterDetails(
  context: vscode.ExtensionContext,
  store: ReportStore,
  node: ClusterNode | OccurrenceNode,
): void {
  const id = clusterIdForTreeNode(node, store);
  if (id) openClusterPanel(context, store, id);
  else void vscode.window.showInformationMessage("Deslop: no cluster resolved for this tree row.");
}

async function setTopOffendersGroupBy(value: "cluster" | "file"): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update(
    "topOffenders.groupBy",
    value,
    vscode.ConfigurationTarget.Workspace,
  );
}

export async function openCanonicalOccurrence(node: ClusterNode): Promise<void> {
  const occurrence = canonicalOccurrenceForCluster(node);
  if (!occurrence) {
    void vscode.window.showInformationMessage(
      "Deslop: no canonical occurrence resolved for this cluster.",
    );
    return;
  }
  await openOccurrence(occurrence);
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

export async function compareWithCanonicalTarget(
  store: ReportStore,
  target: unknown,
): Promise<void> {
  if (isOccurrenceNode(target)) {
    const selection = selectedOccurrenceCompare(store, target.occurrence);
    if (!selection) return;
    await openCompareDiff(selection.cluster.id, selection.canonical, selection.selected);
    return;
  }
  const clusterId = clusterIdFromCompareTarget(store, target);
  if (!clusterId) return;
  await compareWithCanonical(store, clusterId);
}

interface OccurrenceCompareSelection {
  readonly cluster: ReportCluster;
  readonly canonical: ReportOccurrence;
  readonly selected: ReportOccurrence;
}

function selectedOccurrenceCompare(
  store: ReportStore,
  occurrence: ReportOccurrence,
): OccurrenceCompareSelection | undefined {
  const cluster = parentClusterForOccurrence(store, occurrence);
  const canonical = cluster?.occurrences[0];
  const selected = cluster?.occurrences.find((candidate) =>
    sameOccurrence(candidate, occurrence),
  );
  if (!cluster || !canonical || !selected || sameOccurrence(canonical, selected)) {
    return undefined;
  }
  return { cluster, canonical, selected };
}

function parentClusterForOccurrence(
  store: ReportStore,
  occurrence: ReportOccurrence,
): ReportCluster | undefined {
  return store.current.report?.clusters.find((cluster) =>
    cluster.occurrences.some((candidate) => sameOccurrence(candidate, occurrence)),
  );
}

function sameOccurrence(left: ReportOccurrence, right: ReportOccurrence): boolean {
  return (
    left.path === right.path &&
    left.start_byte === right.start_byte &&
    left.end_byte === right.end_byte
  );
}

function clusterIdFromCompareTarget(
  store: ReportStore,
  target: unknown,
): string | undefined {
  if (typeof target === "string") return target;
  if (isCompareTreeTarget(target)) return clusterIdForTreeNode(target, store);
  return undefined;
}

function isCompareTreeTarget(
  target: unknown,
): target is ClusterNode | OccurrenceNode {
  return isOccurrenceNode(target) || isClusterNode(target);
}

function isClusterNode(target: unknown): target is ClusterNode {
  if (typeof target !== "object" || target === null || !("cluster" in target)) {
    return false;
  }
  const cluster = (target as Partial<ClusterNode>).cluster as
    | Partial<ReportCluster>
    | undefined;
  return typeof cluster?.id === "string" && Array.isArray(cluster.occurrences);
}

export async function compareWithCanonical(store: ReportStore, clusterId: string): Promise<void> {
  const cluster = store.current.report?.clusters.find((c) => c.id === clusterId);
  if (!cluster || cluster.occurrences.length < 2) return;
  const [a, b] = cluster.occurrences;
  if (!a || !b) return;
  await openCompareDiff(cluster.id, a, b);
}

async function openCompareDiff(
  clusterId: string,
  a: ReportOccurrence,
  b: ReportOccurrence,
): Promise<void> {
  // Always diff occurrence bytes via the deslop-compare provider — same-file
  // clusters would otherwise collapse to "whole file vs. itself" because
  // `vscode.diff` dedupes identical URIs into a single editor pane.
  await vscode.commands.executeCommand(
    "vscode.diff",
    buildCompareUri(a, "a", clusterId),
    buildCompareUri(b, "b", clusterId),
    `Compare (cluster ${clusterId})`,
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
