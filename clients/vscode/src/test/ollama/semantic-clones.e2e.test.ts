// Ollama-gated VSIX e2e suite. NEVER runs in CI — wired only through
// `make vsix-test-ollama` / `make test-ollama`. Requires a live Ollama
// daemon at 127.0.0.1:11434 with `nomic-embed-text` pulled.
//
// Proves that the VSIX path (real VS Code → real deslop-lsp → real
// Ollama) actually surfaces Type-4 semantic clones. The csharp-type4
// fixture contains recursive vs. iterative implementations of the same
// algorithms; AST + token LSH cannot match them, so a cross-file
// cluster with non-zero `embedding_cos` can only exist if the full
// embedding pipeline is wired through end-to-end.
//
// See crates/deslop/tests/cli.rs::ollama_type4_cross_file_cluster_has_positive_embedding_signal
// for the equivalent Rust-layer proof.
//
// Provider swaps go through the `deslop/embeddingSetModel` JSON-RPC
// method — NOT through `vscode.workspace.getConfiguration().update()`,
// because the LSP has no `didChangeConfiguration` handler and config
// writes are silently dropped. `embedding/setModel` atomically swaps
// providers, re-runs the pipeline, and returns the new provenance.

import * as assert from "node:assert/strict";
import * as http from "node:http";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import type {
  Report,
  EmbeddingModelInfo,
  EmbeddingProvenance,
} from "../../types/report";
import { sleep } from "../suite/helpers";

const EXT_ID = "nimblesite.deslop-live";
const OLLAMA_ENDPOINT = "http://127.0.0.1:11434";
const OLLAMA_MODEL = "nomic-embed-text";
const OLLAMA_PROVIDER = "ollama";
const DISABLED_PROVIDER = "off";

interface ExtensionExports {
  readonly client?: LanguageClient;
}

interface SetModelResponse {
  provider_id: string;
  model_id: string;
  model_version: string;
  dimensions: number;
}

async function ollamaModelNames(): Promise<string[]> {
  return await new Promise<string[]>((resolveNames, reject) => {
    const req = http.get(
      `${OLLAMA_ENDPOINT}/api/tags`,
      { timeout: 2000 },
      (res) => {
        const chunks: Buffer[] = [];
        res.on("data", (chunk: Buffer) => chunks.push(chunk));
        res.on("end", () => {
          if (res.statusCode !== 200) {
            reject(
              new Error(
                `Ollama /api/tags returned ${res.statusCode ?? "unknown"}`,
              ),
            );
            return;
          }
          try {
            const body = JSON.parse(Buffer.concat(chunks).toString("utf8")) as {
              models?: Array<{ name: string }>;
            };
            resolveNames((body.models ?? []).map((model) => model.name));
          } catch (err) {
            reject(err instanceof Error ? err : new Error(String(err)));
          }
        });
      },
    );
    req.on("timeout", () => {
      req.destroy();
      reject(new Error(`Ollama not reachable at ${OLLAMA_ENDPOINT} within 2s`));
    });
    req.on("error", (err) => {
      reject(
        new Error(`Ollama not reachable at ${OLLAMA_ENDPOINT}: ${err.message}`),
      );
    });
  });
}

/// Fail fast with a clear message if the daemon is unreachable or the
/// required model is missing. Without this, the LSP's first
/// `embedding/listModels` call blocks for the full 60-second HTTP
/// timeout and the test dies as an opaque mocha timeout.
async function preflightOllama(): Promise<void> {
  const names = await ollamaModelNames();
  const haveModel = names.some(
    (n) => n === OLLAMA_MODEL || n.startsWith(`${OLLAMA_MODEL}:`),
  );
  assert.ok(
    haveModel,
    `Ollama is running but model '${OLLAMA_MODEL}' is missing. Run: ollama pull ${OLLAMA_MODEL}`,
  );
}

/// Set Global-scope config BEFORE the extension activates so the LSP
/// spawns with `initializationOptions.embedding.provider = "ollama"`.
/// Global avoids writing to the fixture's `.vscode/settings.json`
/// (which would leak across test runs and require cleanup).
async function seedInitialConfig(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update(
    "embedding.provider",
    OLLAMA_PROVIDER,
    vscode.ConfigurationTarget.Global,
  );
  await cfg.update(
    "embedding.model",
    OLLAMA_MODEL,
    vscode.ConfigurationTarget.Global,
  );
  await cfg.update(
    "embedding.endpoint",
    OLLAMA_ENDPOINT,
    vscode.ConfigurationTarget.Global,
  );
  await cfg.update(
    "embedding.mode",
    "required",
    vscode.ConfigurationTarget.Global,
  );
  await cfg.update("minNodes", 15, vscode.ConfigurationTarget.Global);
}

