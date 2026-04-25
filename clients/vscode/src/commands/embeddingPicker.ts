// [VSIX-EMBED-PICKER] — first-class QuickPick with Kinetic Manuscript hints.
// Lists every model returned by embedding/listModels, plus stub + "Pull new" action.

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { log, logError } from "../logging";
import { ReportStore } from "../reportStore";
import { EmbeddingModelInfo } from "../types/report";

const RECOMMENDED: Record<string, string> = {
  "nomic-embed-code": "Recommended for code clone detection.",
  "nomic-embed-text": "Solid default, Apache 2.0, 768-dim.",
  unixcoder: "Alternative; strong on cross-language.",
  codet5p: "Large model, strong on semantic matches.",
};

interface Entry extends vscode.QuickPickItem {
  entryKind: "model" | "pull" | "refresh" | "none";
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

export function buildItems(models: EmbeddingModelInfo[], store: ReportStore): Entry[] {
  const active = store.current.report?.embedding_provenance;
  const items: Entry[] = [];

  items.push({
    entryKind: "none",
    label: "Local embeddings run after selection",
    description: "May be slow; progress stays in Session.",
    detail: "Deslop does not start the live embedding pass until you choose a model.",
  });

  const ollama = models.filter((m) => m.provider_id === "ollama");
  const stub = models.find((m) => m.provider_id === "stub");

  if (ollama.length === 0) {
    items.push({
      entryKind: "none",
      label: "Ollama not detected",
      description: "Install from ollama.com to use local embedding models.",
      detail: "Only the deterministic stub provider is available below.",
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
        m.is_embedding_model ? "embedding" : "may not embed",
        m.size_bytes ? formatSize(m.size_bytes) : null,
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

  items.push({
    entryKind: "model",
    label: `$(circuit-board) stub${isActive(active, stub) ? "  ✓ active" : ""}`,
    description: "deterministic · 64-dim · CI-friendly",
    detail: "Turns off semantic recall. Keeps identical, nearly identical, and loosely similar detection.",
    model: stub ?? {
      provider_id: "stub",
      model_id: "stub",
      model_version: "0",
      dimensions: 64,
      size_bytes: null,
      is_embedding_model: true,
    },
  });

  items.push(
    { entryKind: "pull", label: "$(cloud-download) Pull a new model…", description: "ollama.com/library" },
    { entryKind: "refresh", label: "$(refresh) Refresh list", description: "Re-query Ollama" },
  );
  return items;
}

export async function setModel(client: LanguageClient, model: EmbeddingModelInfo): Promise<void> {
  try {
    if (model.provider_id === "stub") {
      const confirm = await vscode.window.showWarningMessage(
        "The stub provider is deterministic but not semantically meaningful. \"Same behavior, different code\" (AI) recall is disabled.",
        { modal: true },
        "Use stub anyway",
      );
      if (confirm !== "Use stub anyway") return;
    }
    await client.sendRequest("deslop/embeddingSetModel", {
      provider_id: model.provider_id,
      model_id: model.model_id,
    });
    await vscode.workspace
      .getConfiguration("deslop")
      .update("embedding.provider", model.provider_id, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration("deslop")
      .update("embedding.model", model.model_id, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration("deslop")
      .update("embedding.mode", "auto", vscode.ConfigurationTarget.Workspace);
    vscode.window.showInformationMessage(`Embedding model switched to ${model.model_id}.`);
  } catch (err) {
    logError(err, "embedding/setModel");
    const message = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to switch embedding model: ${message}`);
    log("keeping previous model active");
  }
}

// Marks the store's pending embedding model so the Session panel reflects
// the user's choice immediately, then dispatches the LSP swap. On failure
// the pending marker is cleared so the UI reverts to the active model.
export async function setModelFromPicker(
  client: LanguageClient,
  store: ReportStore,
  model: EmbeddingModelInfo,
): Promise<void> {
  if (model.provider_id === "stub") {
    const confirm = await vscode.window.showWarningMessage(
      "The stub provider is deterministic but not semantically meaningful. \"Same behavior, different code\" (AI) recall is disabled.",
      { modal: true },
      "Use stub anyway",
    );
    if (confirm !== "Use stub anyway") return;
  }
  store.setPendingEmbeddingModel(model.model_id);
  try {
    await client.sendRequest("deslop/embeddingSetModel", {
      provider_id: model.provider_id,
      model_id: model.model_id,
    });
    await vscode.workspace
      .getConfiguration("deslop")
      .update("embedding.provider", model.provider_id, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration("deslop")
      .update("embedding.model", model.model_id, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration("deslop")
      .update("embedding.mode", "auto", vscode.ConfigurationTarget.Workspace);
    vscode.window.showInformationMessage(`Embedding model switched to ${model.model_id}.`);
  } catch (err) {
    store.setPendingEmbeddingModel(null);
    logError(err, "embedding/setModel");
    const message = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to switch embedding model: ${message}`);
    log("keeping previous model active");
  }
}


export function isActive(
  active: { provider_id: string; model_id: string } | null | undefined,
  model: EmbeddingModelInfo | undefined,
): boolean {
  if (!active || !model) return false;
  return active.provider_id === model.provider_id && active.model_id === model.model_id;
}

export function formatSize(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit] ?? "B"}`;
}
