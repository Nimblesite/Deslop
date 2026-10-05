// E2E: [LSP-CODE-LENS] / [VSIX-CODE-LENS] — the clone badge sits at the
// first line of every occurrence in an open file, on by default, while
// diagnostics stay opt-in. Driven against the real bundled LSP: the lenses
// come back through VS Code's own provider query, exactly as the editor
// paints them, and every expected line is the engine's own occurrence line.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";
import type { Report, ReportCluster } from "../../types/report";
import {
  activateExtension,
  codeLenses,
  deleteRange,
  fixtureUri,
  openFixture,
  sleep,
  waitFor,
  waitForClusterHolding,
} from "./helpers";

const DESLOP_CONFIGURATION = "deslop";
const DIAGNOSTICS_ENABLED_SETTING = "diagnostics.enabled";
const DESLOP_DIAGNOSTIC_SOURCE = "deslop";
const JUMP_COMMAND = "deslop.jumpToNextOccurrence";
const JUMP_ACTION_SUFFIX = " — jump to next";
const FIRST_COLUMN = 0;
const LENS_TIMEOUT_MS = 30_000;
const LENS_POLL_MS = 250;
const REANALYSIS_TIMEOUT_MS = 15_000;
const FIRST_SOURCE_LINE = 1;
const LEADING_COMMENT = "// a line above every clone\n";
const ONE_LINE = 1;

// Shipping.cs: a unique BaseRate method, then two near-identical quote
// methods. Neither clone starts at line 1, so a lens pinned to the top of
// the file cannot pass for a lens on the occurrence.
const SHIPPING_FILE = "Shipping.cs";
const SHIPPING_KIND = "nearly_identical";
/** One-based first lines of QuoteStandard and QuoteExpress. */
const SHIPPING_START_LINES = [8, 20];
const SHIPPING_LENS_TITLE_PREFIX = "Nearly identical code × 2 — mass ";

// Alpha.cs / Beta.cs: a two-file pair whose occurrences are the whole class.
const ALPHA_FILE = "Alpha.cs";
const ALPHA_START_LINE = 1;
const ALPHA_LENS_TITLE_PREFIX = "Nearly identical code × 2 — mass ";

/** One lens the editor must paint: where, for which cluster, which occurrence. */
interface ExpectedLens {
  readonly line: number;
  readonly clusterId: string;
  readonly occurrenceIndex: number;
}

/** Every lens the report demands, keyed by file name, in file order. */
function expectedLensesByFile(report: Report): Map<string, ExpectedLens[]> {
  const byFile = new Map<string, ExpectedLens[]>();
  for (const cluster of report.clusters) {
    cluster.occurrences.forEach((occurrence, occurrenceIndex) => {
      const file = path.basename(occurrence.path);
      const lenses = byFile.get(file) ?? [];
      lenses.push({ line: occurrence.start_line - 1, clusterId: cluster.id, occurrenceIndex });
      byFile.set(file, lenses);
    });
  }
  for (const lenses of byFile.values()) lenses.sort((left, right) => left.line - right.line);
  return byFile;
}

function paintedLens(lens: vscode.CodeLens): ExpectedLens {
  const args: unknown[] = lens.command?.arguments ?? [];
  return { line: lens.range.start.line, clusterId: String(args[0]), occurrenceIndex: Number(args[1]) };
}

function startLinesIn(cluster: ReportCluster): number[] {
  return cluster.occurrences.map((occurrence) => occurrence.start_line);
}

/** Waits until the cluster holding `fileName` reports exactly `startLines`. */
async function waitForStartLines(
  api: ExtensionApi,
  fileName: string,
  startLines: number[],
): Promise<ReportCluster> {
  return await waitFor(() => {
    const cluster = api.reportStore?.current.report?.clusters.find((candidate) =>
      candidate.occurrences.some((occurrence) => path.basename(occurrence.path) === fileName),
    );
    if (!cluster) return undefined;
    return startLinesIn(cluster).join() === startLines.join() ? cluster : undefined;
  }, REANALYSIS_TIMEOUT_MS);
}

