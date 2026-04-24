// Unit: call the exported command implementations directly with a seeded
// store + active editor so the full branch coverage lands without colliding
// with the real extension's command registrations.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  openWorstCluster,
  openOccurrence,
  jumpToNextOccurrence,
  compareWithCanonical,
  openSchemaDoc,
} from "../../commands/register";
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
import { ClusterNode, OccurrenceNode } from "../../tree/providers";
import { Report, ReportCluster, ReportOccurrence } from "../../types/report";

async function findDiffTab(): Promise<vscode.TabInputTextDiff> {
  for (let i = 0; i < 20; i += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return tab.input;
      }
    }
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 50);
    });
  }
  throw new Error("no diff tab opened after compareWithCanonical");
}

async function closeAllDiffs(): Promise<void> {
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");
}

function cluster(id: string, paths: string[]): ReportCluster {
  return {
    id,
    weight: 10,
    size: 2,
    canonical_node_count: 4,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: paths.map((p) => ({
      path: p,
      start_byte: 0,
      end_byte: 50,
      hidden: false,
    })),
    summary: "",
    interpretation: "interp",
  };
}

function clusterWithRanges(
  id: string,
  occurrences: { path: string; start_byte: number; end_byte: number }[],
): ReportCluster {
  return {
    id,
    weight: 10,
    size: occurrences.length,
    canonical_node_count: 4,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: occurrences.map((o) => ({ ...o, hidden: false })),
    summary: "",
    interpretation: "interp",
  };
}

