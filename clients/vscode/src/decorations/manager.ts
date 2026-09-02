// Editor decorations per [VSIX-DECORATIONS]: gutter severity bar + 1-pixel underline.
// No background fill, no emoji, no border boxes.
//
// [VSIX-PERF] Decorations are driven by the analysis report and editor visibility
// ONLY — never by raw text edits. Deslop reacts to FILE changes (the LSP's file
// watcher), so decorations repaint when a fresh report lands or when an editor
// becomes visible, not on every keystroke. Repaints are coalesced through a
// trailing debounce, and each editor's byte→UTF-16 buffer is built once per
// redraw instead of once per occurrence.
//
// [SEVERITY-COLOR] The underline and ruler stripe are pure colour — there is no
// glyph on an underline — so they carry the bucket channel and nothing else. A
// decoration therefore needs no ranking at all: an occurrence is coloured by
// what kind of duplicate it is, which is the same answer wherever it sorts.

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";

import { clusterHoverMarkdown } from "../clusterHover";
import { ReportStore } from "../reportStore";
import { clusterSeverity, DESLOP_SEVERITIES, DeslopSeverity, DESLOP_SEVERITY_COLOR } from "../severity";
import { sameFile } from "../pathUtils";
import { debounce, Debounced, ScheduleFn } from "../util/debounce";
import {
  Report,
  ReportCluster,
  ReportOccurrence,
  occurrenceCount,
} from "../types/report";

const REDRAW_DEBOUNCE_MS = 60;
const UTF8_ENCODING = "utf8";

export class DecorationManager implements vscode.Disposable {
  private readonly byKind: Map<DeslopSeverity, vscode.TextEditorDecorationType>;
  private readonly disposables: vscode.Disposable[] = [];
  private readonly scheduleRedraw: Debounced;

  constructor(private readonly store: ReportStore, schedule?: ScheduleFn) {
    this.byKind = new Map(DESLOP_SEVERITIES.map((kind) => [kind, createDecoration(kind)]));
    this.scheduleRedraw = debounce(() => this.flush(), REDRAW_DEBOUNCE_MS, schedule);
    this.disposables.push(
      // Repaint when the report changes (a file-change-driven analysis update — an
      // unsaved edit reaches this via the dirty projection on visibleReport) or when
      // the set of visible editors changes. Deliberately NOT subscribed to
      // onDidChangeTextDocument: decorations do no work per keystroke.
      // [VSIX-REACTIVITY-DECORATIONS] Signal-driven: the effect re-runs when
      // the report changes; a removed cluster's decorations drop on diff.
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
    for (const editor of vscode.window.visibleTextEditors) this.redraw(editor, report);
  }

  // [VSIX-STATE-DIRTY]: render from the visible projection so decorations vanish
  // immediately when the user edits a duplicated file (its occurrences are elided
  // from the projection). The document buffer is built lazily and exactly once —
  // only when this editor actually owns an occurrence.
  private redraw(editor: vscode.TextEditor, report: Report): void {
    const activePath = editor.document.uri.fsPath;
    const buckets = new Map<DeslopSeverity, vscode.DecorationOptions[]>(
      DESLOP_SEVERITIES.map((kind) => [kind, []]),
    );
    let buffer: Buffer | undefined;
    for (const cluster of report.clusters) {
      const severity = clusterSeverity(cluster);
      for (const occurrence of cluster.occurrences) {
        if (!sameFile(occurrence.path, activePath)) continue;
        buffer ??= Buffer.from(editor.document.getText(), UTF8_ENCODING);
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

function createDecoration(severity: DeslopSeverity): vscode.TextEditorDecorationType {
  const color = DESLOP_SEVERITY_COLOR[severity];
  return vscode.window.createTextEditorDecorationType({
    textDecoration: `underline ${color}`,
    overviewRulerColor: color,
    overviewRulerLane: vscode.OverviewRulerLane.Left,
    isWholeLine: false,
    rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
  });
}

// Kept for test harness — ClusterHoverProvider uses clusterHoverMarkdown directly.
// The count is the engine's `occurrence_count`, the same number every
// other surface shows.
export function hoverFor(cluster: ReportCluster): vscode.MarkdownString {
  return clusterHoverMarkdown(cluster, { count: occurrenceCount(cluster) });
}

// Maps a UTF-8 byte range to an editor range, allocating the document buffer per
// call. Hot paths use rangeFromBuffer with a hoisted buffer instead ([VSIX-PERF]).
export function byteRangeToRange(
  document: vscode.TextDocument,
  occurrence: ReportOccurrence,
): vscode.Range | null {
  return rangeFromBuffer(document, Buffer.from(document.getText(), UTF8_ENCODING), occurrence);
}

// Converts a UTF-8 byte range to an editor range using a buffer the caller already
// built for this document, so a redraw pays the whole-document encode cost once.
export function rangeFromBuffer(
  document: vscode.TextDocument,
  buffer: Buffer,
  occurrence: ReportOccurrence,
): vscode.Range | null {
  if (occurrence.start_byte > buffer.length || occurrence.end_byte > buffer.length) return null;
  const start = document.positionAt(buffer.slice(0, occurrence.start_byte).toString(UTF8_ENCODING).length);
  const end = document.positionAt(buffer.slice(0, occurrence.end_byte).toString(UTF8_ENCODING).length);
  return new vscode.Range(start, end);
}

export { sameFile } from "../pathUtils";
