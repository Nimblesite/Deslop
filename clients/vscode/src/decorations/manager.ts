// Editor decorations per [VSIX-DECORATIONS]: gutter severity bar + 1-pixel underline.
// No background fill, no emoji, no border boxes.
//
// [VSIX-PERF] Decorations are driven by the analysis report and editor visibility
// ONLY — never by raw text edits. Deslop reacts to FILE changes (the LSP's file
// watcher), so decorations repaint when a fresh report lands or when an editor
// becomes visible, not on every keystroke. Repaints are coalesced through a
// trailing debounce, the severity ranking is memoised per report, and each
// editor's byte→UTF-16 buffer is built once per redraw instead of once per
// occurrence.

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";

import { clusterHoverMarkdown } from "../clusterHover";
import { ReportStore } from "../reportStore";
import { indexedSeverity, SEVERITY_COLOR } from "../severity";
import { debounce, Debounced, ScheduleFn } from "../util/debounce";
import { Report, ReportCluster, ReportOccurrence, Severity, visibleOccurrenceCount } from "../types/report";

const SEVERITIES: Severity[] = ["worst", "top10", "mid", "faint"];
const REDRAW_DEBOUNCE_MS = 60;

interface SeverityCache {
  report: Report;
  severities: Map<string, Severity>;
}

export class DecorationManager implements vscode.Disposable {
  private readonly byKind: Map<Severity, vscode.TextEditorDecorationType>;
  private readonly disposables: vscode.Disposable[] = [];
  private severityCache: SeverityCache | undefined;
  private readonly scheduleRedraw: Debounced;

  constructor(private readonly store: ReportStore, schedule?: ScheduleFn) {
    this.byKind = new Map(SEVERITIES.map((kind) => [kind, createDecoration(kind)]));
    this.scheduleRedraw = debounce(() => this.flush(), REDRAW_DEBOUNCE_MS, schedule);
    this.disposables.push(
      // Repaint when the report changes (a file-change-driven analysis update — an
      // unsaved edit reaches this via the dirty projection on visibleReport) or when
      // the set of visible editors changes. Deliberately NOT subscribed to
      // onDidChangeTextDocument: decorations do no work per keystroke.
      { dispose: effect(() => { void this.store.visibleReport.value; this.scheduleRedraw(); }) },
      vscode.window.onDidChangeVisibleTextEditors(() => this.scheduleRedraw()),
      { dispose: () => this.scheduleRedraw.cancel() },
    );
  }

  dispose(): void {
    for (const d of this.disposables) d.dispose();
    for (const dt of this.byKind.values()) dt.dispose();
  }

  private flush(): void {
    const report = this.store.visibleReport.value;
    if (!report) {
      for (const editor of vscode.window.visibleTextEditors) this.clear(editor);
      return;
    }
    const severities = this.severitiesFor(report);
    for (const editor of vscode.window.visibleTextEditors) this.redraw(editor, report, severities);
  }

  private severitiesFor(report: Report): Map<string, Severity> {
    if (this.severityCache?.report === report) return this.severityCache.severities;
    const severities = indexedSeverity(report.clusters);
    this.severityCache = { report, severities };
    return severities;
  }

  // [VSIX-STATE-DIRTY]: render from the visible projection so decorations vanish
  // immediately when the user edits a duplicated file (its occurrences are elided
  // from the projection). The document buffer is built lazily and exactly once —
  // only when this editor actually owns an occurrence.
  private redraw(editor: vscode.TextEditor, report: Report, severities: Map<string, Severity>): void {
    const activePath = editor.document.uri.fsPath;
    const buckets = new Map<Severity, vscode.DecorationOptions[]>(SEVERITIES.map((kind) => [kind, []]));
    let buffer: Buffer | undefined;
    for (const cluster of report.clusters) {
      const severity = severities.get(cluster.id) ?? "faint";
      for (const occurrence of cluster.occurrences) {
        if (!sameFile(occurrence.path, activePath)) continue;
        buffer ??= Buffer.from(editor.document.getText(), "utf8");
        const range = rangeFromBuffer(editor.document, buffer, occurrence);
        if (range) buckets.get(severity)?.push({ range });
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

// Maps a UTF-8 byte range to an editor range, allocating the document buffer per
// call. Hot paths use rangeFromBuffer with a hoisted buffer instead ([VSIX-PERF]).
export function byteRangeToRange(
  document: vscode.TextDocument,
  occurrence: ReportOccurrence,
): vscode.Range | null {
  return rangeFromBuffer(document, Buffer.from(document.getText(), "utf8"), occurrence);
}

// Converts a UTF-8 byte range to an editor range using a buffer the caller already
// built for this document, so a redraw pays the whole-document encode cost once.
export function rangeFromBuffer(
  document: vscode.TextDocument,
  buffer: Buffer,
  occurrence: ReportOccurrence,
): vscode.Range | null {
  if (occurrence.start_byte > buffer.length || occurrence.end_byte > buffer.length) return null;
  const start = document.positionAt(buffer.slice(0, occurrence.start_byte).toString("utf8").length);
  const end = document.positionAt(buffer.slice(0, occurrence.end_byte).toString("utf8").length);
  return new vscode.Range(start, end);
}

export function sameFile(reportPath: string, editorPath: string): boolean {
  if (reportPath === editorPath) return true;
  return editorPath.endsWith(reportPath) || reportPath.endsWith(editorPath);
}