function deslopDiagnostics(uri: vscode.Uri): vscode.Diagnostic[] {
  return vscode.languages.getDiagnostics(uri).filter((diagnostic) => diagnostic.source === DESLOP_DIAGNOSTIC_SOURCE);
}

// Polls VS Code's lens query until the file carries `count` lenses — the
// observable "the LSP answered for this file" — then returns them.
async function lensesFor(uri: vscode.Uri, count: number): Promise<vscode.CodeLens[]> {
  const deadline = Date.now() + LENS_TIMEOUT_MS;
  let latest = await codeLenses(uri);
  while (latest.length !== count && Date.now() < deadline) {
    await sleep(LENS_POLL_MS);
    latest = await codeLenses(uri);
  }
  return latest;
}

function assertLens(
  lens: vscode.CodeLens,
  cluster: ReportCluster,
  occurrenceIndex: number,
  startLine: number,
  titlePrefix: string,
): void {
  const zeroBasedLine = startLine - 1;
  assert.equal(
    lens.range.start.line,
    zeroBasedLine,
    `[LSP-CODE-LENS] lens ${occurrenceIndex} sits at the first line of its occurrence`,
  );
  assert.equal(lens.range.end.line, zeroBasedLine, "the lens anchor is a single line");
  assert.equal(lens.range.start.character, FIRST_COLUMN, "the lens anchors at column zero");
  assert.equal(lens.command?.command, JUMP_COMMAND, "the lens navigates via Deslop's own command");
  assert.deepEqual(lens.command?.arguments, [cluster.id, occurrenceIndex], "the lens names its cluster and occurrence");
  const title = lens.command?.title ?? "";
  assert.ok(title.startsWith(titlePrefix), `the lens states the clone kind and count: ${title}`);
  assert.ok(title.endsWith(JUMP_ACTION_SUFFIX), `the lens ends with the jump action: ${title}`);
}

