// Editor decorations per [VSIX-DECORATIONS]: gutter severity bar + 1-pixel underline.
// No background fill, no emoji, no border boxes.

import * as vscode from "vscode";

import { ReportStore } from "../reportStore";
import { indexedSeverity, SEVERITY_COLOR } from "../severity";
import {
  bucketLabels,
  ReportCluster,
  ReportOccurrence,
  resolveBucket,
  Severity,
  visibleOccurrenceCount,
} from "../types/report";

const SEVERITIES: Severity[] = ["worst", "top10", "mid", "faint"];

export class DecorationManager implements vscode.Disposable {
  private readonly byKind: Map<Severity, vscode.TextEditorDecorationType>;
  private readonly disposables: vscode.Disposable[] = [];

  constructor(private readonly store: ReportStore) {
    this.byKind = new Map(SEVERITIES.map((kind) => [kind, createDecoration(kind)]));
    this.disposables.push(
      store.onDidChange(() => this.redrawAll()),
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
    const report = this.store.current.report;
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
        buckets.get(severity)?.push({
          range,
          hoverMessage: hoverFor(cluster),
        });
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

// Decoration hover is visible in the editor, so keep it human-first.
// Taxonomy labels and numeric signal details stay in diagnostic data
// and Copy Context For AI, not in this tooltip.
export function hoverFor(cluster: ReportCluster): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  const labels = bucketLabels(resolveBucket(cluster));
  const count = visibleOccurrenceCount(cluster);
  const openArgs = encodeURIComponent(JSON.stringify([cluster.id]));
  md.appendMarkdown(`**${labels.plainTitle}** × ${count} — ${labels.actionSentence}\n\n`);
  md.appendMarkdown(
    `[Open cluster](command:deslop.openCluster?${openArgs}) · ` +
      `[Compare with canonical](command:deslop.compareWithCanonical?${openArgs})`,
  );
  return md;
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

function sameFile(reportPath: string, editorPath: string): boolean {
  if (reportPath === editorPath) return true;
  return editorPath.endsWith(reportPath) || reportPath.endsWith(editorPath);
}
