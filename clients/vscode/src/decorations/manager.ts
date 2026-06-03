// Editor decorations per [VSIX-DECORATIONS]: gutter severity bar + 1-pixel underline.
// No background fill, no emoji, no border boxes.
//
// [VSIX-PERF] Redraws are coalesced through a trailing debounce and target only the
// editors that actually changed, so a keystroke burst collapses into a single pass
// instead of re-decorating every visible editor on every keypress. The document's
// byte→UTF-16 buffer is built once per editor-redraw, never once per occurrence, and
// the severity ranking is memoised per report.

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
  private readonly dirtyPaths = new Set<string>();
  private allDirty = false;
  private severityCache: SeverityCache | undefined;
  private readonly flushDecorations: Debounced;

  constructor(private readonly store: ReportStore, schedule?: ScheduleFn) {
    this.byKind = new Map(SEVERITIES.map((kind) => [kind, createDecoration(kind)]));
    this.flushDecorations = debounce(() => this.flush(), REDRAW_DEBOUNCE_MS, schedule);
    this.disposables.push(
      // Tracks only visibleReport: a report change re-ranks every cluster and so
      // re-decorates all editors; lifecycle/embedding ticks are ignored.
      { dispose: effect(() => { void this.store.visibleReport.value; this.scheduleAll(); }) },
      vscode.window.onDidChangeVisibleTextEditors(() => this.scheduleAll()),
      vscode.workspace.onDidChangeTextDocument((event) => this.scheduleDocument(event.document)),
      { dispose: () => this.flushDecorations.cancel() },
    );
  }

  dispose(): void {
    for (const d of this.disposables) d.dispose();
    for (const dt of this.byKind.values()) dt.dispose();
  }

  // A report or visible-editor change can re-rank every cluster, so all editors
  // must be re-decorated on the next flush.
  private scheduleAll(): void {
    this.allDirty = true;
    this.flushDecorations();
  }

  // A text edit only affects decorations in that document's editors.
  private scheduleDocument(document: vscode.TextDocument): void {
    this.dirtyPaths.add(document.uri.fsPath);
    this.flushDecorations();
  }

  private flush(): void {
    const editors = this.targetEditors();
    this.allDirty = false;
    this.dirtyPaths.clear();
    const report = this.store.visibleReport.value;
    if (!report) {
      for (const editor of editors) this.clear(editor);
      return;
    }
    const severities = this.severitiesFor(report);
    for (const editor of editors) this.redraw(editor, report, severities);
  }

  private targetEditors(): readonly vscode.TextEditor[] {
    const visible = vscode.window.visibleTextEditors;
    if (this.allDirty) return visible;
    return visible.filter((editor) => this.dirtyPaths.has(editor.document.uri.fsPath));
  }

  private severitiesFor(report: Report): Map<string, Severity> {
    if (this.severityCache?.report === report) return this.severityCache.severities;
    const severities = indexedSeverity(report.clusters);
    this.severityCache = { report, severities };
    return severities;
  }

  // [VSIX-STATE-DIRTY]: render from the visible projection so decorations vanish
  // immediately when the user types into a duplicated file. The document buffer is
  // built lazily and exactly once — only when this editor actually owns an occurrence.
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