suite("code lens (real LSP)", () => {
  test("diagnostics are opt-in: the setting defaults to off", async () => {
    await activateExtension();
    const setting = vscode.workspace.getConfiguration(DESLOP_CONFIGURATION).inspect<boolean>(DIAGNOSTICS_ENABLED_SETTING);
    assert.equal(setting?.defaultValue, false, "diagnostics publication is off unless the user turns it on");
    assert.equal(
      vscode.workspace.getConfiguration(DESLOP_CONFIGURATION).get<boolean>(DIAGNOSTICS_ENABLED_SETTING),
      false,
      "the fixture workspace runs with the default",
    );
  });

  test("every occurrence in a file gets a lens at its own first line, with no diagnostics", async () => {
    const api = await activateExtension();
    assert.ok(api.client, "the live client must be exposed on ExtensionApi");
    const cluster = await waitForClusterHolding(api.client, SHIPPING_FILE);
    assert.equal(cluster.kind, SHIPPING_KIND, "the two quote methods are a nearly identical clone");
    assert.deepEqual(
      cluster.occurrences.map((occurrence) => occurrence.start_line),
      SHIPPING_START_LINES,
      "the engine reports both quote methods, in file order",
    );

    const uri = fixtureUri(SHIPPING_FILE);
    await openFixture(SHIPPING_FILE);
    const lenses = await lensesFor(uri, SHIPPING_START_LINES.length);
    assert.equal(lenses.length, SHIPPING_START_LINES.length, "one lens per occurrence in the file");
    SHIPPING_START_LINES.forEach((startLine, index) => {
      const lens = lenses[index];
      assert.ok(lens, `lens ${index} is present`);
      assertLens(lens, cluster, index, startLine, SHIPPING_LENS_TITLE_PREFIX);
    });
    assert.deepEqual(deslopDiagnostics(uri), [], "no Deslop diagnostic is published while diagnostics are off");
  });

  test("every occurrence in the whole report is lensed at its own line, and nothing else is", async () => {
    const api = await activateExtension();
    const report = await waitFor(() => api.reportStore?.current.report ?? undefined, REANALYSIS_TIMEOUT_MS);
    const expected = expectedLensesByFile(report);
    assert.ok(expected.size >= 3, `the fixture workspace spans several files: ${[...expected.keys()].join()}`);
    // A fixture whose every clone starts at line 1 cannot tell a lens on the
    // occurrence from a lens pinned to the top of the file. Refuse to pass on one.
    const belowFirstLine = [...expected.values()].flat().filter((lens) => lens.line > FIRST_SOURCE_LINE - 1);
    assert.ok(belowFirstLine.length >= 2, "the fixture must hold clones that start below line 1");

    for (const [file, lenses] of expected) {
      const uri = fixtureUri(file);
      await openFixture(file);
      const painted = (await lensesFor(uri, lenses.length)).map(paintedLens).sort((left, right) => left.line - right.line);
      assert.deepEqual(painted, lenses, `${file}: one lens per occurrence, each at its occurrence's first line`);
      for (const lens of await codeLenses(uri)) {
        assert.equal(lens.command?.command, JUMP_COMMAND, `${file}: every lens carries the jump command`);
        assert.ok(lens.command?.title.endsWith(JUMP_ACTION_SUFFIX), `${file}: every lens names the jump action`);
      }
      assert.deepEqual(deslopDiagnostics(uri), [], `${file}: no Deslop diagnostic while diagnostics are off`);
    }
  });

  test("the lens follows its occurrence when an edit moves the clone", async () => {
    const api = await activateExtension();
    const uri = fixtureUri(SHIPPING_FILE);
    const editor = await openFixture(SHIPPING_FILE);
    const shifted = SHIPPING_START_LINES.map((line) => line + ONE_LINE);
    try {
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), LEADING_COMMENT));
      await editor.document.save();
      const cluster = await waitForStartLines(api, SHIPPING_FILE, shifted);
      const lenses = await lensesFor(uri, shifted.length);
      shifted.forEach((startLine, index) => {
        const lens = lenses[index];
        assert.ok(lens, `lens ${index} is present after the edit`);
        assertLens(lens, cluster, index, startLine, SHIPPING_LENS_TITLE_PREFIX);
      });
    } finally {
      await deleteRange(editor, 0, 0, ONE_LINE, 0);
      await editor.document.save();
    }
    const restored = await waitForStartLines(api, SHIPPING_FILE, SHIPPING_START_LINES);
    const lenses = await lensesFor(uri, SHIPPING_START_LINES.length);
    SHIPPING_START_LINES.forEach((startLine, index) => {
      const lens = lenses[index];
      assert.ok(lens, `lens ${index} is present after the restore`);
      assertLens(lens, restored, index, startLine, SHIPPING_LENS_TITLE_PREFIX);
    });
  });

  test("a cross-file pair lenses only the occurrence that lives in the open file", async () => {
    const api = await activateExtension();
    assert.ok(api.client, "the live client must be exposed on ExtensionApi");
    const cluster = await waitForClusterHolding(api.client, ALPHA_FILE);
    const alphaIndex = cluster.occurrences.findIndex((occurrence) => occurrence.path.endsWith(ALPHA_FILE));
    assert.ok(alphaIndex >= 0, "Alpha.cs is one occurrence of the pair");
    assert.equal(cluster.occurrences[alphaIndex]?.start_line, ALPHA_START_LINE, "the Alpha occurrence is the whole class");

    const uri = fixtureUri(ALPHA_FILE);
    await openFixture(ALPHA_FILE);
    const lenses = await lensesFor(uri, 1);
    const [lens] = lenses;
    assert.ok(lens, "exactly one lens: the Beta occurrence is not in this file");
    assertLens(lens, cluster, alphaIndex, ALPHA_START_LINE, ALPHA_LENS_TITLE_PREFIX);
    assert.deepEqual(deslopDiagnostics(uri), [], "no Deslop diagnostic is published while diagnostics are off");
  });
});
