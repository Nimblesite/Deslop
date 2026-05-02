// Deslop VSIX entry point. Per CLAUDE.md: < 500 lines, thin glue,
// all UI logic split across bubble/, tree/, decorations/, commands/, webview/.

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  Middleware,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

import {
  BundledBinaryMissingError,
  UnsupportedPlatformError,
  ResolvedBinary,
  loadDeploymentManifest,
  resolveBinary,
  resolveHostBinaries,
  BinaryVerificationError,
  BinaryMissingError,
  DeploymentManifest,
  BinarySettings,
} from "./binary";
import { log, logError, initOutputChannel } from "./logging";
import { ReportStore } from "./reportStore";
import { registerCommands } from "./commands/register";
import {
  TopOffendersProvider,
  FocusedFileProvider,
  SessionProvider,
  StatusTicker,
} from "./tree/providers";
import { ClusterHoverProvider } from "./decorations/clusterHoverProvider";
import { DecorationManager } from "./decorations/manager";
import { LiveBubble } from "./bubble/live";
import { StatusBar } from "./commands/statusBar";
import { registerCompareProvider } from "./compare/provider";
import { registerClusterDocumentProvider } from "./clusterDocument";
import {
  Report,
  ReportChangedNotification,
  ReportDelta,
  AnalysisState,
  EmbeddingProgress,
} from "./types/report";

let client: LanguageClient | undefined;
let resolvedLsp: ResolvedBinary | undefined;
let resolvedMcp: ResolvedBinary | undefined;
let activeReportStore: ReportStore | undefined;

const ANALYSED_DOCUMENTS = [
  { language: "csharp", scheme: "file" },
  { language: "rust", scheme: "file" },
  { language: "python", scheme: "file" },
];

const PRODUCTION_EMBEDDING_PROVIDER = "ollama";
const DEFAULT_EMBEDDING_MODEL = "nomic-embed-text";
const DEFAULT_EMBEDDING_ENDPOINT = "http://127.0.0.1:11434";
const DEFAULT_EMBEDDING_MODE = "off";

const HOVER_SUPPRESSING_MIDDLEWARE = {
  provideHover: () => null,
} satisfies Middleware;

interface EmbeddingSettings {
  readonly provider: typeof PRODUCTION_EMBEDDING_PROVIDER;
  readonly model: string;
  readonly endpoint: string;
  readonly mode: string;
}

/// Public API returned by `activate()`. Lets tests reach the live
/// LanguageClient without parallel activation or command-surface hacks.
export interface ExtensionApi {
  readonly client: LanguageClient | undefined;
  readonly resolvedLsp: ResolvedBinary | undefined;
  readonly resolvedMcp: ResolvedBinary | undefined;
  readonly reportStore: ReportStore | undefined;
}

