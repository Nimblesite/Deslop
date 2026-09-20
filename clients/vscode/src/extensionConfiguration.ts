import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { ReportStore } from "./reportStore";

export const DESLOP_CONFIGURATION_NAMESPACE = "deslop";

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

export function buildServerArgs(
  workspaceRoot: string | undefined,
  debug: boolean,
): string[] {
  if (!workspaceRoot) return debug ? ["--debug"] : [];
  const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
  const args = [workspaceRoot, ...processArguments(cfg)];
  // [RANK-STRUCTURAL-ONLY] Only an explicit editor preference overrides TOML.
  const preference = cfg.inspect<string>("ranking.structuralOnly");
  const structuralOnly = preference?.workspaceFolderValue ?? preference?.workspaceValue ?? preference?.globalValue;
  if (structuralOnly && ["demote", "ignore", "keep"].includes(structuralOnly)) args.push("--ranking-structural-only", structuralOnly);
  if (debug) args.push("--debug");
  return args;
}

function processArguments(cfg: vscode.WorkspaceConfiguration): string[] {
  const args: string[] = [];
  const workerThreads = cfg.get<number>("lsp.workerThreads", 0);
  if (Number.isInteger(workerThreads) && workerThreads > 0) {
    args.push("--worker-threads", String(workerThreads));
  }
  const nice = cfg.get<number>("lsp.nice", 0);
  if (Number.isInteger(nice) && nice !== 0) {
    args.push("--nice", String(Math.max(-20, Math.min(19, nice))));
  }
  return args;
}

export function currentInitializationOptions(): Record<string, unknown> {
  const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
  const embedding = embeddingSettingsFromConfiguration(cfg);
  return {
    minNodes: cfg.get<number>("minNodes", 30),
    embedding,
    incremental: cfg.get<boolean>("incremental", true),
    configPath: cfg.get<string>("configPath", ""),
    diagnostics: {
      enabled: cfg.get<boolean>("diagnostics.enabled", false),
      severityByKind: cfg.get<Record<string, string>>("diagnostics.severityByKind", {}),
      scope: cfg.get<string>("diagnostics.scope", "open-files"),
    },
  };
}

function embeddingSettingsFromConfiguration(cfg: vscode.WorkspaceConfiguration): EmbeddingSettings {
  const supported = cfg.get<string>("embedding.provider", PRODUCTION_EMBEDDING_PROVIDER) === PRODUCTION_EMBEDDING_PROVIDER;
  return {
    provider: PRODUCTION_EMBEDDING_PROVIDER,
    model: supported ? cfg.get<string>("embedding.model", DEFAULT_EMBEDDING_MODEL) : DEFAULT_EMBEDDING_MODEL,
    endpoint: cfg.get<string>("embedding.endpoint", DEFAULT_EMBEDDING_ENDPOINT),
    mode: supported ? cfg.get<string>("embedding.mode", DEFAULT_EMBEDDING_MODE) : DEFAULT_EMBEDDING_MODE,
  };
}

export async function syncEmbeddingSettingsToLsp(
  store: ReportStore,
  clientOf: () => LanguageClient | undefined,
): Promise<void> {
  const c = clientOf();
  if (!c) return;
  const cfg = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION_NAMESPACE);
  const { provider, model, endpoint, mode } = embeddingSettingsFromConfiguration(cfg);
  if (mode === "off") return;
  if (store.current.pendingEmbeddingModel === model) return;
  const active = store.current.report?.embedding_provenance;
  if (active?.provider_id === provider && active.model_id === model) return;
  store.setPendingEmbeddingModel(model);
  try {
    await c.sendRequest("deslop/embeddingSetModel", { provider_id: provider, model_id: model, endpoint });
  } catch (err) {
    store.setPendingEmbeddingModel(null);
    throw err;
  }
}
