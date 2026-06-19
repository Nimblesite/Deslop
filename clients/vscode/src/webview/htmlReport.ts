// A one-shot viewer for the engine-rendered standalone HTML report. The
// LSP renders the document (design-system CSS inlined, no scripts — the
// same artifact the CLI writes to disk); the extension only hosts it in a
// singleton webview tab. Re-invoking the command refreshes the snapshot.
// This deliberately does NOT participate in the Preact signal data flow
// in panels.ts — it is a single set-and-show, not a reactive surface.

import * as vscode from "vscode";

let panel: vscode.WebviewPanel | undefined;

/// Shows `html` in the singleton HTML-report tab, creating it on first use
/// and refreshing the existing tab in place thereafter so repeated renders
/// never spawn duplicate tabs.
export function showHtmlReport(html: string): void {
  const target = panel ?? createHtmlReportPanel();
  panel = target;
  target.webview.html = html;
  target.reveal(vscode.ViewColumn.Active, false);
}

/// Creates the report tab. Scripts stay disabled — the report is pure HTML
/// + inline CSS with no executable content — and the handle is cleared on
/// dispose (guarded so a stale event can't clobber a newer tab).
function createHtmlReportPanel(): vscode.WebviewPanel {
  const created = vscode.window.createWebviewPanel(
    "deslop.htmlReport",
    "Deslop HTML Report",
    vscode.ViewColumn.Active,
    { enableScripts: false, retainContextWhenHidden: true },
  );
  created.onDidDispose(() => {
    if (panel === created) panel = undefined;
  });
  return created;
}
