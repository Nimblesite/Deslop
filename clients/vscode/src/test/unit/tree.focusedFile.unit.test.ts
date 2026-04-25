// Unit: FocusedFileProvider. Drives getChildren() against a seeded store.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { FocusedFileProvider, StatusTicker } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, report } from "./tree.helpers";

suite("FocusedFileProvider", () => {
  test("renders 'No active editor' when no editor is focused", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("returns [] when no report is loaded yet", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "x",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 0);
  });

  test("returns cluster overlap for the active editor", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "content",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const activePath = editor.document.uri.fsPath;
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("a", 10, activePath), cluster("b", 5, "/other")]),
      0,
    );
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.ok(nodes.length >= 1);
    const kids = provider.getChildren(nodes[0]);
    assert.ok(kids.length >= 1);
  });

  test("returns an empty hint when no clusters match the active file", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "z",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 1, "/does-not-match")]), 0);
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("surfaces a failed lifecycle as an error status row", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: "failed", message: "oh no" });
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    const errorNode = nodes.find(
      (n) => typeof n.contextValue === "string" && n.contextValue === "deslop.status.error",
    );
    assert.ok(errorNode, "focused file panel must show a failed-lifecycle banner");
    assert.match(labelText(errorNode), /Stopped: oh no/);
  });
});
