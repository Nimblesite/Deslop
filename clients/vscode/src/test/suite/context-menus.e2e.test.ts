// E2E: drive the tree context-menu commands via the real VS Code command
// registry and assert clipboard state / editor state end-to-end. Issues
// #11, #12, #13, #15, #16, #17, #19.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

import { ClusterNode, OccurrenceNode } from "../../tree/providers";
import type { ReportCluster, ReportOccurrence } from "../../types/report";
import { activateExtension } from "./helpers";
import { occurrence, wireCluster } from "../cluster.helpers";
import { withTempFile, withTempFiles } from "../unit/temp-file.helpers";

const CLOSE_ALL_EDITORS_COMMAND = "workbench.action.closeAllEditors";
const CANONICAL_FILE_NAME = "Canonical.cs";
const SIBLING_FILE_NAME = "Sibling.cs";
const ALPHA_FILE_NAME = "alpha.cs";
const BETA_FILE_NAME = "beta.cs";

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
  return new ClusterNode(c);
}

function occurrenceNode(o: ReportOccurrence): OccurrenceNode {
  return new OccurrenceNode(o);
}

/** Runs `body`, then closes every editor it opened, so an editor-opening
 * command cannot leak an active editor into the next test. */
async function closingEditors(body: () => Promise<void>): Promise<void> {
  try {
    await body();
  } finally {
    await vscode.commands.executeCommand(CLOSE_ALL_EDITORS_COMMAND);
  }
}

suite("tree context menu commands", () => {
  suiteSetup(async () => {
    await activateExtension();
  });

  test("deslop.copyHumanLocation writes path:line:column to the clipboard", async () => {
    await withTempFile("cdd-e2e-hum-", "hum.cs", "a\nb\nc\n", async (file) => {
      const node = occurrenceNode(occurrence(file, 2, 3));
      await vscode.commands.executeCommand("deslop.copyHumanLocation", node);
      const text = await vscode.env.clipboard.readText();
      assert.equal(text, `${file}:2:1`);
    });
  });

  test("deslop.copyClusterLocations copies the cluster header + every row", async () => {
    const entries = [["A.cs", "A\n"], ["B.cs", "B\n"]] as const;
    await withTempFiles("cdd-e2e-cl-", entries, async (file) => {
      const c = cluster("c-e2e-cl", [
        { path: file("A.cs"), start_byte: 0, end_byte: 1 },
        { path: file("B.cs"), start_byte: 0, end_byte: 1 },
      ]);
      await vscode.commands.executeCommand("deslop.copyClusterLocations", clusterNode(c));
      const text = await vscode.env.clipboard.readText();
      const lines = text.split("\n");
      assert.match(lines[0] ?? "", /cluster c-e2e-cl/);
      assert.equal(lines.length, 3);
      assert.ok(!text.includes(".."), "human copy must not include byte ranges");
    });
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
    await withTempFile("cdd-e2e-snip-", "snip.py", "print('hi')\n", async (file) => {
      await vscode.commands.executeCommand(
        "deslop.copySourceSnippet",
        occurrenceNode(occurrence(file, 0, 11)),
      );
      const text = await vscode.env.clipboard.readText();
      assert.match(text, /```python\nprint\('hi'\)/);
      assert.ok(text.endsWith("```"));
    });
  });

  test("deslop.revealOccurrenceInExplorer resolves a workspace path without throwing", async () => {
    await withTempFile("cdd-e2e-rev-", "rev.cs", "x\n", async (file) => {
      await vscode.commands.executeCommand(
        "deslop.revealOccurrenceInExplorer",
        occurrenceNode(occurrence(file, 0, 1)),
      );
    });
  });

  test("deslop.openOccurrence accepts an occurrence tree row", async () => {
    await withTempFile("cdd-e2e-go-", "go.cs", "zero\none\ntwo\n", async (file) => {
      await closingEditors(async () => {
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
      });
    });
  });

  test("deslop.openCanonicalFile opens the cluster's first occurrence by line and column", async () => {
    const source = "header\n    canonical call\n";
    const startByte = Buffer.byteLength("header\n    ", "utf8");
    const endByte = startByte + Buffer.byteLength("canonical", "utf8");
    const entries = [
      [CANONICAL_FILE_NAME, source],
      [SIBLING_FILE_NAME, "sibling call\n"],
    ] as const;
    await withTempFiles("cdd-e2e-canon-", entries, async (file) => {
      await closingEditors(async () => {
        const canonical = file(CANONICAL_FILE_NAME);
        const c = cluster("c-e2e-canon", [
          { path: canonical, start_byte: startByte, end_byte: endByte },
          { path: file(SIBLING_FILE_NAME), start_byte: 0, end_byte: 7 },
        ]);
        await vscode.commands.executeCommand("deslop.openCanonicalFile", clusterNode(c));

        const editor = vscode.window.activeTextEditor;
        assert.ok(editor, "canonical command must open an editor");
        assert.equal(editor.document.uri.fsPath, canonical);
        assert.equal(editor.selection.start.line, 1);
        assert.equal(editor.selection.start.character, 4);
        assert.equal(editor.selection.end.character, 13);
      });
    });
  });

  test("deslop.openAllOccurrences opens every occurrence under the threshold", async () => {
    const entries = [
      [ALPHA_FILE_NAME, "// alpha\n"],
      [BETA_FILE_NAME, "// beta\n"],
    ] as const;
    await withTempFiles("cdd-e2e-all-", entries, async (file) => {
      const files = [file(ALPHA_FILE_NAME), file(BETA_FILE_NAME)];
      await vscode.commands.executeCommand(CLOSE_ALL_EDITORS_COMMAND);
      await closingEditors(async () => {
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
        for (const target of files) {
          assert.ok(openPaths.has(target), `expected tab for ${target}`);
        }
      });
    });
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
