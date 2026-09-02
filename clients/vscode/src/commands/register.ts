// Command palette + gutter interactions. Every command forwards to the LSP
// or opens a webview; nothing owns UI-only state.

import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { ReportStore } from "../reportStore";
import { sameFile } from "../pathUtils";
import { openClusterPanel, openDuplicationReportPanel, openReportPanel } from "../webview/panels";
import { showHtmlReport } from "../webview/htmlReport";
import { pickEmbeddingModel } from "./embeddingPicker";
import {
  chooseTopOffendersFilter,
  clearTopOffendersFilter,
  setTopOffendersGroupBy,
  setTopOffendersSortBy,
} from "./topOffendersView";
import { Report, ReportCluster, ReportOccurrence } from "../types/report";
import { buildCompareUri, CompareEndpointRef } from "../compare/provider";
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

const LSP_REFRESH_REPORT_COMMAND = "deslop.lsp.refreshReport";
const LSP_RENDER_HTML_REPORT_COMMAND = "deslop.lsp.renderHtmlReport";
const UTF8_ENCODING = "utf8";
const WORKSPACE_EXECUTE_COMMAND_METHOD = "workspace/executeCommand";
const LSP_CLIENT_NOT_READY_MESSAGE = "Deslop: LSP client is not ready.";
const SHOW_ALL_LENSES_SETTING = "showAllLenses";
const MARKDOWN_LANGUAGE = "markdown";
const UNKNOWN_VALUE = "unknown";

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNumber(value: unknown): value is number {
  return typeof value === "number";
}

function isObject(value: unknown): value is object {
  return typeof value === "object" && value !== null;
}

interface CommandDeps {
  readonly context: vscode.ExtensionContext;
  readonly store: ReportStore;
  readonly clientOf: ClientFactory;
}

interface CommandBinding {
  readonly id: string;
  readonly run: (deps: CommandDeps, ...args: unknown[]) => unknown;
}

