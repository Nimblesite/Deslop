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

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import type { Report } from "../../types/report";
import { sleep } from "../suite/helpers";

const EXT_ID = "deslop.deslop-vscode";
const COS_FLOOR = 0.3;

interface ExtensionExports {
  readonly client?: LanguageClient;
}

async function activateExtension(): Promise<ExtensionExports> {
  const ext = vscode.extensions.getExtension<ExtensionExports>(EXT_ID);
  assert.ok(ext, "deslop.deslop-vscode must be installed in the test host");
  const api = await ext.activate();
  for (let i = 0; i < 40; i++) {
    const cmds = await vscode.commands.getCommands(true);
    if (cmds.includes("deslop.openCluster")) return api;
    await sleep(250);
  }
  throw new Error("extension did not finish activating within 10s");
}

async function configureProvider(provider: "ollama" | "stub"): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  await cfg.update("embedding.provider", provider, vscode.ConfigurationTarget.Workspace);
  await cfg.update("embedding.model", "nomic-embed-text", vscode.ConfigurationTarget.Workspace);
  await cfg.update("embedding.mode", "required", vscode.ConfigurationTarget.Workspace);
  await cfg.update("minNodes", 8, vscode.ConfigurationTarget.Workspace);
}

async function waitForReport(client: LanguageClient, deadlineMs: number): Promise<Report> {
  const start = Date.now();
  let last: Report | undefined;
  while (Date.now() - start < deadlineMs) {
    try {
      const report = await client.sendRequest<Report>("deslop/reportGet");
      if (report.clusters.length > 0) return report;
      last = report;
    } catch {
      // LSP may not have seeded yet; keep polling.
    }
    await sleep(500);
  }
  throw new Error(
    `no clusters within ${deadlineMs}ms; last report: ${
      last ? `${last.clusters.length} clusters, ${last.files_analysed} files` : "<none>"
    }`,
  );
}

function crossFileType4Cluster(report: Report): Report["clusters"][number] | undefined {
  return report.clusters.find((cluster) => {
    const paths = new Set(cluster.occurrences.map((o) => o.path.replace(/\\/g, "/")));
    const hasIterative = [...paths].some((p) => p.endsWith("Iterative.cs"));
    const hasRecursive = [...paths].some((p) => p.endsWith("Recursive.cs"));
    return hasIterative && hasRecursive;
  });
}

suite("ollama semantic clone detection (real Ollama)", () => {
  let client: LanguageClient;

  suiteSetup(async function () {
    this.timeout(60_000);
    await configureProvider("ollama");
    const api = await activateExtension();
    assert.ok(api.client, "extension must expose the LanguageClient via its API");
    client = api.client;
  });

  test("embedding provenance reports ollama / nomic-embed-text", async function () {
    this.timeout(90_000);
    const report = await waitForReport(client, 60_000);
    assert.ok(
      report.embedding_provenance,
      "report.embedding_provenance must be populated when provider=ollama/mode=required",
    );
    assert.equal(report.embedding_provenance.provider_id, "ollama");
    assert.equal(report.embedding_provenance.model_id, "nomic-embed-text");
    assert.ok(
      report.embedding_provenance.dimensions > 0,
      "embedding dimensions must be positive",
    );
  });

  test("cross-file Type-4 cluster surfaces with embedding_cos > 0.3", async function () {
    this.timeout(90_000);
    const report = await waitForReport(client, 60_000);
    const cluster = crossFileType4Cluster(report);
    assert.ok(
      cluster,
      `no cross-file cluster spans Iterative.cs <-> Recursive.cs; report had ${report.clusters.length} clusters`,
    );
    assert.ok(
      cluster.signals.embedding_cos > COS_FLOOR,
      `embedding_cos must exceed ${COS_FLOOR} for a Type-4 semantic match, got ${cluster.signals.embedding_cos}`,
    );
    assert.ok(
      cluster.signals.embedding_cos > cluster.signals.structural,
      "embedding_cos must dominate structural for a Type-4 match",
    );
    assert.ok(
      cluster.signals.embedding_cos > cluster.signals.token_jaccard,
      "embedding_cos must dominate token_jaccard for a Type-4 match",
    );
  });

  test("switching provider=stub drops the Type-4 cross-file cluster", async function () {
    this.timeout(120_000);
    await configureProvider("stub");
    // Give the LSP a full re-analysis cycle: didChangeConfiguration +
    // pipeline re-run over the two-file fixture.
    await sleep(6000);
    const stubReport = await client.sendRequest<Report>("deslop/reportGet");
    const cluster = crossFileType4Cluster(stubReport);
    if (cluster) {
      assert.ok(
        cluster.signals.embedding_cos <= COS_FLOOR,
        `stub provider must not emit embedding_cos > ${COS_FLOOR}; got ${cluster.signals.embedding_cos}`,
      );
    }
    // Restore for any follow-up suites.
    await configureProvider("ollama");
  });
});
