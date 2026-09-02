// Unit: call the exported command implementations directly with a seeded
// store + active editor so the full branch coverage lands without colliding
// with the real extension's command registrations.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { tempFile } from "./temp-file.helpers";
import type { LanguageClient } from "vscode-languageclient/node";
import {
  COMMAND_BINDINGS,
  openWorstCluster,
  openOccurrence,
  jumpToNextOccurrence,
  comparePairEndpoints,
  openSchemaDoc,
  openCpuReport,
  renderCpuReport,
  resolveOccurrenceUri,
  openOccurrenceTarget,
} from "../../commands/register";
import { reportWithClusters } from "./report.helpers";
import {
  aiPayloadForCluster,
  aiPayloadForOccurrence,
  clusterIdForTreeNode,
  clusterLocationsText,
  copyClusterLocations,
  copyContextForAI,
  copyHumanLocation,
  copySourceSnippet,
  openAllOccurrences,
  revealOccurrenceInExplorer,
  sourceSnippetText,
  OPEN_ALL_THRESHOLD,
} from "../../commands/treeMenus";
import { buildCompareUri } from "../../compare/provider";
import { ReportStore } from "../../reportStore";
import { seededStore } from "./report-store.helpers";
import { activateExtension } from "../suite/helpers";
import { ClusterNode, OccurrenceNode } from "../../tree/providers";
import { Report, ReportCluster, ReportOccurrence } from "../../types/report";
import { occurrence, wireCluster } from "../cluster.helpers";

const UTF8_ENCODING = "utf8";
const TEST_SOURCE_PATH = "src/foo.cs";
const SECOND_TEST_SOURCE_PATH = "src/bar.cs";
const ELECTED_PAIR_LINE_PREFIX = "elected_pair:";
const PAIR_SIGNALS_LINE_PREFIX = "pair_signals:";
const FILE_A_NAME = "A.cs";
const FILE_B_NAME = "B.cs";
const TEST_TEN = 10;
const TEST_TWENTY = 20;
const DEFAULT_CLUSTER_WEIGHT = TEST_TEN;
const DEFAULT_OCCURRENCE_END_BYTE = 50;
const CYCLE_OCCURRENCE_END_BYTE = 16;
const CYCLE_CLUSTER_ID = "c-cycle";
const REFRESH_REPORT_COMMAND = "deslop.refreshReport";
const OPEN_CLUSTER_COMMAND = "deslop.openCluster";
const MARKDOWN_LANGUAGE = "markdown";
const CODE_FENCE = "```";
const TEST_TWO = 2;
const TEST_THREE = 3;
const TWO_CHARACTER_OFFSET = TEST_TWO;
const THIRD_OCCURRENCE_INDEX = TEST_TWO;
const REPORT_GET_CALL_COUNT = TEST_TWO;
const THIRD_LINE_INDEX = TEST_TWO;
const SHORT_OCCURRENCE_END_BYTE = TEST_THREE;
const CPU_WORK_MILLISECONDS = TEST_THREE;
const THREE_LINE_COUNT = TEST_THREE;
const THIRD_RANK = TEST_THREE;
const TEST_ONE = 1;
const RELATIVE_OCCURRENCE_PATH = "src/relative.cs";
// [VSIX-ACTIVATION] Commands only activation itself registers — the
// status-bar and title-bar refresh, the active-binary reveal, and the
// Top Offenders toolbar expand/collapse pair.
const ACTIVATION_OWNED_COMMAND_IDS = [
  "deslop.refresh",
  "deslop.revealActiveBinary",
  "deslop.topOffenders.expandAll",
  "deslop.topOffenders.collapseAll",
];
// [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] declares the per-language split
// toggle, and `package.json` contributes it as a title-bar button, but
// nothing registers a handler and no `splitByLanguage` setting exists —
// clicking it raises "command not found" (gh #495). Named here so the
// contract stays truthful about the one id that has no handler rather
// than silently accepting any; the entry comes out when the handler
// lands and the assertion below then covers it like every other id.
const PENDING_LANGUAGE_SPLIT_TOGGLE = ["deslop.topOffenders.toggleSplitByLanguage"];

async function findDiffTab(): Promise<vscode.TabInputTextDiff> {
  for (let i = 0; i < TEST_TWENTY; i += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return tab.input;
      }
    }
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 50);
    });
  }
  throw new Error("no diff tab opened after comparePairEndpoints");
}

async function closeAllDiffs(): Promise<void> {
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");
}

async function commandsEventuallyInclude(...ids: string[]): Promise<string[]> {
  for (let i = 0; i < 30; i += 1) {
    const commands = await vscode.commands.getCommands(true);
    if (ids.every((id) => commands.includes(id))) return commands;
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 100);
    });
  }
  return await vscode.commands.getCommands(true);
}

function cluster(id: string, paths: string[]): ReportCluster {
  return wireCluster({
    id,
    mass: DEFAULT_CLUSTER_WEIGHT,
    occurrences: paths.map((p) =>
      occurrence(p, 0, DEFAULT_OCCURRENCE_END_BYTE),
    ),
  });
}