// [VSIX-COMMANDS] Single source of truth for every command-palette entry.
const COMMAND_BINDINGS: readonly CommandBinding[] = [
  { id: "deslop.openReport", run: ({ context, store }) => openReportPanel(context, store) },
  { id: "deslop.openWorstCluster", run: ({ context, store }) => openWorstCluster(context, store) },
  { id: "deslop.openCluster", run: ({ context, store }, id) => openClusterPanel(context, store, id as string) },
  { id: "deslop.openOccurrence", run: (_deps, target) => openOccurrenceTarget(target) },
  { id: "deslop.pickEmbeddingModel", run: ({ store, clientOf }) => pickEmbeddingModel(store, clientOf) },
  { id: "deslop.refreshReport", run: ({ clientOf }) => refreshReport(clientOf) },
  { id: "deslop.toggleShowAllLenses", run: toggleShowAllLenses },
  { id: "deslop.showSchemaDoc", run: ({ context, store, clientOf }) => openSchemaDoc(context, store, clientOf) },
  { id: "deslop.revealCpuReport", run: ({ clientOf }) => openCpuReport(clientOf) },
  // [VSIX-CODE-LENS] The lens "Jump" action cycles occurrences without
  // routing through textDocument/definition ([LSP-NON-INTERFERENCE]).
  { id: "deslop.jumpToNextOccurrence", run: ({ store }, clusterId, occurrenceIndex) => jumpToNextOccurrence(store, clusterId, occurrenceIndex) },
  { id: "deslop.comparePair", run: (_deps, left, right) => comparePairEndpoints(left, right) },
  { id: "deslop.openAllOccurrences", run: (_deps, node) => openAllOccurrences(node as ClusterNode) },
  { id: "deslop.openCanonicalFile", run: (_deps, node) => openCanonicalOccurrence(node as ClusterNode) },
  { id: "deslop.openClusterDetails", run: ({ context, store }, node) => openClusterDetails(context, store, node as ClusterNode | OccurrenceNode) },
  { id: "deslop.topOffenders.showByCluster", run: () => setTopOffendersGroupBy("cluster") },
  { id: "deslop.topOffenders.showByFile", run: () => setTopOffendersGroupBy("file") },
  { id: "deslop.topOffenders.showByFolder", run: () => setTopOffendersGroupBy("folder") },
  { id: "deslop.topOffenders.showBySeverity", run: () => setTopOffendersGroupBy("severity") },
  { id: "deslop.topOffenders.chooseFilter", run: ({ store }) => chooseTopOffendersFilter(store) },
  // Same handler as chooseFilter; separate id so the toolbar can swap in
  // the active-filter icon via the deslop.topOffendersFiltered context key.
  { id: "deslop.topOffenders.chooseFilterActive", run: ({ store }) => chooseTopOffendersFilter(store) },
  { id: "deslop.topOffenders.clearFilter", run: () => clearTopOffendersFilter() },
  { id: "deslop.topOffenders.sortByImpact", run: () => setTopOffendersSortBy("impact") },
  { id: "deslop.topOffenders.sortByPath", run: () => setTopOffendersSortBy("path") },
  { id: "deslop.openDuplicationReport", run: ({ context, store }) => openDuplicationReportPanel(context, store) },
  { id: "deslop.openHtmlReport", run: ({ clientOf }) => openHtmlReport(clientOf) },
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

export function refreshReport(clientOf: ClientFactory): Thenable<unknown> | undefined {
  return clientOf()?.sendRequest(WORKSPACE_EXECUTE_COMMAND_METHOD, {
    command: LSP_REFRESH_REPORT_COMMAND,
    arguments: [],
  });
}

// [OUTPUT-HUMAN-HTML] Asks the LSP to render the full standalone HTML report
// and shows it in an in-editor browser tab. The renderer lives in the engine,
// so neither this client nor the JetBrains plugins re-implement it. The render
// is synchronous on the LSP side and can be slow on large workspaces, so the
// round-trip runs under a progress notification that clears when the tab opens
// or on error — otherwise the click reads as a frozen UI (#256).
export async function openHtmlReport(clientOf: ClientFactory): Promise<void> {
  const client = clientOf();
  if (!client) {
    void vscode.window.showInformationMessage(LSP_CLIENT_NOT_READY_MESSAGE);
    return;
  }
  const html = await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: "Deslop: rendering HTML report…" },
    () =>
      client.sendRequest<string>(WORKSPACE_EXECUTE_COMMAND_METHOD, {
        command: LSP_RENDER_HTML_REPORT_COMMAND,
        arguments: [],
      }),
  );
  if (!isString(html) || html.length === 0) {
    void vscode.window.showInformationMessage("Deslop: no HTML report available yet.");
    return;
  }
  showHtmlReport(html);
}

export async function toggleShowAllLenses(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  const next = !cfg.get<boolean>(SHOW_ALL_LENSES_SETTING, false);
  await cfg.update(SHOW_ALL_LENSES_SETTING, next, vscode.ConfigurationTarget.Workspace);
}

export async function openOccurrenceTarget(target: unknown): Promise<void> {
  const occurrence = occurrenceFromCommandTarget(target);
  if (occurrence) await openOccurrence(occurrence);
  else void vscode.window.showInformationMessage("Deslop: no occurrence resolved for this command.");
}

export async function copyClusterContextById(store: ReportStore, id: unknown): Promise<void> {
  const clusterId = isString(id) ? id : String(id);
  const cluster = store.current.report?.clusters.find((c) => c.id === clusterId);
  if (!cluster) return;
  const rank = (store.current.report?.clusters.indexOf(cluster) ?? -1) + 1;
  await vscode.env.clipboard.writeText(aiPayloadForCluster(cluster, rank));
  void vscode.window.showInformationMessage("Copied AI context to clipboard");
}

