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
import { wireMcpRegistration } from "./mcpRegistration";
import { ReportStore } from "./reportStore";
import { registerCommands } from "./commands/register";
import {
  isTopOffendersFilterActive,
  readTopOffendersFilter,
} from "./commands/topOffendersView";
import { normalizeGroupBy } from "./tree/grouping";
import {
  TopOffendersProvider,
  MetricsProvider,
  SessionProvider,
  StatusTicker,
} from "./tree/providers";
import { ANALYSED_LANGUAGE_IDS } from "./types/languages";
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
let mcpDefinition: vscode.McpStdioServerDefinition | undefined;
let activeReportStore: ReportStore | undefined;

const REPORT_READY_CONTEXT = "deslop.reportReady";

// Derived from the single language registry so a newly supported language
// reaches the hover card and LSP document sync without a per-site edit
// ([FACET-MODEL] anti-drift). See ANALYSED_LANGUAGE_IDS in types/languages.
const ANALYSED_DOCUMENTS = ANALYSED_LANGUAGE_IDS.map((language) => ({
  language,
  scheme: "file",
}));

const PRODUCTION_EMBEDDING_PROVIDER = "ollama";
const DEFAULT_EMBEDDING_MODEL = "nomic-embed-text";
const DEFAULT_EMBEDDING_ENDPOINT = "http://127.0.0.1:11434";
const DEFAULT_EMBEDDING_MODE = "off";

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
  readonly mcpDefinition: vscode.McpStdioServerDefinition | undefined;
  readonly reportStore: ReportStore | undefined;
}

// [VSIX-ACTIVATION] Entry point: resolve binaries, start the LSP client,
// wire reports/commands/decorations before live analysis begins.
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

  // [VSIX-TOP-OFFENDERS-GROUPING] / [VSIX-TOP-OFFENDERS-SORT] /
  // [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] Seed the context keys
  // synchronously BEFORE the tree view is created so the title-bar
  // toggles have `when`-clause values to match against on cold start.
  syncTopOffendersContext();
  syncReportReadyContext(reportStore);
  // [FACET-TOP-OFFENDERS-FILTER] Seed the store's facet-filter mirror so
  // the status bar and webviews slice correctly from the first render.
  reportStore.setFacetFilter(readTopOffendersFilter());

  const topOffenders = new TopOffendersProvider(reportStore, ticker);
  const metrics = new MetricsProvider(reportStore, ticker);
  const session = new SessionProvider(reportStore, ticker, () => client);
  // [VSIX-TOP-OFFENDERS-TOOLBAR] Expand All / Collapse All are provider-driven
  // (setBulkExpansion) so they reliably reach every level and sit adjacent in the
  // title bar — no built-in showCollapseAll, which would render a second,
  // detached collapse button.
  const topOffendersView = vscode.window.createTreeView("deslop.topOffenders", {
    treeDataProvider: topOffenders,
  });
  context.subscriptions.push(
    topOffenders,
    metrics,
    session,
    topOffendersView,
    vscode.window.createTreeView("deslop.metrics", {
      treeDataProvider: metrics,
      showCollapseAll: true,
    }),
    vscode.window.createTreeView("deslop.session", {
      treeDataProvider: session,
      showCollapseAll: true,
    }),
    vscode.commands.registerCommand("deslop.revealLog", () =>
      initOutputChannel().show(true),
    ),
    vscode.commands.registerCommand("deslop.topOffenders.expandAll", () => {
      topOffenders.setBulkExpansion("expand");
    }),
    vscode.commands.registerCommand("deslop.topOffenders.collapseAll", () => {
      topOffenders.setBulkExpansion("collapse");
    }),
    vscode.commands.registerCommand("deslop.refresh", () => {
      topOffenders.refresh();
      metrics.refresh();
      session.refresh();
      void vscode.commands.executeCommand("deslop.refreshReport");
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        !event.affectsConfiguration("deslop.topOffenders.groupBy") &&
        !event.affectsConfiguration("deslop.topOffenders.sortBy") &&
        !event.affectsConfiguration("deslop.topOffenders.splitByLanguage") &&
        !event.affectsConfiguration("deslop.topOffenders.filterBuckets") &&
        !event.affectsConfiguration("deslop.topOffenders.filterCategories")
      ) {
        return;
      }
      syncTopOffendersContext();
      // [FACET-TOP-OFFENDERS-FILTER] Mirror the filter into the store so
      // the status-bar count and webviews slice in lock-step with the tree.
      reportStore.setFacetFilter(readTopOffendersFilter());
      topOffenders.refresh();
    }),
    reportStore.onDidChange(() => syncReportReadyContext(reportStore)),
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
    // [VSIX-MCP-INTEGRATION] Resolve the bundled deslop-mcp so VS Code's
    // MCP-aware agent hosts drive the same live daemon as the UI.
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
    client = startLanguageClient(resolvedLsp, resolveWorkspaceRoot());
  } catch (err) {
    surfaceStartupFailure(err, reportStore);
    return currentApi();
  }

  // [VSIX-MCP-INTEGRATION] Register the bundled MCP with VS Code's MCP API
  // so agent hosts launch it by absolute path, never a $PATH lookup (#267).
  mcpDefinition = wireMcpRegistration(context, resolvedMcp, resolveWorkspaceRoot());

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

  if (client) {
    reportStore.setLifecycle({ kind: "analysing" });
    try {
      await client.start();
    } catch (err) {
      surfaceStartupFailure(err, reportStore);
      return currentApi();
    }
    wireNotifications(client, reportStore);
    await seedInitialReport(client, reportStore);
  } else {
    // No workspace folder → nothing to analyse. The LSP was not started
    // (#201), so settle into the idle state rather than a perpetual scan
    // spinner. `scanStatus` renders "ready" as a clean empty view.
    reportStore.setLifecycle({ kind: "ready" });
  }
  context.subscriptions.push(
    // [VSIX-SETTINGS] Hot-reload deslop.* settings to the running LSP
    // without a restart (embedding settings sync here).
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

export function currentApi(): ExtensionApi {
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
    get mcpDefinition() {
      return mcpDefinition;
    },
    get reportStore() {
      return activeReportStore;
    },
  };
}