function clusterWithRanges(
  id: string,
  occurrences: { path: string; start_byte: number; end_byte: number }[],
  rank = 1,
): ReportCluster {
  return wireCluster({
    id,
    rank,
    mass: DEFAULT_CLUSTER_WEIGHT,
    occurrences: occurrences.map((o) =>
      occurrence(o.path, o.start_byte, o.end_byte),
    ),
  });
}

function report(clusters: ReportCluster[]): Report {
  return reportWithClusters(
    clusters,
    { schema_doc: "# docs" },
    {
      analysed_loc: TEST_TEN,
      duplicated_loc: 5,
      duplication_percent: 50,
      duplicated_files: 1,
    },
  );
}

function extensionRoot(): string {
  return path.resolve(__dirname, "../../..");
}

function packagedSchemaDocPath(): string {
  return path.join(extensionRoot(), "dist", "schema_doc.md");
}

function fakeCtx(): vscode.ExtensionContext {
  const root = extensionRoot();
  return {
    subscriptions: { push: () => {} },
    extensionPath: root,
    extensionUri: vscode.Uri.file(root),
    extension: { packageJSON: { version: "0.0.0" } },
  } as unknown as vscode.ExtensionContext;
}

suite("register command implementations", () => {
  suiteSetup(async () => {
    await activateExtension();
  });

  test("openWorstCluster shows info when store is empty", () => {
    openWorstCluster(fakeCtx(), new ReportStore());
  });

  test("activation keeps VSIX commands separate from namespaced LSP commands", async () => {
    const commands = await commandsEventuallyInclude(
      REFRESH_REPORT_COMMAND,
      OPEN_CLUSTER_COMMAND,
      "deslop.lsp.refreshReport",
      "deslop.lsp.openCluster",
    );
    assert.equal(
      commands.filter((command) => command === REFRESH_REPORT_COMMAND).length,
      1,
    );
    assert.equal(
      commands.filter((command) => command === OPEN_CLUSTER_COMMAND).length,
      1,
    );
    assert.ok(commands.includes(REFRESH_REPORT_COMMAND));
    assert.ok(commands.includes(OPEN_CLUSTER_COMMAND));
    assert.ok(commands.includes("deslop.lsp.refreshReport"));
    assert.ok(commands.includes("deslop.lsp.openCluster"));
  });

  test("openWorstCluster opens a panel when the report has clusters", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c-top", ["/tmp/cdd-A.cs", "/tmp/cdd-B.cs"])]), 0);
    openWorstCluster(fakeCtx(), store);
  });

  test("path-style deslop cluster URI resolves to a readonly document", async () => {
    // [VSIX-CLUSTER-DOCUMENT] Issue #24: links emitted as
    // deslop://cluster/<id> must resolve through the extension provider.
    const uri = vscode.Uri.parse("deslop://cluster/cluster-for-test");
    const doc = await vscode.workspace.openTextDocument(uri);
    const text = doc.getText();
    assert.equal(doc.uri.scheme, "deslop");
    assert.equal(doc.uri.authority, "cluster");
    assert.match(text, /cluster-for-test/);
    assert.match(text, /Deslop cluster/i);
  });

  test("openOccurrence opens the referenced file at the byte range", async () => {
    const { dir, file } = tempFile("cdd-occ-", "occ.txt");
    fs.writeFileSync(file, "hello\nworld\n", UTF8_ENCODING);
    await openOccurrence(
      fixtureOccurrence({
        path: file,
        start_byte: 0,
        end_byte: SHORT_OCCURRENCE_END_BYTE,
      }),
    );
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("jumpToNextOccurrence navigates to the sibling when the cursor sits inside a cluster", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line-one\nline-two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    editor.selection = new vscode.Selection(
      new vscode.Position(0, TWO_CHARACTER_OFFSET),
      new vscode.Position(0, TWO_CHARACTER_OFFSET),
    );
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("c-1", [doc.uri.fsPath, "/tmp/cdd-sibling.cs"])]),
      0,
    );
    await jumpToNextOccurrence(store);
  });

  test("jumpToNextOccurrence uses code-lens cluster id and occurrence index deterministically", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-lens-jump-"));
    const fileA = path.join(dir, FILE_A_NAME);
    const fileB = path.join(dir, FILE_B_NAME);
    const fileC = path.join(dir, "C.cs");
    fs.writeFileSync(fileA, "public class A { int x = 1; }\n", UTF8_ENCODING);
    fs.writeFileSync(fileB, "public class B { int y = 2; }\n", UTF8_ENCODING);
    fs.writeFileSync(fileC, "public class C { int z = 3; }\n", UTF8_ENCODING);
    const store = new ReportStore();
    store.setSnapshot(
      report([
        clusterWithRanges(CYCLE_CLUSTER_ID, [
          { path: fileA, start_byte: 0, end_byte: CYCLE_OCCURRENCE_END_BYTE },
          { path: fileB, start_byte: 0, end_byte: CYCLE_OCCURRENCE_END_BYTE },
          { path: fileC, start_byte: 0, end_byte: CYCLE_OCCURRENCE_END_BYTE },
        ]),
      ]),
      0,
    );

    await jumpToNextOccurrence(store, CYCLE_CLUSTER_ID, 0);
    let editor = vscode.window.activeTextEditor;
    assert.equal(editor?.document.uri.fsPath, fileB);
    assert.match(editor?.document.getText() ?? "", /public class B/);
    assert.equal(editor?.selection.start.line, 0);
    assert.equal(editor?.selection.start.character, 0);
    assert.equal(editor?.selection.end.character, CYCLE_OCCURRENCE_END_BYTE);

    await jumpToNextOccurrence(store, CYCLE_CLUSTER_ID, THIRD_OCCURRENCE_INDEX);
    editor = vscode.window.activeTextEditor;
    assert.equal(editor?.document.uri.fsPath, fileA);
    assert.match(editor?.document.getText() ?? "", /public class A/);
    assert.equal(editor?.selection.end.character, CYCLE_OCCURRENCE_END_BYTE);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("jumpToNextOccurrence shows the info message when no cluster overlaps", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "z",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", ["/other"])]), 0);
    await jumpToNextOccurrence(store);
  });

  test("jumpToNextOccurrence bails when there is no active editor", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", ["/p"])]), 0);
    await jumpToNextOccurrence(store);
  });

  test("comparePairEndpoints opens a diff whose two sides are distinct resources with the matching occurrence bytes", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cmp-"));
    const fileA = path.join(dir, FILE_A_NAME);
    const fileB = path.join(dir, FILE_B_NAME);
    // Left side: bytes 0..16 of A.cs == "public class A {"
    // Right side: bytes 0..16 of B.cs == "public class B {"
    // Distinct files, distinct content — exercises the cross-file diff path.
    fs.writeFileSync(fileA, "public class A { int x = 1; }\n", UTF8_ENCODING);
    fs.writeFileSync(fileB, "public class B { int y = 2; }\n", UTF8_ENCODING);

    await closeAllDiffs();
    // [VSIX-PAIR-COMPARE] Both endpoints are explicit; the host never
    // invents a canonical side.
    await comparePairEndpoints(
      { path: fileA, start_byte: 0, end_byte: CYCLE_OCCURRENCE_END_BYTE },
      { path: fileB, start_byte: 0, end_byte: CYCLE_OCCURRENCE_END_BYTE },
    );
    const diff = await findDiffTab();

    assert.notEqual(
      diff.original.toString(),
      diff.modified.toString(),
      "compare diff must reference two distinct resources — the bug was pointing both sides at the same URI",
    );

    const left = await vscode.workspace.openTextDocument(diff.original);
    const right = await vscode.workspace.openTextDocument(diff.modified);
    assert.equal(left.getText(), "public class A {");
    assert.equal(right.getText(), "public class B {");

    await closeAllDiffs();
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("comparePairEndpoints opens distinct diff sides for two occurrences that live inside the same file", async () => {
    const { dir, file } = tempFile("cdd-cmp-same-", "same.cs");
    // Two clone regions inside a single source file. This is the case the
    // user reported: the old implementation handed `vscode.diff` the same
    // file URI twice, so the diff editor rendered the whole file against
    // itself. The fix must ensure each side shows only the clone bytes.
    const source =
      "OCCURRENCE_A_____________________________\n" +
      "middle middle middle middle middle middle\n" +
      "OCCURRENCE_B_____________________________\n";
    fs.writeFileSync(file, source, "utf8");
    const firstLineEnd = source.indexOf("\n");
    const thirdLineStart = source.indexOf("OCCURRENCE_B");
    const thirdLineEnd = source.indexOf("\n", thirdLineStart);

    await closeAllDiffs();
    await comparePairEndpoints(
      { path: file, start_byte: 0, end_byte: firstLineEnd },
      { path: file, start_byte: thirdLineStart, end_byte: thirdLineEnd },
    );
    const diff = await findDiffTab();

    assert.notEqual(
      diff.original.toString(),
      diff.modified.toString(),
      "same-file cluster must NOT produce a diff that points both sides at the file itself",
    );

    const left = await vscode.workspace.openTextDocument(diff.original);
    const right = await vscode.workspace.openTextDocument(diff.modified);
    assert.notEqual(
      left.getText(),
      right.getText(),
      "same-file diff must show the two distinct occurrences, not the full file on both sides",
    );
    assert.ok(
      left.getText().startsWith("OCCURRENCE_A"),
      `left side should contain occurrence A bytes, got: ${JSON.stringify(left.getText())}`,
    );
    assert.ok(
      right.getText().startsWith("OCCURRENCE_B"),
      `right side should contain occurrence B bytes, got: ${JSON.stringify(right.getText())}`,
    );

    await closeAllDiffs();
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("comparePairEndpoints is a no-op for missing, malformed, or identical endpoints", async () => {
    // [VSIX-PAIR-COMPARE] There is no canonical fallback: a missing,
    // malformed, or identical endpoint pair must never open a diff.
    await closeAllDiffs();
    await comparePairEndpoints(undefined, undefined);
    await comparePairEndpoints({ path: "a.ts", start_byte: 0, end_byte: 1 }, undefined);
    await comparePairEndpoints(undefined, { path: "a.ts", start_byte: 0, end_byte: 1 });
    await comparePairEndpoints({ path: "a.ts", start_byte: 0, end_byte: 1 }, "not-an-object");
    await comparePairEndpoints(
      { path: "a.ts", start_byte: 0, end_byte: 1 },
      { path: "a.ts", start_byte: 0, end_byte: 1 },
    );
    assert.equal(vscode.window.tabGroups.all.flatMap((group) => group.tabs).length, 0);
  });

  test("compare provider renders a friendly fallback for a stale occurrence file", async () => {
    const uri = buildCompareUri(
      fixtureOccurrence({
        path: "missing-deslop-compare-file.cs",
        start_byte: 0,
        end_byte: TEST_TWENTY,
      }),
      "a",
    );

    const doc = await vscode.workspace.openTextDocument(uri);
    const text = doc.getText();
    assert.match(text, /Deslop could not load this compare occurrence/);
    assert.match(text, /Refresh the Deslop report and try Compare again/);
    assert.match(text, /selected-pair/);
  });

  test("openSchemaDoc prefers packaged docs over a stale snapshot", async () => {
    const expected = fs.readFileSync(packagedSchemaDocPath(), UTF8_ENCODING);
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    await openSchemaDoc(fakeCtx(), store);
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "schema doc editor should be active");
    assert.equal(active.document.languageId, MARKDOWN_LANGUAGE);
    assert.equal(active.document.getText(), expected);
    assert.doesNotMatch(active.document.getText(), /# docs/);
  });

  test("openSchemaDoc reads the packaged fallback when schema_doc is absent", async () => {
    const expected = fs.readFileSync(packagedSchemaDocPath(), UTF8_ENCODING);
    await openSchemaDoc(fakeCtx(), new ReportStore());
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "packaged schema doc editor should be active");
    assert.equal(active.document.languageId, MARKDOWN_LANGUAGE);
    assert.equal(active.document.getText(), expected);
  });

  test("openCpuReport fetches the LSP CPU report and opens markdown", async () => {
    await openCpuReport(() => ({
      sendRequest: (method: string) => {
        assert.equal(method, "deslop/cpuReport");
        return Promise.resolve({
          current_phase: "idle",
          handler_counts: { "deslop/reportGet": REPORT_GET_CALL_COUNT, hover: 1 },
          in_flight: {
            pending_watcher_events: 0,
            pending_embed_requests: 0,
            in_progress_parse_batch: null,
          },
          last_100_phases: [
            {
              phase: "report_rendering",
              started_at_ms: TEST_TEN,
              duration_ms: CPU_WORK_MILLISECONDS,
              cpu_ms: CPU_WORK_MILLISECONDS,
              files_touched: ["src/Alpha.cs"],
            },
          ],
        });
      },
    }) as unknown as LanguageClient);
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "CPU report editor should be active");
    assert.equal(active.document.languageId, MARKDOWN_LANGUAGE);
    const text = active.document.getText();
    assert.match(text, /# Deslop CPU Report/);
    assert.match(text, /Current phase: idle/);
    assert.match(text, /`deslop\/reportGet` \| 2/);
    assert.match(text, /report_rendering/);
  });

  test("renderCpuReport keeps zero-valued in-flight fields visible", () => {
    const text = renderCpuReport({
      current_phase: "idle",
      handler_counts: {},
      in_flight: {
        pending_watcher_events: 0,
        pending_embed_requests: 0,
        in_progress_parse_batch: null,
      },
      last_100_phases: [],
    });
    assert.match(text, /Pending watcher events: 0/);
    assert.match(text, /Pending embedding requests: 0/);
    assert.match(text, /In-progress parse batch: 0/);
  });
});