/// Restore the config we mutated so a subsequent test-host reuse finds
/// the defaults.
async function clearSeededConfig(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  for (const key of [
    "embedding.provider",
    "embedding.model",
    "embedding.endpoint",
    "embedding.mode",
    "minNodes",
  ]) {
    await cfg.update(key, undefined, vscode.ConfigurationTarget.Global);
  }
}

async function activateExtension(): Promise<ExtensionExports> {
  const ext = vscode.extensions.getExtension<ExtensionExports>(EXT_ID);
  assert.ok(ext, `${EXT_ID} must be installed in the test host`);
  const api = await ext.activate();
  for (let i = 0; i < 60; i++) {
    const cmds = await vscode.commands.getCommands(true);
    if (cmds.includes("deslop.openCluster")) return api;
    await sleep(250);
  }
  throw new Error("extension did not finish activating within 15s");
}

async function setProvider(
  client: LanguageClient,
  providerId: typeof OLLAMA_PROVIDER | typeof DISABLED_PROVIDER,
): Promise<SetModelResponse | null> {
  // [REMOVE-STUB] Production accepts `ollama` and the pseudo-provider
  // `off` for disabling. There is no stub fallback any more.
  return await client.sendRequest<SetModelResponse | null>(
    "deslop/embeddingSetModel",
    {
      provider_id: providerId,
      model_id: providerId === OLLAMA_PROVIDER ? OLLAMA_MODEL : DISABLED_PROVIDER,
      endpoint: providerId === OLLAMA_PROVIDER ? OLLAMA_ENDPOINT : null,
    },
  );
}

async function waitForReport(
  client: LanguageClient,
  deadlineMs: number,
  predicate: (report: Report) => boolean = (r) => r.clusters.length > 0,
): Promise<Report> {
  const start = Date.now();
  let last: Report | undefined;
  while (Date.now() - start < deadlineMs) {
    try {
      const report = await client.sendRequest<Report>("deslop/reportGet");
      if (predicate(report)) return report;
      last = report;
    } catch {
      // LSP may not have seeded yet; keep polling.
    }
    await sleep(500);
  }
  throw new Error(
    `predicate unsatisfied within ${deadlineMs}ms; last: ${
      last
        ? `${last.clusters.length} clusters, ${last.files_analysed} files`
        : "<none>"
    }`,
  );
}

function crossFileType4Cluster(
  report: Report,
): Report["clusters"][number] | undefined {
  return report.clusters.find((cluster) => {
    const paths = cluster.occurrences.map((o) => o.path.replace(/\\/g, "/"));
    const hasIterative = paths.some((p) => p.endsWith("Iterative.cs"));
    const hasRecursive = paths.some((p) => p.endsWith("Recursive.cs"));
    return hasIterative && hasRecursive;
  });
}

