// CodeDedup VSIX entry point. Per CLAUDE.md: < 500 lines, thin glue,
// all UI logic split across bubble/, tree/, decorations/, commands/, webview/.

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

import { resolveBinary, BundledBinaryMissingError, UnsupportedPlatformError } from "./binary";
import { log, logError, initOutputChannel } from "./logging";
import { ReportStore } from "./reportStore";
import { registerCommands } from "./commands/register";
import { TopOffendersProvider, FocusedFileProvider, SessionProvider } from "./tree/providers";
import { DecorationManager } from "./decorations/manager";
import { LiveBubble } from "./bubble/live";
import { StatusBar } from "./commands/statusBar";
import {
  Report,
  ReportChangedNotification,
  AnalysisState,
  ReportDelta,
} from "./types/report";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  initOutputChannel();
  log("CodeDedup extension activating");

  const reportStore = new ReportStore();
  context.subscriptions.push(reportStore);

  try {
    client = startLanguageClient(context);
  } catch (err) {
    surfaceStartupFailure(err);
    return;
  }

  const topOffenders = new TopOffendersProvider(reportStore);
  const focusedFile = new FocusedFileProvider(reportStore);
  const session = new SessionProvider(reportStore, () => client);
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider("codededup.topOffenders", topOffenders),
    vscode.window.registerTreeDataProvider("codededup.focusedFile", focusedFile),
    vscode.window.registerTreeDataProvider("codededup.session", session),
  );

  const decorations = new DecorationManager(reportStore);
  context.subscriptions.push(decorations);

  const bubble = new LiveBubble(reportStore, () => client);
  context.subscriptions.push(bubble);

  const statusBar = new StatusBar(reportStore);
  context.subscriptions.push(statusBar);

  registerCommands(context, reportStore, () => client);

  await client.start();
  wireNotifications(client, reportStore);
  await seedInitialReport(client, reportStore);
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function startLanguageClient(context: vscode.ExtensionContext): LanguageClient {
  const binaryPath = resolveBinary(context.extensionPath, "lsp");
  const serverOptions: ServerOptions = {
    run: { command: binaryPath, transport: TransportKind.stdio, args: [] },
    debug: { command: binaryPath, transport: TransportKind.stdio, args: ["--debug"] },
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { language: "csharp", scheme: "file" },
      { language: "rust", scheme: "file" },
      { language: "python", scheme: "file" },
    ],
    synchronize: {
      configurationSection: "codededup",
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{cs,rs,py}"),
    },
    outputChannel: initOutputChannel(),
    initializationOptions: currentInitializationOptions(),
  };
  return new LanguageClient("codededup", "CodeDedup", serverOptions, clientOptions);
}

function currentInitializationOptions(): Record<string, unknown> {
  const cfg = vscode.workspace.getConfiguration("codededup");
  return {
    minNodes: cfg.get<number>("minNodes", 30),
    embedding: {
      provider: cfg.get<string>("embedding.provider", "ollama"),
      model: cfg.get<string>("embedding.model", "nomic-embed-text"),
      endpoint: cfg.get<string>("embedding.endpoint", "http://127.0.0.1:11434"),
      mode: cfg.get<string>("embedding.mode", "auto"),
    },
    incremental: cfg.get<boolean>("incremental", true),
    configPath: cfg.get<string>("configPath", ""),
  };
}

function wireNotifications(c: LanguageClient, store: ReportStore): void {
  c.onNotification(
    "codededup/reportChanged",
    async (payload: ReportChangedNotification) => {
      store.notifyChange(payload.summary);
      const delta = await c.sendRequest<ReportDelta | null>("codededup/reportDelta", {
        since_generation: store.current.generation,
      });
      if (delta) {
        store.applyDelta(delta);
        return;
      }
      const snapshot = await c.sendRequest<Report>("codededup/reportGet", {});
      store.setSnapshot(snapshot, payload.generation);
    },
  );
  c.onNotification("codededup/analysisState", (state: AnalysisState) => {
    log(`analysis state: ${state}`);
  });
}

async function seedInitialReport(c: LanguageClient, store: ReportStore): Promise<void> {
  try {
    const snapshot = await c.sendRequest<Report>("codededup/reportGet", {});
    store.setSnapshot(snapshot, 0);
  } catch (err) {
    logError(err, "seed initial report");
  }
}

function surfaceStartupFailure(err: unknown): void {
  logError(err, "language client startup");
  const isMissing = err instanceof BundledBinaryMissingError;
  const isUnsupported = err instanceof UnsupportedPlatformError;
  const message =
    isMissing || isUnsupported
      ? (err as Error).message
      : "CodeDedup failed to start its analysis server. See the CodeDedup output channel.";
  vscode.window
    .showErrorMessage(message, "Reveal log")
    .then(
      (choice) => {
        if (choice === "Reveal log") initOutputChannel().show();
      },
      (uiErr) => logError(uiErr, "showErrorMessage"),
    );
}