// [VSIX-TOP-OFFENDERS-GROUPING] / [VSIX-TOP-OFFENDERS-SORT] /
// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] / [FACET-TOP-OFFENDERS-FILTER]
// Mirror the persisted view axes onto context keys so the title-bar
// toggles' mutually exclusive `when` clauses can render the right
// buttons, and the filter button its active-filter icon state. Unknown
// / missing values fall back to the spec defaults — never throws.
export function syncTopOffendersContext(): void {
  const cfg = vscode.workspace.getConfiguration("deslop");
  const groupBy = normalizeGroupBy(cfg.get<string>("topOffenders.groupBy", "cluster"));
  const sortBy = cfg.get<string>("topOffenders.sortBy", "impact") === "path" ? "path" : "impact";
  const splitByLanguage = cfg.get<boolean>("topOffenders.splitByLanguage", false) === true;
  void vscode.commands.executeCommand("setContext", "deslop.topOffendersGroupBy", groupBy);
  void vscode.commands.executeCommand("setContext", "deslop.topOffendersSortBy", sortBy);
  void vscode.commands.executeCommand(
    "setContext",
    "deslop.topOffendersSplitByLanguage",
    splitByLanguage,
  );
  void vscode.commands.executeCommand(
    "setContext",
    "deslop.topOffendersFiltered",
    isTopOffendersFilterActive(),
  );
}

export function syncReportReadyContext(store: ReportStore): void {
  void vscode.commands.executeCommand(
    "setContext",
    REPORT_READY_CONTEXT,
    store.current.report !== null,
  );
}

