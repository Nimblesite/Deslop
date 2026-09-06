// [VSIX-EMBED-PICKER] — first-class QuickPick with Kinetic Manuscript hints.
// Lists every production model returned by embedding/listModels plus actions.

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { log, logError } from "../logging";
import { ReportStore } from "../reportStore";
import { EmbeddingModelInfo, ReportDelta } from "../types/report";

const RECOMMENDED: Record<string, string> = {
  "nomic-embed-code": "Recommended for code clone detection.",
  "nomic-embed-text": "Solid default, Apache 2.0, 768-dim.",
  unixcoder: "Alternative; strong on cross-language.",
  codet5p: "Large model, strong on semantic matches.",
};
const OFF_PROVIDER_ID = "off";
const OFF_MODEL_ID = "off";

interface Entry extends vscode.QuickPickItem {
  entryKind: "model" | "off" | "pull" | "refresh" | "none";
  model?: EmbeddingModelInfo;
}


export async function pickEmbeddingModel(
  store: ReportStore,
  clientOf: () => LanguageClient | undefined,
): Promise<void> {
  const client = clientOf();
  if (!client) {
    vscode.window.showErrorMessage("Deslop: analysis server not running.");
    return;
  }

  const quickPick = vscode.window.createQuickPick<Entry>();
  quickPick.title = "Deslop — pick an embedding model";
  quickPick.placeholder = "Pick a local model; embedding work starts after selection";
  quickPick.matchOnDescription = true;
  quickPick.matchOnDetail = true;
  quickPick.ignoreFocusOut = false;
  quickPick.busy = true;
  let disposed = false;
  quickPick.onDidHide(() => {
    disposed = true;
    quickPick.dispose();
  });
  quickPick.onDidAccept(async () => {
    const picked = quickPick.selectedItems[0] ?? quickPick.activeItems[0];
    if (!picked || picked.entryKind === "none") return;
    quickPick.hide();
    try {
      if (picked.entryKind === "pull") {
        await vscode.env.openExternal(vscode.Uri.parse("https://ollama.com/library"));
      } else if (picked.entryKind === "refresh") {
        await pickEmbeddingModel(store, clientOf);
      } else if (picked.entryKind === "off") {
        await turnEmbeddingsOff(client, store);
      } else if (picked.entryKind === "model" && picked.model) {
        await setModelFromPicker(client, store, picked.model);
      }
    } finally {
      quickPick.dispose();
    }
  });
  quickPick.show();

  let models: EmbeddingModelInfo[] = [];
  try {
    models = await client.sendRequest<EmbeddingModelInfo[]>("deslop/embeddingListModels", {});
  } catch (err) {
    logError(err, "embedding/listModels");
  }
  if (disposed) return;
  quickPick.busy = false;
  quickPick.items = buildItems(models, store);
}

// [REMOVE-STUB] Production picker rows are derived strictly from
// `embedding/listModels`. The deterministic stub provider is test
// infrastructure, never shown to users.
export function buildItems(
  models: EmbeddingModelInfo[],
  store: ReportStore,
): Entry[] {
  const active = store.current.report?.embedding_provenance;
  const items: Entry[] = [];

  items.push({
    entryKind: "off",
    label: active
      ? "$(circle-slash) Turn embeddings off"
      : "$(circle-slash) Embeddings off",
    description: active ? "Stop live embedding analysis" : "Currently disabled",
    detail: "Keeps identical, nearly identical, and loosely similar detection.",
  });

  items.push({
    entryKind: "none",
    label: "Local embeddings run after selection",
    description: "May be slow; progress stays in Session.",
    detail: "Deslop does not start the live embedding pass until you choose a model.",
  });

  const ollama = models.filter((m) => m.provider_id === "ollama");

  if (ollama.length === 0) {
    items.push({
      entryKind: "none",
      label: "Ollama not detected",
      description: "Install from ollama.com to use local embedding models.",
      detail: "Embedding analysis can stay off until a local model is available.",
    });
  } else {
    items.push({
      entryKind: "none",
      label: "Ollama models",
      kind: vscode.QuickPickItemKind.Separator,
    });
    for (const m of ollama) {
      const recommended = RECOMMENDED[m.model_id];
      const activeMark = isActive(active, m) ? "  ✓ active" : "";
      const descParts = [
        `${m.dimensions ?? "?"}-dim`,
        m.recommended ? "recommended" : null,
        m.reachable ? null : "offline",
      ].filter(Boolean);
      items.push({
        entryKind: "model",
        label: `$(database) ${m.model_id}${activeMark}`,
        description: descParts.join(" · "),
        detail: recommended ?? "User-pulled Ollama model.",
        model: m,
      });
    }
  }

  items.push(
    { entryKind: "pull", label: "$(cloud-download) Pull a new model…", description: "ollama.com/library" },
    { entryKind: "refresh", label: "$(refresh) Refresh list", description: "Re-query Ollama" },
  );
  return items;
}

