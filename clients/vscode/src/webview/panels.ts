// Extension-side glue for the two Preact webviews. Per [VSIX-STATE]:
// the extension is the only writer of the webview's signals — any change
// to the report here fans out as a postMessage. Zero ad-hoc state.

import * as vscode from "vscode";
import * as path from "node:path";

import { COLOR } from "../design";
import { ReportStore } from "../reportStore";
import { Report, ReportOccurrence } from "../types/report";

type PanelKind = "cluster" | "report";

interface WebviewPanelState {
  panel: vscode.WebviewPanel;
  kind: PanelKind;
  storeSubscription: vscode.Disposable;
}

const activePanels = new Map<string, WebviewPanelState>();

export function openClusterPanel(
  context: vscode.ExtensionContext,
  store: ReportStore,
  clusterId: string,
): void {
  const key = `cluster:${clusterId}`;
  const existing = activePanels.get(key);
  if (existing) {
    existing.panel.reveal(vscode.ViewColumn.Active);
    return;
  }
  const panel = createPanel(context, "cluster", `Deslop: cluster ${clusterId}`);
  const unsub = wirePanel(panel, store, "cluster", (webview) =>
    webview.postMessage({ kind: "select/cluster", id: clusterId }),
  );
  panel.webview.onDidReceiveMessage((msg) => handleMessage(store, msg));
  panel.onDidDispose(() => {
    unsub.dispose();
    activePanels.delete(key);
  });
  activePanels.set(key, { panel, kind: "cluster", storeSubscription: unsub });
}

export function openReportPanel(context: vscode.ExtensionContext, store: ReportStore): void {
  const key = "report";
  const existing = activePanels.get(key);
  if (existing) {
    existing.panel.reveal(vscode.ViewColumn.Active);
    return;
  }
  const panel = createPanel(context, "report", "Deslop: report");
  const unsub = wirePanel(panel, store, "report");
  panel.webview.onDidReceiveMessage((msg) => handleMessage(store, msg));
  panel.onDidDispose(() => {
    unsub.dispose();
    activePanels.delete(key);
  });
  activePanels.set(key, { panel, kind: "report", storeSubscription: unsub });
}

function createPanel(
  context: vscode.ExtensionContext,
  kind: PanelKind,
  title: string,
): vscode.WebviewPanel {
  const mediaRoot = vscode.Uri.file(path.join(context.extensionPath, "media"));
  const panel = vscode.window.createWebviewPanel(
    `codededup.${kind}`,
    title,
    vscode.ViewColumn.Active,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: [mediaRoot],
    },
  );
  panel.webview.html = buildHtml(panel.webview, context, kind);
  return panel;
}

function wirePanel(
  panel: vscode.WebviewPanel,
  store: ReportStore,
  kind: PanelKind,
  onReady?: (webview: vscode.Webview) => void,
): vscode.Disposable {
  void kind;
  const push = (report: Report | null) => {
    if (!report) return;
    panel.webview.postMessage({ kind: "report/snapshot", report });
  };
  push(store.current.report);
  const sub = store.onDidChange((state) => push(state.report));
  if (onReady) {
    // delay until the webview has mounted and acknowledged via `ready`
    const once = panel.webview.onDidReceiveMessage((m: { kind?: string }) => {
      if (m?.kind === "ready") {
        onReady(panel.webview);
        once.dispose();
      }
    });
    panel.onDidDispose(() => once.dispose());
  }
  return sub;
}

function buildHtml(
  webview: vscode.Webview,
  context: vscode.ExtensionContext,
  kind: PanelKind,
): string {
  const scriptPath = vscode.Uri.file(
    path.join(context.extensionPath, "media", "webview", `${kind}.js`),
  );
  const scriptUri = webview.asWebviewUri(scriptPath);
  const csp = [
    `default-src 'none'`,
    `style-src ${webview.cspSource} 'unsafe-inline'`,
    `script-src ${webview.cspSource}`,
    `font-src ${webview.cspSource}`,
    `img-src ${webview.cspSource} data:`,
  ].join("; ");
  return /* html */ `<!doctype html>
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta http-equiv="Content-Security-Policy" content="${csp}" />
        <title>Deslop</title>
        <style>body { background: ${COLOR.surface}; margin: 0; }</style>
      </head>
      <body>
        <div id="root"></div>
        <script type="module" src="${scriptUri}"></script>
      </body>
    </html>`;
}

export async function handleMessage(store: ReportStore, message: unknown): Promise<void> {
  if (!message || typeof message !== "object") return;
  const m = message as { kind?: string } & Record<string, unknown>;
  switch (m.kind) {
    case "open/cluster": {
      const id = typeof m["id"] === "string" ? (m["id"] as string) : null;
      if (id) await vscode.commands.executeCommand("codededup.openCluster", id);
      return;
    }
    case "open/occurrence": {
      const occurrence = m["occurrence"] as ReportOccurrence | undefined;
      if (occurrence) await vscode.commands.executeCommand("codededup.openOccurrence", occurrence);
      return;
    }
    case "compare/canonical": {
      const id = typeof m["clusterId"] === "string" ? (m["clusterId"] as string) : null;
      if (id) await vscode.commands.executeCommand("codededup.compareWithCanonical", id);
      return;
    }
    case "refresh":
      await vscode.commands.executeCommand("codededup.refreshReport");
      return;
    case "navigate/next":
    case "navigate/prev": {
      const clusters = store.current.report?.clusters ?? [];
      if (clusters.length === 0) return;
      // We don't know which cluster is active in the webview from the host side —
      // the webview already advances its own signal; no-op is acceptable for now.
      return;
    }
  }
}