export function openClusterDetails(
  context: vscode.ExtensionContext,
  store: ReportStore,
  node: ClusterNode | OccurrenceNode,
): void {
  const id = clusterIdForTreeNode(node, store);
  if (id) openClusterPanel(context, store, id);
  else void vscode.window.showInformationMessage("Deslop: no cluster resolved for this tree row.");
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
  if (!isObject(target) || !("occurrence" in target)) {
    return false;
  }
  return isReportOccurrence(target.occurrence);
}

function isReportOccurrence(target: unknown): target is ReportOccurrence {
  if (!isObject(target)) return false;
  const occurrence = target as Partial<ReportOccurrence>;
  return (
    isString(occurrence.path) &&
    isNumber(occurrence.start_byte) &&
    isNumber(occurrence.end_byte)
  );
}

export async function jumpToNextOccurrence(
  store: ReportStore,
  clusterId?: unknown,
  occurrenceIndex?: unknown,
): Promise<void> {
  const report = store.current.report;
  if (!report) return;
  const commandTarget = occurrenceAfterCommandIndex(report, clusterId, occurrenceIndex);
  if (commandTarget) {
    await openOccurrence(commandTarget).catch(() => undefined);
    return;
  }
  if (clusterId !== undefined || occurrenceIndex !== undefined) return;

  const editor = vscode.window.activeTextEditor;
  if (!editor) return;
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
  await openOccurrence(next).catch(() => undefined);
}

function occurrenceAfterCommandIndex(
  report: Report,
  clusterId: unknown,
  occurrenceIndex: unknown,
): ReportOccurrence | undefined {
  if (
    !isString(clusterId) ||
    !isNumber(occurrenceIndex) ||
    !Number.isInteger(occurrenceIndex) ||
    occurrenceIndex < 0
  ) {
    return undefined;
  }
  const cluster = report.clusters.find((candidate) => candidate.id === clusterId);
  if (!cluster?.occurrences.length) return undefined;
  return cluster.occurrences[(occurrenceIndex + 1) % cluster.occurrences.length];
}

// [VSIX-PAIR-COMPARE] Compare exists only between two occurrences the user
// selected explicitly. There is no canonical fallback: a missing, malformed,
// or identical endpoint pair is a no-op, never an implicit comparison.
const COMPARE_DIFF_TITLE = "Compare selected occurrences";

export async function comparePairEndpoints(left: unknown, right: unknown): Promise<void> {
  const leftEndpoint = compareEndpoint(left);
  const rightEndpoint = compareEndpoint(right);
  if (!leftEndpoint || !rightEndpoint || sameEndpoint(leftEndpoint, rightEndpoint)) return;
  await openCompareDiff(leftEndpoint, rightEndpoint);
}

function compareEndpoint(value: unknown): CompareEndpointRef | undefined {
  if (!isObject(value)) return undefined;
  const candidate = value as Partial<CompareEndpointRef>;
  if (typeof candidate.path !== "string" || candidate.path.length === 0) return undefined;
  if (typeof candidate.start_byte !== "number" || !Number.isInteger(candidate.start_byte)) return undefined;
  if (typeof candidate.end_byte !== "number" || !Number.isInteger(candidate.end_byte)) return undefined;
  return { path: candidate.path, start_byte: candidate.start_byte, end_byte: candidate.end_byte };
}

function sameEndpoint(left: CompareEndpointRef, right: CompareEndpointRef): boolean {
  return (
    left.path === right.path &&
    left.start_byte === right.start_byte &&
    left.end_byte === right.end_byte
  );
}

async function openCompareDiff(a: CompareEndpointRef, b: CompareEndpointRef): Promise<void> {
  await vscode.commands.executeCommand(
    "vscode.diff",
    buildCompareUri(a, "a"),
    buildCompareUri(b, "b"),
    COMPARE_DIFF_TITLE,
  );
}