export async function activate(
  context: vscode.ExtensionContext,
): Promise<ExtensionApi> {
  initOutputChannel();
  log("extension activating", {
    extensionPath: context.extensionPath,
    version: currentExtensionVersion(context),
    platform: process.platform,
    arch: process.arch,
  });

  const reportStore = new ReportStore();
  activeReportStore = reportStore;
  context.subscriptions.push(reportStore);
  context.subscriptions.push({
    dispose: () => {
      activeReportStore = undefined;
    },
  });
  registerClusterDocumentProvider(context, reportStore);

  const ticker = new StatusTicker();
  context.subscriptions.push(ticker);

  // [VSIX-TOP-OFFENDERS-GROUPING] Seed the context key synchronously
  // BEFORE the tree provider is registered so the title-bar toggle
  // button has a `when`-clause value to match against on cold start.
  syncTopOffendersGroupByContext();

  const topOffenders = new TopOffendersProvider(reportStore, ticker);
  const focusedFile = new FocusedFileProvider(reportStore, ticker);
  const session = new SessionProvider(reportStore, ticker, () => client);
  context.subscriptions.push(
    topOffenders,
    focusedFile,
    session,
    vscode.window.registerTreeDataProvider("deslop.topOffenders", topOffenders),
    vscode.window.registerTreeDataProvider("deslop.focusedFile", focusedFile),
    vscode.window.registerTreeDataProvider("deslop.session", session),
    vscode.commands.registerCommand("deslop.revealLog", () =>
      initOutputChannel().show(true),
    ),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("deslop.topOffenders.groupBy")) return;
      syncTopOffendersGroupByContext();
      topOffenders.refresh();
    }),
  );

  try {
    const manifest = loadDeploymentManifest(context.extensionPath);
    const resolved = resolveHostBinaries(
      context.extensionPath,
      "vscode",
      manifest,
      currentBinarySettings(),
    );
    resolvedLsp = requireResolved(resolved, "deslop-lsp");
    resolvedMcp = resolved["deslop-mcp"];
    log("lsp resolved", {
      path: resolvedLsp.path,
      source: resolvedLsp.source,
      version: resolvedLsp.version,
    });
    if (resolvedMcp) {
      log("mcp resolved", {
        path: resolvedMcp.path,
        source: resolvedMcp.source,
        version: resolvedMcp.version,
      });
    }
    client = startLanguageClient(resolvedLsp);
  } catch (err) {
    surfaceStartupFailure(err, reportStore);
    return currentApi();
  }

  context.subscriptions.push(wireDirtyDocuments(reportStore));

  const decorations = new DecorationManager(reportStore);
  context.subscriptions.push(decorations);

  context.subscriptions.push(
    vscode.languages.registerHoverProvider(
      ANALYSED_DOCUMENTS,
      new ClusterHoverProvider(reportStore),
    ),
  );

  const bubble = new LiveBubble(reportStore, () => client);
  context.subscriptions.push(bubble);

  const statusBar = new StatusBar(reportStore);
  context.subscriptions.push(statusBar);

  registerCompareProvider(context);
  registerCommands(context, reportStore, () => client);
  context.subscriptions.push(
    vscode.commands.registerCommand("deslop.revealActiveBinary", () =>
      revealActiveBinary(resolvedLsp, resolvedMcp),
    ),
  );

  reportStore.setLifecycle({ kind: "analysing" });
  try {
    await client.start();
  } catch (err) {
    surfaceStartupFailure(err, reportStore);
    return currentApi();
  }
  wireNotifications(client, reportStore);
  await seedInitialReport(client, reportStore);
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("deslop.embedding")) return;
      syncEmbeddingSettingsToLsp(reportStore, () => client).catch(
        (err: unknown) => logError(err, "sync embedding settings to LSP"),
      );
    }),
  );
  return currentApi();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function currentApi(): ExtensionApi {
  return {
    get client() {
      return client;
    },
    get resolvedLsp() {
      return resolvedLsp;
    },
    get resolvedMcp() {
      return resolvedMcp;
    },
    get reportStore() {
      return activeReportStore;
    },
  };
}

// [VSIX-TOP-OFFENDERS-GROUPING] Mirror the persisted setting onto a
// VS Code context key so the title-bar toggle's mutually exclusive
// `when` clauses can render the right button. Unknown / missing
// values fall back to "cluster" — the spec's default.
function syncTopOffendersGroupByContext(): void {
  const raw = vscode.workspace
    .getConfiguration("deslop")
    .get<string>("topOffenders.groupBy", "cluster");
  const value = raw === "file" ? "file" : "cluster";
  void vscode.commands.executeCommand(
    "setContext",
    "deslop.topOffendersGroupBy",
    value,
  );
}

function startLanguageClient(lsp: ResolvedBinary): LanguageClient {
  const workspaceRoot = resolveWorkspaceRoot();
  const runArgs = buildServerArgs(workspaceRoot, false);
  const debugArgs = buildServerArgs(workspaceRoot, true);
  log("starting language client", {
    lspPath: lsp.path,
    workspaceRoot: workspaceRoot ?? null,
    args: runArgs,
  });
  if (!workspaceRoot) {
    log("no workspace folder open; LSP will have nothing to analyse", {});
  }
  const serverOptions: ServerOptions = {
    run: { command: lsp.path, transport: TransportKind.stdio, args: runArgs },
    debug: {
      command: lsp.path,
      transport: TransportKind.stdio,
      args: debugArgs,
    },
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: ANALYSED_DOCUMENTS,
    synchronize: {
      configurationSection: "deslop",
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{cs,rs,py}"),
    },
    outputChannel: initOutputChannel(),
    initializationOptions: currentInitializationOptions(),
    // The TypeScript ClusterHoverProvider owns the editor hover card.
    // Suppress the LSP textDocument/hover so they don't stack in the popup.
    middleware: HOVER_SUPPRESSING_MIDDLEWARE,
  };
  return new LanguageClient("deslop", "Deslop", serverOptions, clientOptions);
}

