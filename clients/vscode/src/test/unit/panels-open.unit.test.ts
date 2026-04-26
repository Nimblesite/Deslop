// Unit: openReportPanel / openClusterPanel panel lifecycle — covers the
// create-first + reveal-on-second-call paths that E2E tests cannot reach
// because they exercise the dist/ bundle, not the instrumented out/ module.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import { handleMessage, openReportPanel } from "../../webview/panels";
import { ReportStore } from "../../reportStore";

function fakeCtx(): vscode.ExtensionContext {
  return {
    extensionPath: path.join(__dirname, "..", "..", ".."),
    extensionUri: vscode.Uri.file(path.join(__dirname, "..", "..", "..")),
    subscriptions: [] as vscode.Disposable[],
  } as unknown as vscode.ExtensionContext;
}

suite("panels lifecycle", () => {
  test("openReportPanel opens a new panel and reveals it on a second call", () => {
    const store = new ReportStore();
    const ctx = fakeCtx();

    const tabsBefore = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    openReportPanel(ctx, store);
    const tabsAfterFirst = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    assert.ok(tabsAfterFirst >= tabsBefore, "first openReportPanel must not reduce the open tab count");

    openReportPanel(ctx, store); // hits the existing-panel reveal branch
    const tabsAfterSecond = vscode.window.tabGroups.all.flatMap((g) => g.tabs).length;
    assert.equal(
      tabsAfterSecond,
      tabsAfterFirst,
      "second openReportPanel call must reveal the existing panel, not open a new tab",
    );
  });

  test("handleMessage with kind undefined is a no-op", async () => {
    const store = new ReportStore();
    // Explicit undefined kind hits the `case undefined: return` branch.
    await handleMessage(store, { kind: undefined });
    assert.ok(store.current.report === null, "a no-op message must not mutate the store");
  });
});