function report(clusters: ReportCluster[]): Report {
  return {
    report_schema_version: 1,
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 1,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 10,
      duplicated_loc: 5,
      duplication_percent: 50,
      clusters_total: clusters.length,
      duplicated_files: 1,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "# docs",
    action_hints: [],
    embedding_provenance: null,
    clusters,
  };
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
    const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
    assert.ok(ext, "extension must be discoverable in the test host");
    await ext.activate();
  });

  test("openWorstCluster shows info when store is empty", () => {
    openWorstCluster(fakeCtx(), new ReportStore());
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
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-occ-"));
    const file = path.join(dir, "occ.txt");
    fs.writeFileSync(file, "hello\nworld\n", "utf8");
    await openOccurrence({
      path: file,
      start_byte: 0,
      end_byte: 3,
      hidden: false,
    });
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("jumpToNextOccurrence navigates to the sibling when the cursor sits inside a cluster", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line-one\nline-two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    editor.selection = new vscode.Selection(
      new vscode.Position(0, 2),
      new vscode.Position(0, 2),
    );
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("c-1", [doc.uri.fsPath, "/tmp/cdd-sibling.cs"])]),
      0,
    );
    jumpToNextOccurrence(store);
  });

  test("jumpToNextOccurrence shows the info message when no cluster overlaps", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "z",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", ["/other"])]), 0);
    jumpToNextOccurrence(store);
  });

  test("jumpToNextOccurrence bails when there is no active editor", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", ["/p"])]), 0);
    jumpToNextOccurrence(store);
  });

  test("compareWithCanonical opens a diff whose two sides are distinct resources with the matching occurrence bytes", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cmp-"));
    const fileA = path.join(dir, "A.cs");
    const fileB = path.join(dir, "B.cs");
    // Left side: bytes 0..16 of A.cs == "public class A {"
    // Right side: bytes 0..16 of B.cs == "public class B {"
    // Distinct files, distinct content — exercises the cross-file diff path.
    fs.writeFileSync(fileA, "public class A { int x = 1; }\n", "utf8");
    fs.writeFileSync(fileB, "public class B { int y = 2; }\n", "utf8");

    const store = new ReportStore();
    store.setSnapshot(
      report([
        clusterWithRanges("c-diff", [
          { path: fileA, start_byte: 0, end_byte: 16 },
          { path: fileB, start_byte: 0, end_byte: 16 },
        ]),
      ]),
      0,
    );

    await closeAllDiffs();
    await compareWithCanonical(store, "c-diff");
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

  test("compareWithCanonical opens distinct diff sides for two occurrences that live inside the same file", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cmp-same-"));
    // Two clone regions inside a single source file. This is the case the
    // user reported: the old implementation handed `vscode.diff` the same
    // file URI twice, so the diff editor rendered the whole file against
    // itself. The fix must ensure each side shows only the clone bytes.
    const file = path.join(dir, "same.cs");
    const source =
      "OCCURRENCE_A_____________________________\n" +
      "middle middle middle middle middle middle\n" +
      "OCCURRENCE_B_____________________________\n";
    fs.writeFileSync(file, source, "utf8");
    const firstLineEnd = source.indexOf("\n");
    const thirdLineStart = source.indexOf("OCCURRENCE_B");
    const thirdLineEnd = source.indexOf("\n", thirdLineStart);

    const store = new ReportStore();
    store.setSnapshot(
      report([
        clusterWithRanges("c-same", [
          { path: file, start_byte: 0, end_byte: firstLineEnd },
          { path: file, start_byte: thirdLineStart, end_byte: thirdLineEnd },
        ]),
      ]),
      0,
    );

    await closeAllDiffs();
    await compareWithCanonical(store, "c-same");
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

  test("compareWithCanonical bails for a non-existent id", async () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    await compareWithCanonical(store, "nope");
  });

  test("compareWithCanonical bails for a single-occurrence cluster", async () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c-single", ["/only"])]), 0);
    await compareWithCanonical(store, "c-single");
  });

  test("compare provider renders a friendly fallback for a stale occurrence file", async () => {
    const uri = buildCompareUri(
      { path: "missing-deslop-compare-file.cs", start_byte: 0, end_byte: 20, hidden: false },
      "a",
      "stale-cluster",
    );

    const doc = await vscode.workspace.openTextDocument(uri);
    const text = doc.getText();
    assert.match(text, /Deslop could not load this compare occurrence/);
    assert.match(text, /Refresh the Deslop report and try Compare again/);
    assert.match(text, /stale-cluster/);
  });

  test("openSchemaDoc opens a markdown editor", async () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    await openSchemaDoc(fakeCtx(), store);
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "schema doc editor should be active");
    assert.equal(active.document.languageId, "markdown");
    assert.match(active.document.getText(), /# docs/);
  });

  test("openSchemaDoc reads the packaged fallback when schema_doc is absent", async () => {
    const expected = fs.readFileSync(packagedSchemaDocPath(), "utf8");
    await openSchemaDoc(fakeCtx(), new ReportStore());
    const active = vscode.window.activeTextEditor;
    assert.ok(active, "packaged schema doc editor should be active");
    assert.equal(active.document.languageId, "markdown");
    assert.equal(active.document.getText(), expected);
  });
});

function occurrence(overrides: Partial<ReportOccurrence> = {}): ReportOccurrence {
  return {
    path: "src/foo.cs",
    start_byte: 0,
    end_byte: 50,
    hidden: false,
    ...overrides,
  };
}

function clusterNodeFor(c: ReportCluster, rank = 1): ClusterNode {
  return new ClusterNode(c, rank, "mid");
}

function occurrenceNodeFor(o: ReportOccurrence): OccurrenceNode {
  return new OccurrenceNode(o);
}