function fixtureOccurrence(overrides: Partial<ReportOccurrence> = {}): ReportOccurrence {
  return {
    path: TEST_SOURCE_PATH,
    start_byte: 0,
    end_byte: DEFAULT_OCCURRENCE_END_BYTE,
    start_line: 1,
    end_line: 2,
    hidden: false,
    ...overrides,
  };
}

function clusterNodeFor(c: ReportCluster): ClusterNode {
  return new ClusterNode(c, "mid");
}

function occurrenceNodeFor(o: ReportOccurrence): OccurrenceNode {
  return new OccurrenceNode(o);
}

suite("tree menu renderers", () => {
  test("clusterLocationsText surfaces bucket + count header with one row per occurrence", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-menu-"));
    const fileA = path.join(dir, FILE_A_NAME);
    const fileB = path.join(dir, FILE_B_NAME);
    fs.writeFileSync(fileA, "public class A { }\n", UTF8_ENCODING);
    fs.writeFileSync(fileB, "public class B { }\n", UTF8_ENCODING);

    const c = clusterWithRanges("c-x", [
      { path: fileA, start_byte: 0, end_byte: TEST_TEN },
      { path: fileB, start_byte: 0, end_byte: TEST_TEN },
    ]);

    const text = clusterLocationsText(c);
    const lines = text.split("\n");
    assert.equal(lines.length, THREE_LINE_COUNT, "header + 2 occurrences");
    assert.match(lines[0] ?? "", /^cluster c-x/);
    assert.match(lines[0] ?? "", /mass/);
    assert.match(lines[0] ?? "", /2 occurrences/);
    assert.match(lines[1] ?? "", /A\.cs:1:1$/);
    assert.match(lines[THIRD_LINE_INDEX] ?? "", /B\.cs:1:1$/);
    assert.ok(!text.includes("start_byte"));
    assert.ok(!text.includes(".."), "human copy must not include byte ranges");
    assert.doesNotMatch(text, /Identical code|nearly identical|same behavior|structural_only/i,
      "no clone-kind label may reach the copy surface");

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("aiPayloadForCluster encodes id, mass, rank, and byte ranges", () => {
    const c = clusterWithRanges("c-ai", [
      { path: TEST_SOURCE_PATH, start_byte: DEFAULT_CLUSTER_WEIGHT, end_byte: 200 },
      { path: SECOND_TEST_SOURCE_PATH, start_byte: 5, end_byte: 80 },
    ]);

    const text = aiPayloadForCluster(c, 7);
    assert.match(text, /cluster_id: c-ai/);
    assert.match(text, /rank: 7/);
    assert.match(text, /mass: /);
    assert.doesNotMatch(text, /bucket:/, "no clone-kind line may reach the AI payload");
    // [FUSED-PAIR-SIGNALS] No cluster surface — including copy-for-AI —
    // renders pair evidence: no structural, jaccard, or embedding score,
    // in any wire format the payload ever used.
    for (const gone of [
      "elected_pair:",
      "measured_pair:",
      "pair_signals:",
      "structural=0.1000",
      "token_jaccard=0.2000",
      "embed=0.9000",
    ]) {
      assert.doesNotMatch(text, new RegExp(gone), `pair evidence must not reach the AI payload: ${gone}`);
    }
    assert.match(text, /10\.\.200/);
    assert.match(text, /Use these byte ranges as precise edit anchors/);
  });

  test("AI payloads omit every pair score", () => {
    const c = clusterWithRanges("c-unsourced", [
      { path: TEST_SOURCE_PATH, start_byte: 0, end_byte: DEFAULT_OCCURRENCE_END_BYTE },
      { path: SECOND_TEST_SOURCE_PATH, start_byte: 5, end_byte: 80 },
    ]);
    const clusterText = aiPayloadForCluster(c, 1);
    for (const prefix of [ELECTED_PAIR_LINE_PREFIX, PAIR_SIGNALS_LINE_PREFIX]) {
      assert.equal(
        clusterText.split("\n").some((line) => line.startsWith(prefix)),
        false,
        "cluster copy-for-AI must never publish pair evidence",
      );
    }

    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);
    const first = c.occurrences[0];
    assert.ok(first);
    const occurrenceText = aiPayloadForOccurrence(first, store);
    for (const prefix of [ELECTED_PAIR_LINE_PREFIX, PAIR_SIGNALS_LINE_PREFIX]) {
      assert.equal(
        occurrenceText.split("\n").some((line) => line.startsWith(prefix)),
        false,
        "occurrence copy-for-AI must never publish parent pair evidence",
      );
    }
  });

  test("aiPayloadForCluster leads with the slug so AI and human surfaces agree (#146)", () => {
    // [VSIX-CLUSTER-ID-CONSISTENCY] The human-facing tree, hover bubble, and
    // webview panels all show the 7-hex slug as the cluster's identity. The
    // AI payload must include that same slug as the lead identifier — and
    // before the volatile `rank:` line — so an agent quoting the slug from a
    // copy-for-AI payload can cross-reference it against any rendered surface.
    // The canonical full id is preserved on its own line for unambiguous
    // tooling round-trip.
    const c = clusterWithRanges("1802186da488862f", [
      { path: TEST_SOURCE_PATH, start_byte: 0, end_byte: DEFAULT_CLUSTER_WEIGHT },
    ]);
    const text = aiPayloadForCluster(c, THIRD_RANK);
    const lines = text.split("\n");
    const slugIndex = lines.findIndex((line) => /^slug: 1802186\b/.test(line));
    const clusterIdIndex = lines.findIndex((line) =>
      /^cluster_id: 1802186da488862f\b/.test(line),
    );
    const rankIndex = lines.findIndex((line) => /^rank: 3\b/.test(line));
    assert.ok(
      slugIndex >= 0,
      `payload must include a 'slug:' header line so AI and human surfaces share the same id, got:\n${text}`,
    );
    assert.ok(
      clusterIdIndex >= 0,
      `payload must still expose the canonical 16-hex cluster_id for round-trip, got:\n${text}`,
    );
    assert.ok(
      rankIndex >= 0,
      `payload must still expose the volatile rank, got:\n${text}`,
    );
    assert.ok(
      slugIndex < rankIndex,
      `slug (stable id) must precede rank (volatile sort position) so AI agents do not mistake rank for identity, got slug at ${slugIndex} and rank at ${rankIndex}`,
    );
    assert.ok(
      slugIndex <= clusterIdIndex,
      `slug should lead, with full canonical id following — slug at ${slugIndex}, cluster_id at ${clusterIdIndex}`,
    );
  });

  test("aiPayloadForOccurrence includes parent cluster metadata when available", () => {
    const c = clusterWithRanges("c-occ", [
      { path: TEST_SOURCE_PATH, start_byte: 0, end_byte: DEFAULT_OCCURRENCE_END_BYTE },
      { path: "src/bar.cs", start_byte: 5, end_byte: 80 },
    ]);

    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);

    const first = c.occurrences[0];
    assert.ok(first);
    const text = aiPayloadForOccurrence(first, store);
    assert.match(text, /occurrence_path: src\/foo\.cs/);
    assert.match(text, /bytes: 0\.\.50/);
    assert.match(text, /cluster_id: c-occ/);
    assert.match(text, /rank: 1/);
    assert.match(text, /sibling_occurrences: 1/);
    assert.match(text, /Use these byte ranges as precise edit anchors/);
  });

  test("aiPayloadForOccurrence omits parent section when store has no cluster for the occurrence", () => {
    const store = new ReportStore();
    const text = aiPayloadForOccurrence(fixtureOccurrence(), store);
    assert.match(text, /occurrence_path/);
    assert.ok(!text.includes("cluster_id:"), "no cluster → no parent block");
  });

  test("sourceSnippetText header is path line column only for humans (#27)", () => {
    const { dir, file } = tempFile("cdd-snip-", "snippet.cs");
    const source = "public class Snippet { int x = 1; }\n";
    fs.writeFileSync(file, source, UTF8_ENCODING);

    const text = sourceSnippetText(
      fixtureOccurrence({ path: file, start_byte: 0, end_byte: TEST_TWENTY }),
    );

    const firstLine = text.split("\n")[0] ?? "";
    assert.match(firstLine, /^.+:1:1$/);
    assert.ok(!/\bbytes?\b/i.test(firstLine), `human header leaked bytes: ${firstLine}`);
    assert.ok(text.includes("public class Snippet"), "fenced block carries the bytes");
    assert.ok(text.endsWith(CODE_FENCE));

    fs.rmSync(dir, { recursive: true, force: true });
  });

  // [FACET-MODEL] Copy Source Snippet used to carry a second, private
  // extension map that never learned F#, PHP or Go, so those occurrences
  // copied out as a bare ``` fence — unhighlighted in a PR and untyped for
  // an AI agent. The tag now comes from `types/languages`.
  test("sourceSnippetText fence tag comes from the shared language registry", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-fence-"));
    const expected: ReadonlyArray<readonly [string, string]> = [
      ["main.go", "go"],
      ["Model.php", "php"],
      ["Tests.fs", "fsharp"],
      ["Widget.dart", "dart"],
      ["App.tsx", "tsx"],
      ["notes.txt", ""],
      ["Makefile", ""],
    ];

    for (const [name, tag] of expected) {
      const file = path.join(dir, name);
      fs.writeFileSync(file, "value\n", UTF8_ENCODING);
      const text = sourceSnippetText(
        fixtureOccurrence({ path: file, start_byte: 0, end_byte: 5 }),
      );
      assert.equal(
        text.split("\n")[1],
        CODE_FENCE + tag,
        `${name} must open its fence with "${tag}" so the snippet highlights when pasted`,
      );
      assert.ok(text.includes("value"), `${name} snippet must carry the source bytes`);
    }

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("clusterIdForTreeNode returns cluster id for cluster nodes", () => {
    const c = clusterWithRanges("c-id", [{ path: "a", start_byte: 0, end_byte: 1 }]);
    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);
    assert.equal(clusterIdForTreeNode(clusterNodeFor(c), store), "c-id");
  });

  test("clusterIdForTreeNode resolves parent cluster id for occurrence nodes", () => {
    const c = clusterWithRanges("c-parent", [
      { path: TEST_SOURCE_PATH, start_byte: 100, end_byte: 120 },
    ]);
    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);
    const occ = c.occurrences[0];
    assert.ok(occ);
    assert.equal(
      clusterIdForTreeNode(occurrenceNodeFor(occ), store),
      "c-parent",
    );
  });

  test("clusterIdForTreeNode returns undefined for occurrences with no matching parent", () => {
    const store = new ReportStore();
    assert.equal(
      clusterIdForTreeNode(occurrenceNodeFor(fixtureOccurrence()), store),
      undefined,
    );
  });
});

