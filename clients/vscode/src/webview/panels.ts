// Extension-side glue for the two Preact webviews. Per [VSIX-STATE]:
// the extension is the only writer of the webview's signals — any change
// to the report here fans out as a postMessage. Zero ad-hoc state.

import * as vscode from "vscode";
import * as path from "node:path";
import { effect, untracked } from "@preact/signals-core";

import { COLOR } from "../design";
import { reportWithDisplayLocations } from "../locations";
import { logWarn } from "../logging";
import { ReportStore } from "../reportStore";
import { anchorForClusterId, ClusterAnchor, clusterPanelFeed } from "../clusterSelection";
import { Report, ReportOccurrence } from "../types/report";

type PanelKind = "cluster" | "report" | "duplication";

const CLUSTER_PANEL_KIND: PanelKind = "cluster";
const REPORT_PANEL_KIND: PanelKind = "report";
const DUPLICATION_PANEL_KIND: PanelKind = "duplication";

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
  if (existing) return existing.panel.reveal(vscode.ViewColumn.Active);
  const anchor = anchorForClusterId(store.current.report, clusterId);
  const panel = createPanel(context, CLUSTER_PANEL_KIND, `Deslop: cluster ${clusterId}`);
  const unsub = wirePanel(panel, store, CLUSTER_PANEL_KIND, { anchor });
  wireMessages(panel, store);
  panel.onDidDispose(() => {
    unsub.dispose();
    activePanels.delete(key);
  });
  activePanels.set(key, { panel, kind: CLUSTER_PANEL_KIND, storeSubscription: unsub });
}

// [VSIX-REPORT-WEBVIEW] Full report webview — the host pushes report
// snapshots; the webview stays dumb and renders from store signals.
export function openReportPanel(context: vscode.ExtensionContext, store: ReportStore): void {
  const key = "report";
  const existing = activePanels.get(key);
  if (existing) return existing.panel.reveal(vscode.ViewColumn.Active);
  const panel = createPanel(context, REPORT_PANEL_KIND, "Deslop: report");
  const unsub = wirePanel(panel, store, REPORT_PANEL_KIND);
  wireMessages(panel, store);
  panel.onDidDispose(() => {
    unsub.dispose();
    activePanels.delete(key);
  });
  activePanels.set(key, { panel, kind: REPORT_PANEL_KIND, storeSubscription: unsub });
}

// [VSIX-METRICS-REPORT] Duplication report — the headline of the
// Duplication panel opens this. Reuses the report-snapshot push so the
// webview renders the per-folder/per-file breakdown from the same data.
export function openDuplicationReportPanel(
  context: vscode.ExtensionContext,
  store: ReportStore,
): void {
  const key = "duplication";
  const existing = activePanels.get(key);
  if (existing) return existing.panel.reveal(vscode.ViewColumn.Active);
  const panel = createPanel(context, DUPLICATION_PANEL_KIND, "Deslop: Duplication");
  const unsub = wirePanel(panel, store, DUPLICATION_PANEL_KIND);
  wireMessages(panel, store);
  panel.onDidDispose(() => {
    unsub.dispose();
    activePanels.delete(key);
  });
  activePanels.set(key, { panel, kind: DUPLICATION_PANEL_KIND, storeSubscription: unsub });
}