export function buildServerArgs(
  workspaceRoot: string | undefined,
  debug: boolean,
): string[] {
  if (!workspaceRoot) return debug ? ["--debug"] : [];
  const args = [workspaceRoot];
  if (debug) args.push("--debug");
  return args;
}

export function resolveWorkspaceRoot(): string | undefined {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) return undefined;
  const first = folders[0];
  return first?.uri.fsPath;
}

export function currentInitializationOptions(): Record<string, unknown> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  const embedding = embeddingSettingsFromConfiguration(cfg);
  return {
    minNodes: cfg.get<number>("minNodes", 30),
    embedding,
    incremental: cfg.get<boolean>("incremental", true),
    configPath: cfg.get<string>("configPath", ""),
  };
}

function embeddingSettingsFromConfiguration(
  cfg: vscode.WorkspaceConfiguration,
): EmbeddingSettings {
  const provider = cfg.get<string>(
    "embedding.provider",
    PRODUCTION_EMBEDDING_PROVIDER,
  );
  const endpoint = cfg.get<string>(
    "embedding.endpoint",
    DEFAULT_EMBEDDING_ENDPOINT,
  );
  if (provider !== PRODUCTION_EMBEDDING_PROVIDER) {
    return {
      provider: PRODUCTION_EMBEDDING_PROVIDER,
      model: DEFAULT_EMBEDDING_MODEL,
      endpoint,
      mode: DEFAULT_EMBEDDING_MODE,
    };
  }
  return {
    provider: PRODUCTION_EMBEDDING_PROVIDER,
    model: cfg.get<string>("embedding.model", DEFAULT_EMBEDDING_MODEL),
    endpoint,
    mode: cfg.get<string>("embedding.mode", DEFAULT_EMBEDDING_MODE),
  };
}

async function refreshAfterChange(
  c: LanguageClient,
  store: ReportStore,
  payload: ReportChangedNotification,
): Promise<void> {
  const delta = await c.sendRequest<ReportDelta | null>("deslop/reportDelta");
  // applyDelta silently bails when no current report exists, which would
  // strand the notification during the startup window before
  // seedInitialReport completes. Fall back to the full snapshot in that case.
  if (delta && store.current.report) {
    store.applyDelta(delta);
    return;
  }
  const snapshot = await c.sendRequest<Report>("deslop/reportGet");
  store.setSnapshot(snapshot, payload.generation);
}

async function refreshAfterEmbedding(
  c: LanguageClient,
  store: ReportStore,
): Promise<void> {
  const snapshot = await c.sendRequest<Report>("deslop/reportGet");
  store.setSnapshot(snapshot, store.current.generation + 1);
}

export function wireNotifications(c: LanguageClient, store: ReportStore): void {
  c.onNotification(
    "deslop/reportChanged",
    (payload: ReportChangedNotification) => {
      store.notifyChange(payload.summary);
      refreshAfterChange(c, store, payload).catch((err: unknown) =>
        logError(err, "refresh report after change"),
      );
    },
  );
  c.onNotification("deslop/analysisState", (state: AnalysisState) => {
    log("analysis state", { state });
    if (state.state === "running") store.setLifecycle({ kind: "analysing" });
    else if (state.state === "idle") store.setLifecycle({ kind: "ready" });
    else if (state.state === "errored") {
      store.setLifecycle({
        kind: "failed",
        message: state.message,
      });
    }
  });
  c.onNotification(
    "deslop/embeddingProgress",
    (progress: EmbeddingProgress) => {
      if (progress.phase === "complete") {
        store.setEmbeddingProgress(null);
        refreshAfterEmbedding(c, store).catch((err: unknown) =>
          logError(err, "refresh report after embedding"),
        );
      } else {
        store.setEmbeddingProgress(progress);
      }
    },
  );
}

