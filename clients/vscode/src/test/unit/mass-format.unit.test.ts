// Unit: mass rendering ([RANK-MASS-SUM]). Mass is a whole-number count —
// canonical nodes × additional visible occurrences — so no surface may print
// it at the decimal precision reserved for measured pair signals, and the
// extension must print the same string the CLI text report prints. The
// webview cannot execute under vscode-test, so its render paths, like the
// extension host's, are pinned on the parsed source: every read of a mass
// inside rendered text must pass through the one whole-number formatter.

import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as ts from "typescript";

import { loadDeploymentManifest, resolveBinary } from "../../binary";
import { formatMass, formatScore } from "../../types/format";
import { formatSignal } from "../../types/signals";
import {
  descendants,
  extensionPath,
  parseTypeScriptSource,
  parseWebviewSource,
} from "./webview-source.helpers";

/** A mass whose two-decimal rendering (`527.00`) differs from its count. */
const WHOLE_NUMBER_MASS = 527;
const WHOLE_NUMBER_MASS_TEXT = "527";
const TWO_DECIMAL_MASS_TEXT = "527.00";
/** A measured pair signal, which genuinely carries a fraction. */
const FRACTIONAL_SIGNAL = 0.8;
const FRACTIONAL_SIGNAL_TEXT = "0.80";

const MASS_FORMATTER = "formatMass";
const MASS_PROPERTY = "mass";
const WORST_MASS_PARAMETER = "worstMass";

/** Every extension-host file that prints a mass. */
const HOST_MASS_RENDER_SOURCES = [
  "src/clusterDocument.ts",
  "src/tree/nodes.ts",
  "src/commands/treeMenus.ts",
];
/** The webview that prints a mass: the panel header, its body sentence,
 * the stat row, and the stat-row hover copy. */
const WEBVIEW_MASS_RENDER_SOURCE = "cluster/main.tsx";

/** The bundle is resolved through the manifest by its LSP; the CLI is
 * staged beside it by `_vsix-stage-bundled-binaries`, never resolved on its
 * own, because the manifest lists the CLI as a PATH / package-manager
 * install rather than a bundled component. */
const LSP_KIND = "lsp";
const CLI_BINARY_NAME = "deslop";
const CLI_FIXTURE_ENV = "DESLOP_TEST_FIXTURE";
const CLI_OUTPUT_PREFIX = "report";
const CLI_TEXT_MASS_KEY = "mass=";
const TEMP_DIR_PREFIX = "deslop-mass-";
const UTF8 = "utf8";

suite("mass rendering", () => {
  test("mass prints as a whole number, never at the two-decimal signal precision", () => {
    assert.equal(formatMass(WHOLE_NUMBER_MASS), WHOLE_NUMBER_MASS_TEXT);
    assert.equal(
      formatScore(WHOLE_NUMBER_MASS),
      TWO_DECIMAL_MASS_TEXT,
      "the signal formatter would dress a count up as a measurement",
    );
    assert.notEqual(formatMass(WHOLE_NUMBER_MASS), formatScore(WHOLE_NUMBER_MASS));
  });

  test("measured pair signals keep their two-decimal precision", () => {
    assert.equal(formatSignal(FRACTIONAL_SIGNAL), FRACTIONAL_SIGNAL_TEXT);
  });

  test("every rendered mass passes through the whole-number formatter", () => {
    for (const relativePath of HOST_MASS_RENDER_SOURCES) {
      assertRenderedMassUsesFormatMass(
        parseTypeScriptSource(extensionPath(relativePath), ts.ScriptKind.TS),
      );
    }
    assertRenderedMassUsesFormatMass(parseWebviewSource(WEBVIEW_MASS_RENDER_SOURCE));
  });

  test("the extension prints the same mass string as the CLI text report", () => {
    const { clusters, text } = scanFixtureWithBundledCli();
    assert.ok(clusters.length > 0, "the fixture must report at least one cluster");
    for (const cluster of clusters) {
      const row = `[${cluster.id}] ${CLI_TEXT_MASS_KEY}${formatMass(cluster.mass)} `;
      assert.ok(text.includes(row), `the CLI text report must print "${row}":\n${text}`);
    }
  });
});

/** A read of a cluster's mass, or of the worst mass a row is handed. */
function isMassRead(node: ts.Node): boolean {
  if (ts.isPropertyAccessExpression(node)) return node.name.text === MASS_PROPERTY;
  return (
    ts.isIdentifier(node) &&
    node.text === WORST_MASS_PARAMETER &&
    !ts.isParameter(node.parent)
  );
}

/** Whether `node` sits inside text a user reads: a template string or a
 * JSX expression. */
function isInsideRenderedText(node: ts.Node): boolean {
  for (let ancestor = node.parent; ancestor; ancestor = ancestor.parent) {
    if (ts.isTemplateExpression(ancestor) || ts.isJsxExpression(ancestor)) return true;
  }
  return false;
}

/** The name of the function a node is passed straight into, or "". */
function directCalleeName(node: ts.Node): string {
  const call = node.parent;
  if (!ts.isCallExpression(call) || !call.arguments.includes(node as ts.Expression)) return "";
  return ts.isIdentifier(call.expression) ? call.expression.text : "";
}

function assertRenderedMassUsesFormatMass(root: ts.SourceFile): void {
  const rendered = descendants(root, isMassRead).filter(isInsideRenderedText);
  assert.ok(rendered.length > 0, `${root.fileName} must render a mass`);
  for (const read of rendered) {
    assert.equal(
      directCalleeName(read),
      MASS_FORMATTER,
      `${root.fileName}: a rendered mass must pass through ${MASS_FORMATTER}: ${read.parent.getText()}`,
    );
  }
}

interface ScannedFixture {
  clusters: { id: string; mass: number }[];
  text: string;
}

/** The CLI staged beside the bundled LSP, on any platform's suffix. */
function stagedCliPath(): string {
  const root = extensionPath(".");
  const lsp = resolveBinary(root, LSP_KIND, loadDeploymentManifest(root));
  const cli = path.join(path.dirname(lsp.path), `${CLI_BINARY_NAME}${path.extname(lsp.path)}`);
  assert.ok(fs.existsSync(cli), `the CLI must be staged beside the bundled LSP: ${cli}`);
  return cli;
}

/** Scans the test fixture with the staged CLI and returns its JSON
 * clusters beside its text report. */
function scanFixtureWithBundledCli(): ScannedFixture {
  const fixture = process.env[CLI_FIXTURE_ENV] ?? "";
  assert.ok(fixture, `${CLI_FIXTURE_ENV} must name the staged fixture`);
  const outputDir = fs.mkdtempSync(path.join(os.tmpdir(), TEMP_DIR_PREFIX));
  const prefix = path.join(outputDir, CLI_OUTPUT_PREFIX);
  execFileSync(stagedCliPath(), [fixture, "--embeddings", "off", "--no-incremental", "--nohtml", "--output", prefix]);
  const report = JSON.parse(fs.readFileSync(`${prefix}.json`, UTF8)) as ScannedFixture;
  return { clusters: report.clusters, text: fs.readFileSync(`${prefix}.txt`, UTF8) };
}