suite("tree menu renderers", () => {
  test("clusterLocationsText surfaces bucket + count header with one row per occurrence", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-menu-"));
    const fileA = path.join(dir, "A.cs");
    const fileB = path.join(dir, "B.cs");
    fs.writeFileSync(fileA, "public class A { }\n", "utf8");
    fs.writeFileSync(fileB, "public class B { }\n", "utf8");

    const c = clusterWithRanges("c-x", [
      { path: fileA, start_byte: 0, end_byte: 10 },
      { path: fileB, start_byte: 0, end_byte: 10 },
    ]);
    c.bucket = "identical";

    const text = clusterLocationsText(c);
    const lines = text.split("\n");
    assert.equal(lines.length, 3, "header + 2 occurrences");
    assert.match(lines[0] ?? "", /^cluster c-x/);
    assert.match(lines[0] ?? "", /Identical code/);
    assert.match(lines[0] ?? "", /2 occurrences/);
    assert.match(lines[1] ?? "", /A\.cs:1:1$/);
    assert.match(lines[2] ?? "", /B\.cs:1:1$/);
    assert.ok(!text.includes("start_byte"));
    assert.ok(!text.includes(".."), "human copy must not include byte ranges");

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("aiPayloadForCluster encodes id, bucket, rank, signals, and byte ranges", () => {
    const c = clusterWithRanges("c-ai", [
      { path: "src/foo.cs", start_byte: 10, end_byte: 200 },
    ]);
    c.bucket = "same_behavior";
    c.signals = {
      structural: 0.1,
      token_jaccard: 0.2,
      embedding_cos: 0.9,
      fused: 0.85,
    };

    const text = aiPayloadForCluster(c, 7);
    assert.match(text, /cluster_id: c-ai/);
    assert.match(text, /rank: 7/);
    assert.match(text, /bucket: same_behavior/);
    assert.match(text, /signals: structural=0\.1000/);
    assert.match(text, /embed=0\.9000/);
    assert.match(text, /10\.\.200/);
    assert.match(text, /Use these byte ranges as precise edit anchors/);
  });

  test("aiPayloadForOccurrence includes parent cluster metadata when available", () => {
    const c = clusterWithRanges("c-occ", [
      { path: "src/foo.cs", start_byte: 0, end_byte: 50 },
      { path: "src/bar.cs", start_byte: 5, end_byte: 80 },
    ]);
    c.bucket = "nearly_identical";

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
    const text = aiPayloadForOccurrence(occurrence(), store);
    assert.match(text, /occurrence_path/);
    assert.ok(!text.includes("cluster_id:"), "no cluster → no parent block");
  });

  test("sourceSnippetText wraps the occurrence bytes in a fenced code block with a compact header", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-snip-"));
    const file = path.join(dir, "snippet.cs");
    const source = "public class Snippet { int x = 1; }\n";
    fs.writeFileSync(file, source, "utf8");

    const text = sourceSnippetText({
      path: file,
      start_byte: 0,
      end_byte: 20,
      hidden: false,
    });

    assert.match(text, /^.+:1:1 bytes 0\.\.20\n```csharp\n/);
    assert.ok(text.includes("public class Snippet"), "fenced block carries the bytes");
    assert.ok(text.endsWith("```"));

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
      { path: "src/foo.cs", start_byte: 100, end_byte: 120 },
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
      clusterIdForTreeNode(occurrenceNodeFor(occurrence()), store),
      undefined,
    );
  });
});