export function wireDirtyDocuments(store: ReportStore): vscode.Disposable {
  return vscode.workspace.onDidChangeTextDocument((event) => {
    store.markFileDirty(event.document.uri.fsPath);
  });
}

export async function syncEmbeddingSettingsToLsp(
  store: ReportStore,
  clientOf: () => LanguageClient | undefined,
): Promise<void> {
  const c = clientOf();
  if (!c) return;
  const cfg = vscode.workspace.getConfiguration("deslop");
  const { provider, model, endpoint, mode } = embeddingSettingsFromConfiguration(cfg);
  if (mode === "off") return;
  if (store.current.pendingEmbeddingModel === model) return;
  const active = store.current.report?.embedding_provenance;
  if (active?.provider_id === provider && active.model_id === model) return;
  store.setPendingEmbeddingModel(model);
  try {
    await c.sendRequest("deslop/embeddingSetModel", {
      provider_id: provider,
      model_id: model,
      endpoint,
    });
  } catch (err) {
    store.setPendingEmbeddingModel(null);
    throw err;
  }
}

export async function seedInitialReport(
  c: LanguageClient,
  store: ReportStore,
): Promise<void> {
  try {
    const snapshot = await c.sendRequest<Report>("deslop/reportGet");
    store.setSnapshot(snapshot, 0);
  } catch (err) {
    logError(err, "seed initial report");
  }
}

export function tryResolveOptional(
  extensionPath: string,
  kind: "mcp",
  manifest: DeploymentManifest,
  settings: BinarySettings = {},
): ResolvedBinary | undefined {
  try {
    return resolveBinary(extensionPath, kind, manifest, settings);
  } catch (err) {
    logError(err, `resolve ${kind} (optional)`);
    return undefined;
  }
}

function currentBinarySettings(): BinarySettings {
  const cfg = vscode.workspace.getConfiguration("deslop");
  return {
    lspPath: cfg.get<string>("lspPath", ""),
    mcpPath: cfg.get<string>("mcpPath", ""),
  };
}

function requireResolved(
  resolved: Record<string, ResolvedBinary>,
  componentId: string,
): ResolvedBinary {
  const binary = resolved[componentId];
  if (!binary)
    throw new Error(
      `Required Deslop component ${componentId} did not resolve.`,
    );
  return binary;
}

export function currentExtensionVersion(
  context: vscode.ExtensionContext,
): string {
  const raw = context.extension.packageJSON as { version?: unknown };
  return typeof raw.version === "string" ? raw.version : "0.0.0";
}

export function revealActiveBinary(
  lsp: ResolvedBinary | undefined,
  mcp: ResolvedBinary | undefined,
): void {
  const lines = [
    lsp
      ? `deslop-lsp → ${lsp.path}  [${lsp.source}, version=${lsp.version ?? "unknown"}]`
      : "deslop-lsp → not resolved",
    mcp
      ? `deslop-mcp → ${mcp.path}  [${mcp.source}, version=${mcp.version ?? "unknown"}]`
      : "deslop-mcp → not bundled",
  ];
  void vscode.window
    .showInformationMessage(lines.join("\n"), { modal: true })
    .then(undefined, (err) => logError(err, "show active binary dialog"));
}

export function surfaceStartupFailure(err: unknown, store?: ReportStore): void {
  logError(err, "language client startup");
  const isMissing = err instanceof BundledBinaryMissingError;
  const isUnsupported = err instanceof UnsupportedPlatformError;
  const isMismatch = err instanceof BinaryVerificationError;
  const isConfiguredMissing = err instanceof BinaryMissingError;
  const message =
    isMissing || isUnsupported || isMismatch || isConfiguredMissing
      ? err.message
      : "Deslop failed to start its analysis server. See the Deslop output channel.";
  store?.setLifecycle({ kind: "failed", message });
  vscode.window.showErrorMessage(message, "Reveal log").then(
    (choice) => {
      if (choice === "Reveal log") initOutputChannel().show();
    },
    (uiErr) => logError(uiErr, "showErrorMessage"),
  );
}
