// E2E: the Duplication panel's file rows must open the file they name
// ([VSIX-METRICS-PANEL], [Deslop#328]). This drives the real live report
// produced by the bundled LSP over the fixture workspace, so the path form
// under test is whatever the engine actually emits — not a hand-written
// fixture that could drift from it.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

import { FileMetricNode, Node } from "../../tree/nodes";
import { MetricsProvider, StatusTicker } from "../../tree/providers";
import { activateExtension, waitFor } from "./helpers";

/** Depth-first search for the first file row beneath `nodes`, so the test
 * does not assume how deeply the fixture corpus nests. */
function findFileRow(provider: MetricsProvider, nodes: Node[]): FileMetricNode | undefined {
  for (const node of nodes) {
    if (node instanceof FileMetricNode) return node;
    const nested = findFileRow(provider, provider.getChildren(node));
    if (nested) return nested;
  }
  return undefined;
}

suite("duplication panel", () => {
  test("a file row opens the source file it names", async () => {
    const api = await activateExtension();
    const store = api.reportStore;
    assert.ok(store, "reportStore must be exposed on ExtensionApi");
    const perFile = await waitFor(() => {
      const rows = store.current.report?.metrics.per_file;
      return rows && rows.length > 0 ? rows : undefined;
    }, 30_000);
    assert.ok(perFile[0], "the fixture corpus must report at least one file metric");

    const provider = new MetricsProvider(store, new StatusTicker());
    const fileNode = findFileRow(provider, provider.getChildren());
    assert.ok(fileNode, "the Duplication panel must render at least one file row");

    const target = fileNode.command?.arguments?.[0] as vscode.Uri | undefined;
    assert.ok(target, "the file row must carry an open target");
    // Fails loudly today: the engine's scan-root-relative path reaches
    // `Uri.file` unresolved, so this rejects with "Unable to resolve
    // nonexistent file" — the panel's "file was not found" editor.
    const opened = await vscode.workspace.openTextDocument(target);
    assert.equal(opened.uri.fsPath, target.fsPath, "the row opens the file it named");
    assert.ok(opened.getText().includes("class"), "and that file is the real C# source");
    assert.equal(
      fileNode.resourceUri?.fsPath,
      target.fsPath,
      "the decoration URI must match the click target",
    );
  });
});