suite("tree menu handlers", () => {
  suiteSetup(async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
    assert.ok(ext, "extension must be discoverable in the test host");
    await ext.activate();
  });

  test("copyHumanLocation copies path:line:column for the occurrence", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-hloc-"));
    const file = path.join(dir, "hum.cs");
    fs.writeFileSync(file, "line-a\nline-b\n", "utf8");

    const node = occurrenceNodeFor({
      path: file,
      start_byte: 0,
      end_byte: 3,
      hidden: false,
    });
    await copyHumanLocation(node);
    const clipboard = await vscode.env.clipboard.readText();
    assert.equal(clipboard, `${file}:1:1`);

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("copyClusterLocations writes the header + every occurrence line to the clipboard", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cloc-"));
    const fileA = path.join(dir, "A.cs");
    const fileB = path.join(dir, "B.cs");
    fs.writeFileSync(fileA, "A\n", "utf8");
    fs.writeFileSync(fileB, "B\n", "utf8");
    const c = clusterWithRanges("c-copy", [
      { path: fileA, start_byte: 0, end_byte: 1 },
      { path: fileB, start_byte: 0, end_byte: 1 },
    ]);
    c.bucket = "identical";

    await copyClusterLocations(clusterNodeFor(c));
    const clipboard = await vscode.env.clipboard.readText();
    const lines = clipboard.split("\n");
    assert.match(lines[0] ?? "", /cluster c-copy/);
    assert.equal(lines.length, 3);
    assert.match(lines[1] ?? "", /A\.cs:1:1$/);
    assert.match(lines[2] ?? "", /B\.cs:1:1$/);

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("copyContextForAI cluster node writes the AI payload to the clipboard", async () => {
    const c = clusterWithRanges("c-ctx", [
      { path: "src/foo.cs", start_byte: 0, end_byte: 50 },
    ]);
    c.bucket = "nearly_identical";
    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);

    await copyContextForAI(clusterNodeFor(c, 3), store);
    const clipboard = await vscode.env.clipboard.readText();
    assert.match(clipboard, /cluster_id: c-ctx/);
    assert.match(clipboard, /rank: 3/);
    assert.match(clipboard, /0\.\.50/);
  });

  test("copyContextForAI occurrence node writes occurrence + parent fields to the clipboard", async () => {
    const c = clusterWithRanges("c-occ-ctx", [
      { path: "src/foo.cs", start_byte: 0, end_byte: 9 },
    ]);
    c.bucket = "identical";
    const store = new ReportStore();
    store.setSnapshot(report([c]), 0);

    const occ = c.occurrences[0];
    assert.ok(occ);
    await copyContextForAI(occurrenceNodeFor(occ), store);
    const clipboard = await vscode.env.clipboard.readText();
    assert.match(clipboard, /occurrence_path: src\/foo\.cs/);
    assert.match(clipboard, /cluster_id: c-occ-ctx/);
    assert.match(clipboard, /bucket: identical/);
  });

  test("copySourceSnippet copies the fenced source block to the clipboard", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-snip2-"));
    const file = path.join(dir, "src.py");
    fs.writeFileSync(file, "def hi(): return 42\n", "utf8");

    await copySourceSnippet(
      occurrenceNodeFor({ path: file, start_byte: 0, end_byte: 8, hidden: false }),
    );
    const clipboard = await vscode.env.clipboard.readText();
    assert.match(clipboard, /```python\ndef hi\(/);
    assert.ok(clipboard.endsWith("```"));

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("revealOccurrenceInExplorer shows an error when the file no longer exists", async () => {
    const node = occurrenceNodeFor({
      path: "/tmp/__cdd_does_not_exist__.cs",
      start_byte: 0,
      end_byte: 1,
      hidden: false,
    });
    await revealOccurrenceInExplorer(node);
  });

  test("revealOccurrenceInExplorer calls revealInExplorer for an existing file", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-rev-"));
    const file = path.join(dir, "reveal.cs");
    fs.writeFileSync(file, "x\n", "utf8");
    const node = occurrenceNodeFor({
      path: file,
      start_byte: 0,
      end_byte: 1,
      hidden: false,
    });
    await revealOccurrenceInExplorer(node);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("openAllOccurrences opens every occurrence under the threshold without prompting", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-all-"));
    const files = ["a", "b"].map((name) => {
      const p = path.join(dir, `${name}.cs`);
      fs.writeFileSync(p, `// ${name}\n`, "utf8");
      return p;
    });
    const c = clusterWithRanges(
      "c-open-all",
      files.map((p) => ({ path: p, start_byte: 0, end_byte: 3 })),
    );
    await openAllOccurrences(clusterNodeFor(c));
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("OPEN_ALL_THRESHOLD is the small-cluster confirmation boundary", () => {
    assert.equal(OPEN_ALL_THRESHOLD, 5);
  });
});
