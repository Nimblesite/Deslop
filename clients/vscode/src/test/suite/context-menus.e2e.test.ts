// E2E: drive the tree context-menu commands via the real VS Code command
// registry and assert clipboard state / editor state end-to-end. Issues
// #11, #12, #13, #15, #16, #17, #19.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

import { Bucket, ReportCluster, ReportOccurrence } from "../../types/report";
import { ClusterNode, OccurrenceNode } from "../../tree/providers";
import { activateExtension } from "./helpers";
import { signalsWith } from "../signals.helpers";

function cluster(
  id: string,
  bucket: Bucket,
  occurrences: { path: string; start_byte: number; end_byte: number }[],
): ReportCluster {
  const c: ReportCluster = {
    id,
    weight: 42,
    size: occurrences.length,
    canonical_node_count: 12,
    bucket: "identical",
    signals: signalsWith(bucket, {
      structural: 0.5,
      token_jaccard: 0.6,
      embedding_cos: 0.7,
      fused: 0.8,
    }),
    occurrences: occurrences.map((o) => ({ ...o, hidden: false })),
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
  c.bucket = bucket;
  return c;
}

function clusterNode(c: ReportCluster, rank = 1): ClusterNode {
  return new ClusterNode(c, rank, "mid");
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
    fs.writeFileSync(file, "a\nb\nc\n", "utf8");

    const node = occurrenceNode({ path: file, start_byte: 2, end_byte: 3, hidden: false });
    await vscode.commands.executeCommand("deslop.copyHumanLocation", node);
    const text = await vscode.env.clipboard.readText();
    assert.equal(text, `${file}:2:1`);

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.copyClusterLocations copies the cluster header + every row", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-cl-"));
    const fileA = path.join(dir, "A.cs");
    const fileB = path.join(dir, "B.cs");
    fs.writeFileSync(fileA, "A\n", "utf8");
    fs.writeFileSync(fileB, "B\n", "utf8");

    const c = cluster("c-e2e-cl", "identical", [
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
    const c = cluster("c-e2e-ai", "same_behavior", [
      { path: "src/foo.cs", start_byte: 0, end_byte: 123 },
    ]);
    await vscode.commands.executeCommand("deslop.copyContextForAI", clusterNode(c, 9));
    const text = await vscode.env.clipboard.readText();
    assert.match(text, /cluster_id: c-e2e-ai/);
    assert.match(text, /rank: 9/);
    assert.match(text, /0\.\.123/);
  });

  test("deslop.copySourceSnippet wraps the occurrence bytes in a fenced block", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-snip-"));
    const file = path.join(dir, "snip.py");
    fs.writeFileSync(file, "print('hi')\n", "utf8");

    await vscode.commands.executeCommand(
      "deslop.copySourceSnippet",
      occurrenceNode({ path: file, start_byte: 0, end_byte: 11, hidden: false }),
    );
    const text = await vscode.env.clipboard.readText();
    assert.match(text, /```python\nprint\('hi'\)/);
    assert.ok(text.endsWith("```"));

    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.revealOccurrenceInExplorer resolves a workspace path without throwing", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-rev-"));
    const file = path.join(dir, "rev.cs");
    fs.writeFileSync(file, "x\n", "utf8");
    await vscode.commands.executeCommand(
      "deslop.revealOccurrenceInExplorer",
      occurrenceNode({ path: file, start_byte: 0, end_byte: 1, hidden: false }),
    );
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.openOccurrence accepts an occurrence tree row", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-e2e-go-"));
    const file = path.join(dir, "go.cs");
    fs.writeFileSync(file, "zero\none\ntwo\n", "utf8");

    try {
      await vscode.commands.executeCommand(
        "deslop.openOccurrence",
        occurrenceNode({ path: file, start_byte: 5, end_byte: 8, hidden: false }),
      );

      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "occurrence command must open an editor");
      assert.equal(editor.document.uri.fsPath, file);
      assert.equal(editor.selection.start.line, 1);
      assert.equal(editor.selection.start.character, 0);
      assert.equal(editor.selection.end.character, 3);
    } finally {
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
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
    fs.writeFileSync(canonical, source, "utf8");
    fs.writeFileSync(sibling, "sibling call\n", "utf8");

    try {
      const c = cluster("c-e2e-canon", "identical", [
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
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
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
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const c = cluster(
      "c-e2e-all",
      "identical",
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
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("deslop.openClusterDetails is a no-op for an orphan occurrence node", async () => {
    await vscode.commands.executeCommand(
      "deslop.openClusterDetails",
      occurrenceNode({
        path: "/tmp/__cdd_no_parent__.cs",
        start_byte: 0,
        end_byte: 1,
        hidden: false,
      }),
    );
  });
});