function createPanel(
  context: vscode.ExtensionContext,
  kind: PanelKind,
  title: string,
): vscode.WebviewPanel {
  const mediaRoot = vscode.Uri.file(path.join(context.extensionPath, "media"));
  const panel = vscode.window.createWebviewPanel(
    `deslop.${kind}`,
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

function wireMessages(panel: vscode.WebviewPanel, store: ReportStore): void {
  panel.webview.onDidReceiveMessage((msg: unknown) => {
    handleMessage(store, msg).catch((err: unknown) => console.error("handleMessage failed", err));
  });
}

interface ClusterSelection {
  readonly anchor: ClusterAnchor;
}

function wirePanel(
  panel: vscode.WebviewPanel,
  store: ReportStore,
  kind: PanelKind,
  selection?: ClusterSelection,
): vscode.Disposable {
  void kind;
  const pushReport = (report: Report | null): void => {
    if (!report) return;
    void panel.webview.postMessage({
      kind: "report/snapshot",
      report: reportWithDisplayLocations(report),
    });
  };
  if (selection) return wireClusterFeed(panel, store, selection.anchor, pushReport);
  // [VSIX-STATE-DIRTY]: webviews mirror the visible projection so an
  // unsaved edit hides occurrences in lock-step with the tree. Commands
  // that need cluster-id lookup go through canonical separately.
  // [FACET-TOP-OFFENDERS-FILTER] The workspace facet filter rides along
  // so the webview's cluster list agrees with the filtered tree.
  return {
    dispose: effect(() => {
      void panel.webview.postMessage({
        kind: "facetFilter/set",
        filter: store.facetFilter.value,
      });
      pushReport(store.visibleReport.value);
    }),
  };
}

// The cluster detail panel pins to a stable anchor instead of the volatile
// content-hash id, re-resolving the live cluster on every generation so the
// selection survives id churn and dirty elision (#173). Delays the first push
// until the webview has mounted and acknowledged via `ready`.
function wireClusterFeed(
  panel: vscode.WebviewPanel,
  store: ReportStore,
  anchor: ClusterAnchor,
  pushReport: (report: Report | null) => void,
): vscode.Disposable {
  let ready = false;
  const push = (): void => {
    if (ready) pushClusterFeed(panel, store, anchor, pushReport);
  };
  // [VSIX-PERF] Track only the report signals. The explicit reads keep the
  // subscription alive while we wait for `ready`, and they exclude lifecycle /
  // embedding-progress / pending-model ticks so those no longer re-push the feed.
  const sub = { dispose: effect(() => {
    void store.report.value;
    void store.visibleReport.value;
    push();
  }) };
  const once = panel.webview.onDidReceiveMessage((m: { kind?: string }) => {
    if (m.kind !== "ready") return;
    ready = true;
    push();
    once.dispose();
  });
  panel.onDidDispose(() => {
    once.dispose();
  });
  return sub;
}

// Pushes the snapshot + selection so the opened cluster is always resolvable,
// logging when it has genuinely left the report so the failure is never silent.
function pushClusterFeed(
  panel: vscode.WebviewPanel,
  store: ReportStore,
  anchor: ClusterAnchor,
  pushReport: (report: Report | null) => void,
): void {
  const canonical = store.report.value;
  const visible = store.visibleReport.value;
  if (!canonical || !visible) return;
  const feed = clusterPanelFeed(canonical, visible, anchor);
  pushReport(feed.report);
  void panel.webview.postMessage({ kind: "select/cluster", id: feed.selectedId });
  if (feed.selectedId === null) {
    logWarn("cluster no longer exists", {
      clusterId: anchor.id,
      // Diagnostic only — read untracked so it doesn't widen the effect's deps.
      generation: untracked(() => store.current.generation),
    });
  }
}

function buildHtml(
  webview: vscode.Webview,
  context: vscode.ExtensionContext,
  kind: PanelKind,
): string {
  const scriptPath = vscode.Uri.file(
    path.join(context.extensionPath, "media", "webview", `${kind}.js`),
  );
  const scriptUri = webview.asWebviewUri(scriptPath).toString();
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

export async function handleMessage(_store: ReportStore, message: unknown): Promise<void> {
  if (!message || typeof message !== "object") return;
  const m = message as { kind?: string } & Record<string, unknown>;
  switch (m.kind) {
    case "open/cluster": {
      const id = typeof m["id"] === "string" ? m["id"] : null;
      if (id) await vscode.commands.executeCommand("deslop.openCluster", id);
      return;
    }
    case "open/occurrence": {
      const occurrence = m["occurrence"] as ReportOccurrence | undefined;
      if (occurrence) await vscode.commands.executeCommand("deslop.openOccurrence", occurrence);
      return;
    }
    case "compare/canonical": {
      const id = typeof m["clusterId"] === "string" ? m["clusterId"] : null;
      if (id) await vscode.commands.executeCommand("deslop.compareWithCanonical", id);
      return;
    }
    case "refresh":
      await vscode.commands.executeCommand("deslop.refreshReport");
      return;
    case undefined:
      return;
    default:
      return;
  }
}
