// [VSIX-STATUS-BAR] — `dedup · N · #1=File.cs:230 · embed=<model>`.
// Fully signal-driven: the effect() in the constructor tracks every
// signal read inside render(), so the bar updates automatically on
// any relevant store change ([VSIX-REACTIVITY]).

import * as vscode from "vscode";
import { signal, effect } from "@preact/signals-core";

import { ReportStore } from "../reportStore";

export class StatusBar implements vscode.Disposable {
  private readonly item: vscode.StatusBarItem;
  private readonly disposables: vscode.Disposable[] = [];
  private readonly _analysing = signal(false);

  constructor(private readonly store: ReportStore) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 50);
    this.item.name = "Deslop";
    this.item.command = "deslop.openWorstCluster";
    this.disposables.push(
      this.item,
      // Tracks store.report and _analysing signals. Runs immediately
      // (initial render) and again on every relevant change.
      { dispose: effect(() => this.render()) },
      vscode.window.onDidChangeActiveTextEditor(() => this.render()),
    );
    this.item.show();
  }

  setAnalysing(on: boolean): void {
    this._analysing.value = on;
  }

  dispose(): void {
    for (const d of this.disposables) d.dispose();
  }

  private render(): void {
    // [VSIX-STATE-DIRTY]: status bar is a surface — show the visible count.
    const report = this.store.current.visibleReport;
    const analysing = this._analysing.value;
    if (!report) {
      this.item.text = "$(sync~spin) dedup analysing";
      this.item.tooltip = "Deslop is warming up";
      return;
    }
    const editorPath = vscode.window.activeTextEditor?.document.uri.fsPath;
    const clustersInFile = editorPath
      ? report.clusters.filter((c) => c.occurrences.some((o) => sameFile(o.path, editorPath)))
      : report.clusters;
    const n = clustersInFile.length;
    const worst = report.clusters[0];
    const worstLabel = worst
      ? ` · #1=${shortPath(worst.occurrences[0]?.path ?? "?")}`
      : "";
    const embed = report.embedding_provenance?.model_id ?? "off";
    const analysingSuffix = analysing ? " (analysing…)" : "";
    this.item.text = `dedup · ${n}${worstLabel} · embed=${embed}${analysingSuffix}`;
    this.item.tooltip = new vscode.MarkdownString(
      `**Deslop**\n\n` +
        `${report.clusters.length} clusters total, ${n} in this file\n\n` +
        `duplication: \`${report.metrics.duplication_percent.toFixed(1)}%\` ` +
        `(${report.metrics.duplicated_loc}/${report.metrics.analysed_loc} LOC)\n\n` +
        `Click to jump to the worst offender.`,
    );
  }
}

export function sameFile(a: string, b: string): boolean {
  if (a === b) return true;
  return a.endsWith(b) || b.endsWith(a);
}

export function shortPath(p: string): string {
  const slash = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return slash >= 0 ? p.slice(slash + 1) : p;
}