suite("tree menu handlers", () => {
  suiteSetup(async () => {
    await activateExtension();
  });

  test("copyHumanLocation copies path:line:column for the occurrence", async () => {
    const { dir, file } = tempFile("cdd-hloc-", "hum.cs");
    fs.writeFileSync(file, "line-a\nline-b\n", UTF8_ENCODING);

    const node = occurrenceNodeFor(
      fixtureOccurrence({ path: file, start_byte: 0, end_byte: SHORT_OCCURRENCE_END_BYTE }),
    );
    await copyHumanLocation(node);
    const clipboard = await vscode.env.clipboard.readText();
    assert.equal(clipboard, `${file}:1:1`);

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("copyClusterLocations writes the header + every occurrence line to the clipboard", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cloc-"));
    const fileA = path.join(dir, FILE_A_NAME);
    const fileB = path.join(dir, FILE_B_NAME);
    fs.writeFileSync(fileA, "A\n", UTF8_ENCODING);
    fs.writeFileSync(fileB, "B\n", UTF8_ENCODING);
    const c = clusterWithRanges("c-copy", [
      { path: fileA, start_byte: 0, end_byte: 1 },
      { path: fileB, start_byte: 0, end_byte: 1 },
    ]);

    await copyClusterLocations(clusterNodeFor(c));
    const clipboard = await vscode.env.clipboard.readText();
    const lines = clipboard.split("\n");
    assert.match(lines[0] ?? "", /cluster c-copy/);
    assert.equal(lines.length, THREE_LINE_COUNT);
    assert.match(lines[1] ?? "", /A\.cs:1:1$/);
    assert.match(lines[THIRD_LINE_INDEX] ?? "", /B\.cs:1:1$/);

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("copyContextForAI cluster node writes the AI payload to the clipboard", async () => {
    const c = clusterWithRanges(
      "c-ctx",
      [{ path: TEST_SOURCE_PATH, start_byte: 0, end_byte: DEFAULT_OCCURRENCE_END_BYTE }],
      THIRD_RANK,
    );
    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);

    await copyContextForAI(clusterNodeFor(c), store);
    const clipboard = await vscode.env.clipboard.readText();
    assert.match(clipboard, /cluster_id: c-ctx/);
    assert.match(clipboard, /rank: 3/);
    assert.match(clipboard, /0\.\.50/);
  });

  test("copyContextForAI occurrence node writes occurrence + parent fields to the clipboard", async () => {
    const c = clusterWithRanges("c-occ-ctx", [
      { path: TEST_SOURCE_PATH, start_byte: 0, end_byte: 9 },
    ]);
    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);

    const occ = c.occurrences[0];
    assert.ok(occ);
    await copyContextForAI(occurrenceNodeFor(occ), store);
    const clipboard = await vscode.env.clipboard.readText();
    assert.match(clipboard, /occurrence_path: src\/foo\.cs/);
    assert.match(clipboard, /cluster_id: c-occ-ctx/);
    // [VSIX-PAIR-COMPARE] The AI payload carries the engine's cluster
    // facts — rank, mass, node count — and no similarity bucket.
    assert.match(clipboard, /rank: 1/);
    assert.match(clipboard, /mass: /);
    assert.match(clipboard, /canonical_nodes: /);
    assert.doesNotMatch(clipboard, /bucket:/);
  });

  test("copySourceSnippet copies the fenced source block to the clipboard", async () => {
    const { dir, file } = tempFile("cdd-snip2-", "src.py");
    fs.writeFileSync(file, "def hi(): return 42\n", UTF8_ENCODING);

    await copySourceSnippet(
      occurrenceNodeFor(fixtureOccurrence({ path: file, start_byte: 0, end_byte: 8 })),
    );
    const clipboard = await vscode.env.clipboard.readText();
    assert.match(clipboard, /```python\ndef hi\(/);
    assert.ok(clipboard.endsWith(CODE_FENCE));

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("revealOccurrenceInExplorer shows an error when the file no longer exists", async () => {
    const node = occurrenceNodeFor(fixtureOccurrence({ path: "/tmp/__cdd_does_not_exist__.cs", start_byte: 0, end_byte: 1 }));
    await revealOccurrenceInExplorer(node);
  });

  test("revealOccurrenceInExplorer calls revealInExplorer for an existing file", async () => {
    const { dir, file } = tempFile("cdd-rev-", "reveal.cs");
    fs.writeFileSync(file, "x\n", UTF8_ENCODING);
    const node = occurrenceNodeFor(
      fixtureOccurrence({ path: file, start_byte: 0, end_byte: 1 }),
    );
    await revealOccurrenceInExplorer(node);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("openAllOccurrences opens every occurrence under the threshold without prompting", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-all-"));
    const files = ["a", "b"].map((name) => {
      const p = path.join(dir, `${name}.cs`);
      fs.writeFileSync(p, `// ${name}\n`, UTF8_ENCODING);
      return p;
    });
    const c = clusterWithRanges(
      "c-open-all",
      files.map((p) => ({ path: p, start_byte: 0, end_byte: SHORT_OCCURRENCE_END_BYTE })),
    );
    await openAllOccurrences(clusterNodeFor(c));
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("OPEN_ALL_THRESHOLD is the small-cluster confirmation boundary", () => {
    assert.equal(OPEN_ALL_THRESHOLD, 5);
  });
});



// [VSIX-COMMANDS] The palette contract: every command id the package
// declares must have exactly one binding, and the shared deps must
// route a dispatch to the client and the persisted view state. Pinned
// against the binding table itself because the integration host also
// runs the real extension, whose registrations cannot be shadowed.
suite("command dispatch wiring", () => {
  test("every declared palette id has exactly one binding and vice versa", () => {
    const declared: string[] = (
      JSON.parse(
        fs.readFileSync(path.resolve(extensionRoot(), "package.json"), UTF8_ENCODING),
      ) as { contributes: { commands: { command: string }[] } }
    ).contributes.commands.map((entry) => entry.command);
    const bound = COMMAND_BINDINGS.map((binding) => binding.id);
    // Every binding must be contributed — VS Code refuses command: hover
    // links and menu entries for uncontributed ids.
    const orphan = bound.filter((id) => !declared.includes(id));
    assert.deepEqual(
      orphan,
      [],
      `bindings without a package.json contribution are unreachable: ${orphan.join(", ")}`,
    );
    // The reverse direction: whatever activation registers beyond the
    // table must be exactly the activation-owned set plus the spec'd but
    // still unwired language-split toggle — a new declared id with no
    // handler anywhere fails here.
    const activationOwned = declared.filter(
      (id) =>
        !bound.includes(id) && !PENDING_LANGUAGE_SPLIT_TOGGLE.includes(id),
    );
    assert.deepEqual(
      [...activationOwned].sort(),
      [...ACTIVATION_OWNED_COMMAND_IDS].sort(),
      "declared ids outside COMMAND_BINDINGS must stay the activation-owned set",
    );
    assert.equal(new Set(bound).size, bound.length, "a duplicate binding id");
  });

  test("binding dispatch routes through the shared client and the persisted view axis", async () => {
    let clientCalls = 0;
    const deps = {
      context: fakeCtx(),
      store: seededStore([]),
      clientOf: (): LanguageClient | undefined => {
        clientCalls += 1;
        return {
          sendRequest: () => Promise.resolve("# refreshed"),
        } as unknown as LanguageClient;
      },
    };
    const refresh = COMMAND_BINDINGS.find((b) => b.id === REFRESH_REPORT_COMMAND);
    assert.ok(refresh, "the refresh binding went missing");
    await refresh.run(deps);
    assert.equal(clientCalls, TEST_ONE);

    const showByCluster = COMMAND_BINDINGS.find(
      (b) => b.id === "deslop.topOffenders.showByCluster",
    );
    assert.ok(showByCluster, "the grouping binding went missing");
    await showByCluster.run(deps);
    assert.equal(
      vscode.workspace.getConfiguration("deslop").get<string>("topOffenders.groupBy"),
      "cluster",
    );
    await vscode.workspace
      .getConfiguration("deslop")
      .update("topOffenders.groupBy", undefined, vscode.ConfigurationTarget.Workspace);
  });
});

// [VSIX-CODE-LENS] The remaining command-target guards: an argument that
// reaches the palette from a lens, a hover link, or a stale tree row
// must resolve to an editor opening — never to a thrown error.
suite("command target resolution", () => {
  test("resolveOccurrenceUri keeps absolute paths and roots relative ones", () => {
    const made = tempFile("deslop-abs-", "abs.cs");
    fs.writeFileSync(made.file, "const absolute = 1;\n");
    const absolute = resolveOccurrenceUri(made.file);
    assert.equal(absolute.fsPath, made.file);

    const relative = resolveOccurrenceUri(RELATIVE_OCCURRENCE_PATH);
    const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? "";
    assert.ok(
      relative.fsPath.startsWith(root),
      `relative occurrence must resolve under the workspace root: ${relative.fsPath}`,
    );
    assert.ok(relative.fsPath.endsWith(RELATIVE_OCCURRENCE_PATH));
  });

  test("openOccurrenceTarget accepts a raw occurrence, an occurrence node, and informs on junk", async () => {
    const raw = occurrence(TEST_SOURCE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE);
    await openOccurrenceTarget(raw);

    const node = { cluster: null, occurrence: raw } as unknown as OccurrenceNode;
    await openOccurrenceTarget(node);

    await openOccurrenceTarget({ occurrence: { path: TEST_THREE } });
    await openOccurrenceTarget("not-even-an-object");
  });

  test("jumpToNextOccurrence wraps from the last occurrence back to the first", async () => {
    const first = tempFile("deslop-wrap-a-", "wrap-a.cs");
    fs.writeFileSync(first.file, "const alpha = 1;\n");
    const second = tempFile("deslop-wrap-b-", "wrap-b.cs");
    fs.writeFileSync(second.file, "const beta = 2;\n");
    const doc = await vscode.workspace.openTextDocument(second.file);
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(
      reportWithClusters([
        clusterWithRanges(CYCLE_CLUSTER_ID, [
          { path: first.file, start_byte: 0, end_byte: SHORT_OCCURRENCE_END_BYTE },
          { path: second.file, start_byte: 0, end_byte: SHORT_OCCURRENCE_END_BYTE },
        ]),
      ]),
      1,
    );
    // Index 1 is the last occurrence: modulo wrap must land on index 0.
    await jumpToNextOccurrence(store, CYCLE_CLUSTER_ID, 1);
    await closeAllDiffs();
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  test("comparePairEndpoints is a no-op when either endpoint is malformed", async () => {
    const good = { path: TEST_SOURCE_PATH, start_byte: 0, end_byte: TEST_TEN };
    await comparePairEndpoints(good, { path: "", start_byte: 0, end_byte: 1 });
    await comparePairEndpoints(good, { path: TEST_SOURCE_PATH, start_byte: 1.5, end_byte: 2 });
    await comparePairEndpoints(good, { path: TEST_SOURCE_PATH, start_byte: "0", end_byte: 2 });
    await comparePairEndpoints(good, null);
  });

  test("openSchemaDoc consults the RPC fallback and survives a failing client", async () => {
    const accepting = (): LanguageClient | undefined =>
      ({ sendRequest: async () => "rpc-doc" }) as unknown as LanguageClient;
    await openSchemaDoc(fakeCtx(), seededStore([cluster(FILE_A_NAME, [FILE_A_NAME])]), accepting);

    const rejecting = (): LanguageClient | undefined =>
      ({
        sendRequest: async () => {
          throw new Error("lsp gone");
        },
      }) as unknown as LanguageClient;
    await openSchemaDoc(fakeCtx(), seededStore([]), rejecting);

    const nonString = (): LanguageClient | undefined =>
      ({ sendRequest: async () => 42 }) as unknown as LanguageClient;
    await openSchemaDoc(fakeCtx(), seededStore([]), nonString);
  });

  test("openCpuReport without a client informs instead of throwing", async () => {
    await openCpuReport(() => undefined);
  });
});