export function startLanguageClient(
  lsp: ResolvedBinary,
  workspaceRoot: string | undefined,
): LanguageClient | undefined {
  if (!workspaceRoot) {
    // No workspace folder ⇒ nothing to analyse. Do not launch the server:
    // vscode-languageclient would append its `--stdio` transport flag as the
    // only argv, the Rust binary would read `--stdio` as the workspace root,
    // and the file watcher would crash-loop on that bogus path (#201). This
    // mirrors the MCP guard in wireMcpRegistration.
    log("no workspace folder open; LSP not started (nothing to analyse)", {});
    return undefined;
  }
  const runArgs = buildServerArgs(workspaceRoot, false);
  const debugArgs = buildServerArgs(workspaceRoot, true);
  log("starting language client", {
    lspPath: lsp.path,
    workspaceRoot: workspaceRoot ?? null,
    args: runArgs,
  });
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
    // [LSP-NON-INTERFERENCE] No middleware. Deslop's LSP advertises no
    // hover/definition/etc. providers, so there is nothing to intercept
    // or suppress — the editor's own language server owns those. The
    // clone card is rendered by the additive ClusterHoverProvider
    // (registered separately), which stacks alongside the language
    // server's hover instead of replacing it.
  };
  return new LanguageClient("deslop", "Deslop", serverOptions, clientOptions);
}

export function buildServerArgs(
  workspaceRoot: string | undefined,
  debug: boolean,
): string[] {
  if (!workspaceRoot) return debug ? ["--debug"] : [];
  const args = [workspaceRoot];
  const cfg = vscode.workspace.getConfiguration("deslop");
  const workerThreads = cfg.get<number>("lsp.workerThreads", 0);
  if (Number.isInteger(workerThreads) && workerThreads > 0) {
    args.push("--worker-threads", String(workerThreads));
  }
  const nice = cfg.get<number>("lsp.nice", 0);
  if (Number.isInteger(nice) && nice !== 0) {
    args.push("--nice", String(Math.max(-20, Math.min(19, nice))));
  }
  // [VSIX-SETTINGS-RANKING] / [RANK-STRUCTURAL-ONLY]: "default" defers
  // to .deslop.toml; anything else overrides it for this session.
  const structuralOnly = cfg.get<string>("ranking.structuralOnly", "default");
  if (["demote", "ignore", "keep"].includes(structuralOnly)) {
    args.push("--ranking-structural-only", structuralOnly);
  }
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

export async function refreshAfterChange(
  c: LanguageClient,
  store: ReportStore,
  payload: ReportChangedNotification,
): Promise<void> {
  // Pull the delta spanning the store's own baseline (#230). Without a
  // `since_generation` the server defaults to `current - 1`, which only spans
  // one generation — so a client that missed a `reportChanged` (lagged
  // broadcast or async gap) would merge a delta that never retracts the
  // clusters dropped in the skipped generations, leaving them as phantoms.
  const delta = await c.sendRequest<ReportDelta | null>("deslop/reportDelta", {
    since_generation: store.current.generation,
  });
  // applyDelta rejects (returns false) when no report is seeded yet or the
  // delta's baseline does not match the store's generation. Either way fall
  // back to the full snapshot so the store converges to the live engine.
  if (delta && store.applyDelta(delta)) return;
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
  // [VSIX-STATE-DIRTY]: text edits add to the dirty set so the visible
  // projection elides their occurrences; saves remove the file from the set
  // so the LSP's next reportChanged converges both views again.
  const onChange = vscode.workspace.onDidChangeTextDocument((event) => {
    store.markFileDirty(event.document.uri.fsPath);
  });
  const onSave = vscode.workspace.onDidSaveTextDocument((doc) => {
    store.clearFileDirty(doc.uri.fsPath);
  });
  return {
    dispose: () => {
      onChange.dispose();
      onSave.dispose();
    },
  };
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

export function currentBinarySettings(): BinarySettings {
  const cfg = vscode.workspace.getConfiguration("deslop");
  return {
    lspPath: cfg.get<string>("lspPath", ""),
    mcpPath: cfg.get<string>("mcpPath", ""),
  };
}

export function requireResolved(
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
  // [VSIX-NOTIFICATIONS] Daemon startup failure → error toast with a
  // "Reveal log" button (sparing notifications only).
  vscode.window.showErrorMessage(message, "Reveal log").then(
    (choice) => {
      if (choice === "Reveal log") initOutputChannel().show();
    },
    (uiErr) => logError(uiErr, "showErrorMessage"),
  );
}
