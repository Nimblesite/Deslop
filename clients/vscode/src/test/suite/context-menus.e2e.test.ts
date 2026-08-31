// E2E: drive the tree context-menu commands via the real VS Code command
// registry and assert clipboard state / editor state end-to-end. Issues
// #11, #12, #13, #15, #16, #17, #19.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

import { ClusterNode, OccurrenceNode } from "../../tree/providers";
import type { ReportCluster, ReportOccurrence } from "../../types/report";
import { activateExtension } from "./helpers";
import { occurrence, wireCluster } from "../cluster.helpers";

const UTF8_ENCODING = "utf8";
const CLOSE_ALL_EDITORS_COMMAND = "workbench.action.closeAllEditors";

function cluster(
  id: string,
  occurrences: { path: string; start_byte: number; end_byte: number }[],
  rank = 1,
): ReportCluster {
  return wireCluster({
    id,
    rank,
    mass: 42,
    canonical_node_count: 12,
    occurrences: occurrences.map((o) =>
      occurrence(o.path, o.start_byte, o.end_byte),
    ),
  });
}

function clusterNode(c: ReportCluster): ClusterNode {
  return new ClusterNode(c, "mid");
}

function occurrenceNode(o: ReportOccurrence): OccurrenceNode {
  return new OccurrenceNode(o);
}

suite("tree context menu commands", () => {
  suiteSetup(async () => {
    await activateExtension();
  });

  test("deslop.copyHumanLocation writes path:line:column to the clipboard", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-hum-"));
    const file = path.join(dir, "hum.cs");
    fs.writeFileSync(file, "a\nb\nc\n", UTF8_ENCODING);

    const node = occurrenceNode(occurrence(file, 2, 3));
    await vscode.commands.executeCommand("deslop.copyHumanLocation", node);
    const text = await vscode.env.clipboard.readText();
    assert.equal(text, `${file}:2:1`);

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.copyClusterLocations copies the cluster header + every row", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-cl-"));
    const fileA = path.join(dir, "A.cs");
    const fileB = path.join(dir, "B.cs");
    fs.writeFileSync(fileA, "A\n", UTF8_ENCODING);
    fs.writeFileSync(fileB, "B\n", UTF8_ENCODING);

    const c = cluster("c-e2e-cl", [
      { path: fileA, start_byte: 0, end_byte: 1 },
      { path: fileB, start_byte: 0, end_byte: 1 },
    ]);
    await vscode.commands.executeCommand("deslop.copyClusterLocations", clusterNode(c));
    const text = await vscode.env.clipboard.readText();
    const lines = text.split("\n");
    assert.match(lines[0] ?? "", /cluster c-e2e-cl/);
    assert.equal(lines.length, 3);
    assert.ok(!text.includes(".."), "human copy must not include byte ranges");

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.copyContextForAI on a cluster embeds byte ranges for tool consumption", async () => {
    const c = cluster(
      "c-e2e-ai",
      [{path: "src/foo.cs", start_byte: 0, end_byte: 123}],
      9,
    );
    await vscode.commands.executeCommand("deslop.copyContextForAI", clusterNode(c));
    const text = await vscode.env.clipboard.readText();
    assert.match(text, /cluster_id: c-e2e-ai/);
    assert.match(text, /rank: 9/);
    assert.match(text, /0\.\.123/);
  });

  test("deslop.copySourceSnippet wraps the occurrence bytes in a fenced block", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-snip-"));
    const file = path.join(dir, "snip.py");
    fs.writeFileSync(file, "print('hi')\n", UTF8_ENCODING);

    await vscode.commands.executeCommand(
      "deslop.copySourceSnippet",
      occurrenceNode(occurrence(file, 0, 11)),
    );
    const text = await vscode.env.clipboard.readText();
    assert.match(text, /```python\nprint\('hi'\)/);
    assert.ok(text.endsWith("```"));

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.revealOccurrenceInExplorer resolves a workspace path without throwing", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-rev-"));
    const file = path.join(dir, "rev.cs");
    fs.writeFileSync(file, "x\n", UTF8_ENCODING);
    await vscode.commands.executeCommand(
      "deslop.revealOccurrenceInExplorer",
      occurrenceNode(occurrence(file, 0, 1)),
    );
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.openOccurrence accepts an occurrence tree row", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-go-"));
    const file = path.join(dir, "go.cs");
    fs.writeFileSync(file, "zero\none\ntwo\n", UTF8_ENCODING);

    try {
      await vscode.commands.executeCommand(
        "deslop.openOccurrence",
        occurrenceNode(occurrence(file, 5, 8)),
      );

      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "occurrence command must open an editor");
      assert.equal(editor.document.uri.fsPath, file);
      assert.equal(editor.selection.start.line, 1);
      assert.equal(editor.selection.start.character, 0);
      assert.equal(editor.selection.end.character, 3);
    } finally {
      await vscode.commands.executeCommand(CLOSE_ALL_EDITORS_COMMAND);
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("deslop.openCanonicalFile opens the cluster's first occurrence by line and column", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-canon-"));
    const canonical = path.join(dir, "Canonical.cs");
    const sibling = path.join(dir, "Sibling.cs");
    const source = "header\n    canonical call\n";
    const startByte = Buffer.byteLength("header\n    ", "utf8");
    const endByte = startByte + Buffer.byteLength("canonical", "utf8");
    fs.writeFileSync(canonical, source, UTF8_ENCODING);
    fs.writeFileSync(sibling, "sibling call\n", UTF8_ENCODING);

    try {
      const c = cluster("c-e2e-canon", [
        { path: canonical, start_byte: startByte, end_byte: endByte },
        { path: sibling, start_byte: 0, end_byte: 7 },
      ]);
      await vscode.commands.executeCommand("deslop.openCanonicalFile", clusterNode(c));

      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "canonical command must open an editor");
      assert.equal(editor.document.uri.fsPath, canonical);
      assert.equal(editor.selection.start.line, 1);
      assert.equal(editor.selection.start.character, 4);
      assert.equal(editor.selection.end.character, 13);
    } finally {
      await vscode.commands.executeCommand(CLOSE_ALL_EDITORS_COMMAND);
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("deslop.openAllOccurrences opens every occurrence under the threshold", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-all-"));
    const files = ["alpha", "beta"].map((name) => {
      const p = path.join(dir, `${name}.cs`);
      fs.writeFileSync(p, `// ${name}\n`, "utf8");
      return p;
    });
    await vscode.commands.executeCommand(CLOSE_ALL_EDITORS_COMMAND);
    const c = cluster(
      "c-e2e-all",
      files.map((p) => ({ path: p, start_byte: 0, end_byte: 3 })),
    );
    await vscode.commands.executeCommand("deslop.openAllOccurrences", clusterNode(c));

    const openPaths = new Set<string>();
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputText) {
          openPaths.add(tab.input.uri.fsPath);
        }
      }
    }
    for (const file of files) {
      assert.ok(openPaths.has(file), `expected tab for ${file}`);
    }
    await vscode.commands.executeCommand(CLOSE_ALL_EDITORS_COMMAND);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.openClusterDetails is a no-op for an orphan occurrence node", async () => {
    await vscode.commands.executeCommand(
      "deslop.openClusterDetails",
      occurrenceNode({path: "/tmp/__cdd_no_parent__.cs",
        start_byte: 0,
        end_byte: 1,
        hidden: false, start_line: 1, end_line: 2}),
    );
  });
});
