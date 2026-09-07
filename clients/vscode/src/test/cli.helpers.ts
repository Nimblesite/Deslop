// The bundled CLI as a test oracle. Suites that pin an extension string
// against what the engine prints — a mass, a clone kind title — scan a
// fixture with the CLI staged beside the bundled LSP and read its JSON,
// text and HTML reports back. One harness, so every parity suite drives
// the same binary with the same flags.
//
// Non-`.test.ts` so the Mocha glob does not load this as a suite.

import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import { loadDeploymentManifest, resolveBinary } from "../binary";
import type { ClusterKind } from "../types/report";

/** The bundle is resolved through the manifest by its LSP; the CLI is
 * staged beside it by `_vsix-stage-bundled-binaries`, never resolved on its
 * own, because the manifest lists the CLI as a PATH / package-manager
 * install rather than a bundled component. */
const LSP_KIND = "lsp";
const CLI_BINARY_NAME = "deslop";
/** The fixture `.vscode-test.mjs` stages for every suite. */
const CLI_FIXTURE_ENV = "DESLOP_TEST_FIXTURE";
const CLI_OUTPUT_PREFIX = "report";
const TEMP_DIR_PREFIX = "deslop-cli-oracle-";
const UTF8 = "utf8";

/** The cluster facts a parity suite reads back from the JSON report. */
export interface ScannedCluster {
  id: string;
  mass: number;
  kind: ClusterKind;
}

/** One CLI scan of the staged fixture: its clusters beside the rendered
 * text and HTML reports. */
export interface ScannedFixture {
  clusters: ScannedCluster[];
  text: string;
  html: string;
}

/** The extension root, resolved from the compiled test tree. */
function extensionRoot(): string {
  return path.resolve(__dirname, "../..");
}

/** The CLI staged beside the bundled LSP, on any platform's suffix. */
export function stagedCliPath(): string {
  const root = extensionRoot();
  const lsp = resolveBinary(root, LSP_KIND, loadDeploymentManifest(root));
  const cli = path.join(path.dirname(lsp.path), `${CLI_BINARY_NAME}${path.extname(lsp.path)}`);
  assert.ok(fs.existsSync(cli), `the CLI must be staged beside the bundled LSP: ${cli}`);
  return cli;
}

/** The fixture `.vscode-test.mjs` staged for this run. */
export function stagedFixturePath(): string {
  const fixture = process.env[CLI_FIXTURE_ENV] ?? "";
  assert.ok(fixture, `${CLI_FIXTURE_ENV} must name the staged fixture`);
  return fixture;
}

/** Scans `root` with the bundled CLI — embeddings off, no cache, plus
 * `extraArgs` — and returns its JSON clusters beside its text and HTML
 * reports. */
export function scanWithBundledCli(root: string, extraArgs: readonly string[] = []): ScannedFixture {
  const outputDir = fs.mkdtempSync(path.join(os.tmpdir(), TEMP_DIR_PREFIX));
  const prefix = path.join(outputDir, CLI_OUTPUT_PREFIX);
  execFileSync(stagedCliPath(), [
    root,
    "--embeddings",
    "off",
    "--no-incremental",
    "--output",
    prefix,
    ...extraArgs,
  ]);
  const report = JSON.parse(fs.readFileSync(`${prefix}.json`, UTF8)) as { clusters: ScannedCluster[] };
  return {
    clusters: report.clusters,
    text: fs.readFileSync(`${prefix}.txt`, UTF8),
    html: fs.readFileSync(`${prefix}.html`, UTF8),
  };
}

/** Scans the staged fixture with the bundled CLI. */
export function scanFixtureWithBundledCli(): ScannedFixture {
  return scanWithBundledCli(stagedFixturePath());
}
