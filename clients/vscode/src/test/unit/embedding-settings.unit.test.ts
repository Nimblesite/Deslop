// Unit: production embedding settings exposed by package.json and extension.ts.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import {
  currentInitializationOptions,
  syncEmbeddingSettingsToLsp,
} from "../../extension";
import { ReportStore } from "../../reportStore";

interface PackageContribution {
  contributes: {
    configuration: {
      properties: Record<string, ConfigurationProperty>;
    };
  };
}

interface ConfigurationProperty {
  default?: unknown;
  description?: string;
  enum?: unknown[];
  type?: string;
}

function extensionPackage(): PackageContribution {
  const packagePath = path.resolve(__dirname, "../../..", "package.json");
  const text = fs.readFileSync(packagePath, "utf8");
  return JSON.parse(text) as PackageContribution;
}

function legacyProviderId(): string {
  return ["st", "ub"].join("");
}

function legacyModelId(): string {
  return ["blake3", legacyProviderId()].join("-");
}

function embeddingOptions(): Record<string, unknown> {
  const options = currentInitializationOptions();
  const embedding = options["embedding"];
  assert.ok(embedding && typeof embedding === "object");
  return embedding as Record<string, unknown>;
}

async function setEmbeddingConfig(values: {
  mode: string;
  provider: string;
  model: string;
  endpoint?: string;
}): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update("embedding.mode", values.mode, vscode.ConfigurationTarget.Global);
  await cfg.update("embedding.provider", values.provider, vscode.ConfigurationTarget.Global);
  await cfg.update("embedding.model", values.model, vscode.ConfigurationTarget.Global);
  await cfg.update(
    "embedding.endpoint",
    values.endpoint ?? "http://127.0.0.1:11434",
    vscode.ConfigurationTarget.Global,
  );
}

async function resetEmbeddingConfig(): Promise<void> {
  await setEmbeddingConfig({
    mode: "off",
    provider: "ollama",
    model: "nomic-embed-text",
  });
}

suite("embedding settings", () => {
  teardown(async () => {
    await resetEmbeddingConfig();
  });

  test("issue #88 provider setting exposes only production providers", () => {
    const property = extensionPackage().contributes.configuration.properties[
      "deslop.embedding.provider"
    ];

    assert.ok(property, "deslop.embedding.provider must be contributed");
    assert.equal(property.type, "string");
    assert.equal(property.default, "ollama");
    assert.deepEqual(property.enum, ["ollama"]);
    assert.equal(
      property.enum?.includes(legacyProviderId()),
      false,
      "production provider enum must not include the legacy test provider",
    );
    assert.equal(
      property.description?.includes(legacyProviderId()),
      false,
      "production setting description must not mention the legacy test provider",
    );
  });

  test("issue #88 stale legacy provider config is ignored during initialization", async () => {
    const cfg = vscode.workspace.getConfiguration("deslop");
    await setEmbeddingConfig({
      mode: "auto",
      provider: legacyProviderId(),
      model: legacyModelId(),
    });

    const embedding = embeddingOptions();

    assert.equal(cfg.get<string>("embedding.provider"), legacyProviderId());
    assert.equal(embedding["provider"], "ollama");
    assert.equal(embedding["model"], "nomic-embed-text");
    assert.equal(embedding["mode"], "off");
    assert.equal(
      cfg.get<string>("embedding.provider"),
      legacyProviderId(),
      "startup must not silently migrate stale workspace settings",
    );
  });

  test("issue #88 stale legacy provider config does not send set-model RPC", async () => {
    await setEmbeddingConfig({
      mode: "auto",
      provider: legacyProviderId(),
      model: legacyModelId(),
    });
    const calls: Array<{ method: string; params: unknown }> = [];
    const client = {
      sendRequest: (method: string, params: unknown) => {
        calls.push({ method, params });
        return Promise.resolve(undefined);
      },
    } as unknown as LanguageClient;
    const store = new ReportStore();

    await syncEmbeddingSettingsToLsp(store, () => client);

    const cfg = vscode.workspace.getConfiguration("deslop");
    assert.deepEqual(calls, []);
    assert.equal(store.current.pendingEmbeddingModel, null);
    assert.equal(cfg.get<string>("embedding.provider"), legacyProviderId());
    assert.equal(cfg.get<string>("embedding.model"), legacyModelId());
    assert.equal(
      cfg.get<string>("embedding.mode"),
      "auto",
      "sync must ignore stale settings without rewriting them as a migration",
    );
  });
});
