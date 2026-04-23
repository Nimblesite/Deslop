// Deslop VSIX entry point. Per CLAUDE.md: < 500 lines, thin glue,
// all UI logic split across bubble/, tree/, decorations/, commands/, webview/.

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

import {
  resolveBinary,
  BundledBinaryMissingError,
  UnsupportedPlatformError,
  ResolvedBinary,
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
import { DecorationManager } from "./decorations/manager";
import { LiveBubble } from "./bubble/live";
import { StatusBar } from "./commands/statusBar";
import { registerCompareProvider } from "./compare/provider";
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

/// Public API returned by `activate()`. Lets tests reach the live
/// LanguageClient without parallel activation or command-surface hacks.
export interface ExtensionApi {
  readonly client: LanguageClient | undefined;
}

export async function activate(context: vscode.ExtensionContext): Promise<ExtensionApi> {
  initOutputChannel();
  log("extension activating", {
    extensionPath: context.extensionPath,
    version: currentExtensionVersion(context),
    platform: process.platform,
    arch: process.arch,
  });

  const reportStore = new ReportStore();
  context.subscriptions.push(reportStore);

  const ticker = new StatusTicker();
  context.subscriptions.push(ticker);

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
    vscode.commands.registerCommand("deslop.revealLog", () => initOutputChannel().show(true)),
  );

  try {
    resolvedLsp = resolveBinary(context.extensionPath, "lsp", currentExtensionVersion(context));
    resolvedMcp = tryResolveOptional(context.extensionPath, "mcp", currentExtensionVersion(context));
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
    return { get client() { return client; } };
  }

  const decorations = new DecorationManager(reportStore);
  context.subscriptions.push(decorations);

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
    return { get client() { return client; } };
  }
  wireNotifications(client, reportStore);
  await seedInitialReport(client, reportStore);
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("deslop.embedding")) return;
      syncEmbeddingSettingsToLsp(reportStore, () => client).catch((err: unknown) =>
        logError(err, "sync embedding settings to LSP"),
      );
    }),
  );
  return { get client() { return client; } };
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
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
    debug: { command: lsp.path, transport: TransportKind.stdio, args: debugArgs },
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { language: "csharp", scheme: "file" },
      { language: "rust", scheme: "file" },
      { language: "python", scheme: "file" },
    ],
    synchronize: {
      configurationSection: "deslop",
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{cs,rs,py}"),
    },
    outputChannel: initOutputChannel(),
    initializationOptions: currentInitializationOptions(),
  };
  return new LanguageClient("deslop", "Deslop", serverOptions, clientOptions);
}

export function buildServerArgs(
  workspaceRoot: string | undefined,
  debug: boolean,
): string[] {
  // [LSP-EMBEDDING-CONSENT] Fresh settings pass `--embeddings off`;
  // the picker persists `auto` only after explicit model selection.
  if (!workspaceRoot) return debug ? ["--debug"] : [];
  const cfg = vscode.workspace.getConfiguration("deslop");
  const args = [workspaceRoot];
  if (debug) args.push("--debug");
  args.push("--min-nodes", String(cfg.get<number>("minNodes", 30)));
  args.push("--embeddings", cfg.get<string>("embedding.mode", "off"));
  args.push("--embedding-provider", cfg.get<string>("embedding.provider", "ollama"));
  args.push("--embedding-model", cfg.get<string>("embedding.model", "nomic-embed-text"));
  args.push(
    "--embedding-endpoint",
    cfg.get<string>("embedding.endpoint", "http://127.0.0.1:11434"),
  );
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
  return {
    minNodes: cfg.get<number>("minNodes", 30),
    embedding: {
      provider: cfg.get<string>("embedding.provider", "ollama"),
      model: cfg.get<string>("embedding.model", "nomic-embed-text"),
      endpoint: cfg.get<string>("embedding.endpoint", "http://127.0.0.1:11434"),
      mode: cfg.get<string>("embedding.mode", "off"),
    },
    incremental: cfg.get<boolean>("incremental", true),
    configPath: cfg.get<string>("configPath", ""),
  };
}

async function refreshAfterChange(
  c: LanguageClient,
  store: ReportStore,
  payload: ReportChangedNotification,
): Promise<void> {
  const delta = await c.sendRequest<ReportDelta | null>("deslop/reportDelta");
  if (delta) {
    store.applyDelta(delta);
    return;
  }
  const snapshot = await c.sendRequest<Report>("deslop/reportGet");
  store.setSnapshot(snapshot, payload.generation);
}

async function refreshAfterEmbedding(c: LanguageClient, store: ReportStore): Promise<void> {
  const snapshot = await c.sendRequest<Report>("deslop/reportGet");
  store.setSnapshot(snapshot, store.current.generation + 1);
}

export function wireNotifications(c: LanguageClient, store: ReportStore): void {
  c.onNotification("deslop/reportChanged", (payload: ReportChangedNotification) => {
    store.notifyChange(payload.summary);
    refreshAfterChange(c, store, payload).catch((err: unknown) =>
      logError(err, "refresh report after change"),
    );
  });
  c.onNotification("deslop/analysisState", (state: AnalysisState) => {
    log("analysis state", { state });
    if (state === "running") store.setLifecycle({ kind: "analysing" });
    else if (state === "idle") store.setLifecycle({ kind: "ready" });
    else if (state === "errored") {
      store.setLifecycle({
        kind: "failed",
        message: "Analysis failed — see the Deslop log for details.",
      });
    }
  });
  c.onNotification("deslop/embeddingProgress", (progress: EmbeddingProgress) => {
    if (progress.phase === "complete") {
      store.setEmbeddingProgress(null);
      refreshAfterEmbedding(c, store).catch((err: unknown) =>
        logError(err, "refresh report after embedding"),
      );
    } else {
      store.setEmbeddingProgress(progress);
    }
  });
}

export async function syncEmbeddingSettingsToLsp(
  store: ReportStore,
  clientOf: () => LanguageClient | undefined,
): Promise<void> {
  const c = clientOf();
  if (!c) return;
  const cfg = vscode.workspace.getConfiguration("deslop");
  const mode = cfg.get<string>("embedding.mode", "off");
  if (mode === "off") return;
  const provider = cfg.get<string>("embedding.provider", "ollama");
  const model = cfg.get<string>("embedding.model", "nomic-embed-text");
  const endpoint = cfg.get<string>("embedding.endpoint", "http://127.0.0.1:11434");
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

export async function seedInitialReport(c: LanguageClient, store: ReportStore): Promise<void> {
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
  version: string,
): ResolvedBinary | undefined {
  try {
    return resolveBinary(extensionPath, kind, version);
  } catch (err) {
    logError(err, `resolve ${kind} (optional)`);
    return undefined;
  }
}

export function currentExtensionVersion(context: vscode.ExtensionContext): string {
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
  vscode.window.showInformationMessage(lines.join("\n"), { modal: true });
}

export function surfaceStartupFailure(err: unknown, store?: ReportStore): void {
  logError(err, "language client startup");
  const isMissing = err instanceof BundledBinaryMissingError;
  const isUnsupported = err instanceof UnsupportedPlatformError;
  const message =
    isMissing || isUnsupported
      ? (err).message
      : "Deslop failed to start its analysis server. See the Deslop output channel.";
  store?.setLifecycle({ kind: "failed", message });
  vscode.window
    .showErrorMessage(message, "Reveal log")
    .then(
      (choice) => {
        if (choice === "Reveal log") initOutputChannel().show();
      },
      (uiErr) => logError(uiErr, "showErrorMessage"),
    );
}