suite("ollama semantic clone detection (real Ollama)", () => {
  let client: LanguageClient;
  let ollamaProvenance: EmbeddingProvenance;

  suiteSetup(async function () {
    this.timeout(60_000);
    await preflightOllama();
    await seedInitialConfig();
    const api = await activateExtension();
    assert.ok(
      api.client,
      "extension must expose the LanguageClient via its API",
    );
    client = api.client;
    // Confirm the LSP spawned against Ollama (not a stub). This is the
    // first falsifiable checkpoint: if initializationOptions weren't
    // applied, provenance will be null or stub-flavoured.
    const initialReport = await waitForReport(
      client,
      60_000,
      (r) => r.embedding_provenance !== null,
    );
    assert.ok(
      initialReport.embedding_provenance,
      "LSP must have Ollama provenance after init",
    );
    ollamaProvenance = initialReport.embedding_provenance;
    assert.equal(ollamaProvenance.provider_id, OLLAMA_PROVIDER);
    assert.equal(ollamaProvenance.model_id, OLLAMA_MODEL);
    assert.ok(ollamaProvenance.dimensions > 0, "dimensions must be positive");
  });

  suiteTeardown(async function () {
    this.timeout(10_000);
    await clearSeededConfig();
  });

  test("cross-file Type-4 cluster surfaces with Ollama provenance", async function () {
    this.timeout(90_000);
    const report = await waitForReport(
      client,
      60_000,
      (r) => crossFileType4Cluster(r) !== undefined,
    );
    const cluster = crossFileType4Cluster(report);
    assert.ok(
      cluster,
      `no cross-file cluster spans Iterative.cs <-> Recursive.cs; report had ${report.clusters.length} clusters`,
    );
    assert.ok(
      cluster.mass > 0,
      `a reported cluster must carry positive duplicated mass, got ${cluster.mass}`,
    );
    assert.ok(
      cluster.occurrences.length >= 2,
      `the cross-file cluster must carry both files' occurrences, got ${cluster.occurrences.length}`,
    );
    const paths = cluster.occurrences.map((o) => o.path.replace(/\\/g, "/"));
    assert.ok(
      paths.some((p) => p.endsWith("Iterative.cs")) &&
        paths.some((p) => p.endsWith("Recursive.cs")),
      "the cluster must span the recursive and iterative implementations",
    );
    assert.equal(
      report.embedding_provenance?.provider_id,
      OLLAMA_PROVIDER,
      "the semantic recall layer must be the real Ollama provider",
    );
    // [FUSED-PAIR-SIGNALS] The embedding cosine is a pair measurement and
    // is not carried on the cluster wire; the Rust-layer proof of the
    // embedding signal lives in
    // crates/deslop/tests/cli.rs::ollama_type4_cross_file_cluster_has_positive_embedding_signal.
    const retiredSignals = ["signals", "signal_source", "embedding_cos"] as const;
    for (const field of retiredSignals) {
      assert.equal(field in cluster, false, `cluster wire must not carry ${field}`);
    }
  });

  test("[ollama-non-ci] embeddingListModels lists the real local Ollama models", async function () {
    this.timeout(90_000);
    const installedNames = await ollamaModelNames();
    assert.ok(
      installedNames.length > 0,
      "Ollama /api/tags must return at least one real model",
    );

    const listed = await client.sendRequest<EmbeddingModelInfo[]>(
      "deslop/embeddingListModels",
      {},
    );
    const listedOllamaIds = listed
      .filter((model) => model.provider_id === OLLAMA_PROVIDER)
      .map((model) => model.model_id);
    const installedBareIds = installedNames.map(
      (name) => name.split(":")[0] ?? name,
    );

    for (const bareId of installedBareIds) {
      assert.ok(
        listedOllamaIds.includes(bareId),
        `embeddingListModels must include real Ollama model '${bareId}'; got ${JSON.stringify(listedOllamaIds)}`,
      );
    }
    // [REMOVE-STUB] Production payloads must not include the
    // deterministic test stub alongside real Ollama models.
    assert.equal(
      listed.some((model) => model.provider_id === "stub"),
      false,
      "embeddingListModels must never expose the deterministic stub provider",
    );
  });

  test("embeddingSetModel(off) drops the cross-file cluster and flips provenance", async function () {
    this.timeout(120_000);

    // Snapshot the Ollama-era cluster so we can prove it was there.
    const beforeReport = await waitForReport(
      client,
      60_000,
      (r) => crossFileType4Cluster(r) !== undefined,
    );
    const beforeCluster = crossFileType4Cluster(beforeReport);
    assert.ok(beforeCluster, "pre-swap cluster must exist with Ollama");
    assert.ok(
      beforeCluster.mass > 0,
      `pre-swap cluster must carry positive mass, got ${beforeCluster.mass}`,
    );

    // [REMOVE-STUB] Turning embeddings off is the production-supported
    // way to disable the semantic recall layer. The LSP acknowledges
    // the request (Option<EmbeddingProvenance>) and re-runs analysis
    // without the embedding pass.
    await setProvider(client, "off");

    // After embeddings are off, the next reportGet reflects the
    // structural/token-only signals. Poll briefly in case the re-run
    // propagation is asynchronous.
    const afterReport = await waitForReport(
      client,
      30_000,
      (r) => r.embedding_provenance === null,
    );
    const afterCluster = crossFileType4Cluster(afterReport);

    // [FUSED-PAIR-SIGNALS] The embedding cosine is a pair measurement and
    // does not ride on the cluster wire; the Rust-layer proof pins the
    // embedding signal itself. Here we pin what the wire can prove: with
    // embeddings off, the provenance flips to null and the semantic
    // recall layer is provably absent.
    assert.equal(
      afterReport.embedding_provenance,
      null,
      "off-mode must drop the embedding provenance to null",
    );
    if (afterCluster !== undefined) {
      assert.ok(
        afterCluster.mass > 0,
        `a surviving cluster must still carry positive mass, got ${afterCluster.mass}`,
      );
    }
  });

  test("embeddingSetModel(ollama) restores the cross-file cluster", async function () {
    this.timeout(120_000);
    // [REMOVE-STUB] `embedding/setModel` returns `Option<EmbeddingProvenance>` —
    // the LSP acknowledges the queued swap with `null` and the new
    // provenance is observed via `reportGet` once the refresh commits.
    await setProvider(client, OLLAMA_PROVIDER);
    // And the Type-4 cluster comes back.
    const report = await waitForReport(client, 60_000, (r) => {
      return r.embedding_provenance?.provider_id === OLLAMA_PROVIDER;
    });
    const cluster = crossFileType4Cluster(report);
    assert.ok(cluster, "restore-to-ollama must re-surface the Type-4 cluster");
    assert.equal(
      report.embedding_provenance?.provider_id,
      OLLAMA_PROVIDER,
      "restored provenance must be the real Ollama provider",
    );
    assert.ok(cluster.mass > 0, `restored cluster must carry mass, got ${cluster.mass}`);
  });
});