export async function openSchemaDoc(
  ctx: vscode.ExtensionContext,
  store: ReportStore,
  clientOf?: ClientFactory,
): Promise<void> {
  // The packaged markdown is the current extension-facing reference.
  // RPC/snapshot fallbacks are only for unusual packaging failures;
  // persisted reports may be discarded and recreated when their shape
  // no longer matches.
  const packaged = await readPackagedSchemaDoc(ctx);
  const remote = await fetchSchemaDocViaRpc(clientOf);
  const fallback = store.current.report?.schema_doc;
  const content =
    firstNonEmpty(packaged, remote, fallback) ?? "Schema doc unavailable.";
  const doc = await vscode.workspace.openTextDocument({
    language: MARKDOWN_LANGUAGE,
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
    return isString(text) && text.length > 0 ? text : undefined;
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
    const text = Buffer.from(bytes).toString(UTF8_ENCODING);
    return text.length > 0 ? text : undefined;
  } catch {
    return undefined;
  }
}

function firstNonEmpty(...values: (string | undefined)[]): string | undefined {
  return values.find((value) => isString(value) && value.length > 0);
}

interface CpuPhaseRecord {
  readonly phase?: string;
  readonly started_at_ms?: number;
  readonly duration_ms?: number;
  readonly cpu_ms?: number;
  readonly files_touched?: readonly string[];
}

interface CpuReport {
  readonly current_phase?: string;
  readonly last_100_phases?: readonly CpuPhaseRecord[];
  readonly handler_counts?: Readonly<Record<string, number>>;
  readonly in_flight?: {
    readonly pending_watcher_events?: number;
    readonly pending_embed_requests?: number;
    readonly in_progress_parse_batch?: number | null;
  };
}

export async function openCpuReport(clientOf: ClientFactory): Promise<void> {
  const client = clientOf();
  if (!client) {
    void vscode.window.showInformationMessage(LSP_CLIENT_NOT_READY_MESSAGE);
    return;
  }
  const report = await client.sendRequest<CpuReport>("deslop/cpuReport");
  const doc = await vscode.workspace.openTextDocument({
    language: MARKDOWN_LANGUAGE,
    content: renderCpuReport(report),
  });
  await vscode.window.showTextDocument(doc, { preview: true });
}

export function renderCpuReport(report: CpuReport): string {
  const inFlight = report.in_flight ?? {};
  const handlers = Object.entries(report.handler_counts ?? {}).sort(([left], [right]) =>
    left.localeCompare(right),
  );
  const phases = report.last_100_phases ?? [];
  const lines = [
    "# Deslop CPU Report",
    "",
    `- Current phase: ${report.current_phase ?? UNKNOWN_VALUE}`,
    `- Pending watcher events: ${inFlight.pending_watcher_events ?? 0}`,
    `- Pending embedding requests: ${inFlight.pending_embed_requests ?? 0}`,
    `- In-progress parse batch: ${inFlight.in_progress_parse_batch ?? 0}`,
    "",
    "## Handler Counts",
    "",
    "| Handler | Count |",
    "|---|---:|",
    ...handlers.map(([name, count]) => `| \`${name}\` | ${count} |`),
    "",
    "## Last 100 Phases",
    "",
    "| Phase | Started ms | Wall ms | CPU ms | Files |",
    "|---|---:|---:|---:|---|",
    ...phases.map((phase) => {
      const files = (phase.files_touched ?? []).join(", ");
      return `| ${phase.phase ?? UNKNOWN_VALUE} | ${phase.started_at_ms ?? 0} | ${phase.duration_ms ?? 0} | ${phase.cpu_ms ?? 0} | ${files || "-"} |`;
    }),
  ];
  return lines.join("\n");
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
  const buffer = Buffer.from(doc.getText(), UTF8_ENCODING);
  const slice = buffer.slice(0, Math.min(byte, buffer.length)).toString(UTF8_ENCODING);
  return doc.positionAt(slice.length);
}

export function utf8ByteOffset(doc: vscode.TextDocument, position: vscode.Position): number {
  return Buffer.byteLength(
    doc.getText(new vscode.Range(new vscode.Position(0, 0), position)),
    UTF8_ENCODING,
  );
}