export async function setModel(client: LanguageClient, model: EmbeddingModelInfo): Promise<void> {
  await switchModel(client, model);
}

// Marks the store's pending embedding model so the Session panel reflects
// the user's choice immediately, then dispatches the LSP swap. On failure
// the pending marker is cleared so the UI reverts to the active model.
export async function setModelFromPicker(
  client: LanguageClient,
  store: ReportStore,
  model: EmbeddingModelInfo,
): Promise<void> {
  await switchModel(client, model, store);
}

async function switchModel(
  client: LanguageClient,
  model: EmbeddingModelInfo,
  store?: ReportStore,
): Promise<void> {
  try {
    store?.setPendingEmbeddingModel(model.model_id);
    await requestModelSwitch(client, model);
    await persistModelConfig(model);
    vscode.window.showInformationMessage(`Embedding model switched to ${model.model_id}.`);
  } catch (err) {
    store?.setPendingEmbeddingModel(null);
    logError(err, "embedding/setModel");
    const message = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to switch embedding model: ${message}`);
    log("keeping previous model active");
  }
}

export async function turnEmbeddingsOff(client: LanguageClient, store: ReportStore): Promise<void> {
  try {
    store.setPendingEmbeddingModel(OFF_MODEL_ID);
    await client.sendRequest("deslop/embeddingSetModel", {
      provider_id: OFF_PROVIDER_ID,
      model_id: OFF_MODEL_ID,
    });
    await persistOffConfig();
    const delta = await client.sendRequest<ReportDelta | null>("deslop/reportDelta", {
      since_generation: store.current.generation,
    });
    if (delta) store.applyDelta(delta);
    else store.setPendingEmbeddingModel(null);
    vscode.window.showInformationMessage("Embedding analysis turned off.");
  } catch (err) {
    store.setPendingEmbeddingModel(null);
    logError(err, "embedding/off");
    const message = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to turn embeddings off: ${message}`);
  }
}

function requestModelSwitch(
  client: LanguageClient,
  model: EmbeddingModelInfo,
): Promise<unknown> {
  return client.sendRequest("deslop/embeddingSetModel", {
    provider_id: model.provider_id,
    model_id: model.model_id,
  });
}

async function persistModelConfig(model: EmbeddingModelInfo): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update("embedding.provider", model.provider_id, vscode.ConfigurationTarget.Workspace);
  await cfg.update("embedding.model", model.model_id, vscode.ConfigurationTarget.Workspace);
  await cfg.update("embedding.mode", "auto", vscode.ConfigurationTarget.Workspace);
}

async function persistOffConfig(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update("embedding.mode", "off", vscode.ConfigurationTarget.Workspace);
}

export function isActive(
  active: { provider_id: string; model_id: string } | null | undefined,
  model: EmbeddingModelInfo | undefined,
): boolean {
  if (!active || !model) return false;
  return active.provider_id === model.provider_id && active.model_id === model.model_id;
}
