// Editor decorations per [VSIX-DECORATIONS]: gutter severity bar + 1-pixel underline.
// No background fill, no emoji, no border boxes.

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";

import { clusterHoverMarkdown } from "../clusterHover";
import { ReportStore } from "../reportStore";
import { indexedSeverity, SEVERITY_COLOR } from "../severity";
import { ReportCluster, ReportOccurrence, Severity, visibleOccurrenceCount } from "../types/report";

const SEVERITIES: Severity[] = ["worst", "top10", "mid", "faint"];

export class DecorationManager implements vscode.Disposable {
  private readonly byKind: Map<Severity, vscode.TextEditorDecorationType>;
  private readonly disposables: vscode.Disposable[] = [];

  constructor(private readonly store: ReportStore) {
    this.byKind = new Map(SEVERITIES.map((kind) => [kind, createDecoration(kind)]));
    this.disposables.push(
      // effect() tracks store.report via the redraw() read — rerenders only
      // when the report signal changes, not on every store mutation.
      { dispose: effect(() => this.redrawAll()) },
      vscode.window.onDidChangeVisibleTextEditors(() => this.redrawAll()),
      vscode.workspace.onDidChangeTextDocument(() => this.redrawAll()),
    );
  }

  dispose(): void {
    for (const d of this.disposables) d.dispose();
    for (const dt of this.byKind.values()) dt.dispose();
  }

  private redrawAll(): void {
    for (const editor of vscode.window.visibleTextEditors) this.redraw(editor);
  }

  private redraw(editor: vscode.TextEditor): void {
    // [VSIX-STATE-DIRTY]: render from the visible projection so decorations
    // disappear immediately when the user types into a duplicated file.
    const report = this.store.current.visibleReport;
    if (!report) {
      this.clear(editor);
      return;
    }
    const severities = indexedSeverity(report.clusters);
    const buckets = new Map<Severity, vscode.DecorationOptions[]>(
      SEVERITIES.map((kind) => [kind, []]),
    );
    const activePath = editor.document.uri.fsPath;
    for (const cluster of report.clusters) {
      const severity = severities.get(cluster.id) ?? "faint";
      for (const occurrence of cluster.occurrences) {
        if (!sameFile(occurrence.path, activePath)) continue;
        const range = byteRangeToRange(editor.document, occurrence);
        if (!range) continue;
        buckets.get(severity)?.push({ range });
      }
    }
    for (const [kind, decoration] of this.byKind) {
      editor.setDecorations(decoration, buckets.get(kind) ?? []);
    }
  }

  private clear(editor: vscode.TextEditor): void {
    for (const decoration of this.byKind.values()) editor.setDecorations(decoration, []);
  }
}

function createDecoration(severity: Severity): vscode.TextEditorDecorationType {
  const color = SEVERITY_COLOR[severity];
  return vscode.window.createTextEditorDecorationType({
    textDecoration: `underline ${color}`,
    overviewRulerColor: color,
    overviewRulerLane: vscode.OverviewRulerLane.Left,
    isWholeLine: false,
    rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
  });
}

// Kept for test harness — ClusterHoverProvider uses clusterHoverMarkdown directly.
// Uses visibleOccurrenceCount so the count reflects what the user can act on.
export function hoverFor(cluster: ReportCluster): vscode.MarkdownString {
  return clusterHoverMarkdown(cluster, { count: visibleOccurrenceCount(cluster) });
}

export function byteRangeToRange(
  document: vscode.TextDocument,
  occurrence: ReportOccurrence,
): vscode.Range | null {
  const text = document.getText();
  const buffer = Buffer.from(text, "utf8");
  if (occurrence.start_byte > buffer.length || occurrence.end_byte > buffer.length) return null;
  const startText = buffer.slice(0, occurrence.start_byte).toString("utf8");
  const endText = buffer.slice(0, occurrence.end_byte).toString("utf8");
  const start = document.positionAt(startText.length);
  const end = document.positionAt(endText.length);
  return new vscode.Range(start, end);
}

export function sameFile(reportPath: string, editorPath: string): boolean {
  if (reportPath === editorPath) return true;
  return editorPath.endsWith(reportPath) || reportPath.endsWith(editorPath);
}
