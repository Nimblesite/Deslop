// Live duplication bubble — [VSIX-LIVE-BUBBLE].
// Fires after every coalesced buffer edit. Calls codededup/duplicatesFindSimilar
// on the most-recently-touched range; if fused >= 0.85, renders:
//   primary: after-text decoration (severity dot + verdict + count + canonical)
//   secondary: inlay hint with a 3-bar signal strip
// Ghost-line mode uses a CodeLens on a phantom line.

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { COLOR, FONT, SEVERITY_COLOR, SEVERITY_DOT } from "../design";
import { ReportStore } from "../reportStore";
import { indexedSeverity } from "../severity";
import {
  FUSED_THRESHOLD,
  ReportCluster,
  Severity,
  verdictOf,
} from "../types/report";

const DEBOUNCE_MS = 250;
const BUDGET_MS = 250;

interface ActiveBubble {
  editor: vscode.TextEditor;
  clusterId: string;
  range: vscode.Range;
}

export class LiveBubble implements vscode.Disposable {
  private readonly bubbleDecoration: vscode.TextEditorDecorationType;
  private readonly ghostDecoration: vscode.TextEditorDecorationType;
  private readonly inlayProvider: BubbleInlayProvider;
  private readonly disposables: vscode.Disposable[] = [];
  private active: ActiveBubble | null = null;
  private dismissedClusters = new Set<string>();
  private debounceTimer: NodeJS.Timeout | undefined;

  constructor(
    private readonly store: ReportStore,
    private readonly clientOf: () => LanguageClient | undefined,
  ) {
    this.bubbleDecoration = vscode.window.createTextEditorDecorationType({
      after: {
        margin: "0 0 0 12px",
        color: COLOR.onSurface,
      },
    });
    this.ghostDecoration = vscode.window.createTextEditorDecorationType({
      isWholeLine: true,
      after: {
        margin: "0 0 0 0",
        fontStyle: "italic",
        color: COLOR.onSurfaceMuted,
      },
    });
    this.inlayProvider = new BubbleInlayProvider();

    this.disposables.push(
      this.bubbleDecoration,
      this.ghostDecoration,
      vscode.languages.registerInlayHintsProvider(
        [{ language: "csharp" }, { language: "rust" }, { language: "python" }],
        this.inlayProvider,
      ),
      vscode.workspace.onDidChangeTextDocument((e) => this.onEdit(e)),
      vscode.window.onDidChangeActiveTextEditor(() => this.clearBubble()),
      vscode.commands.registerCommand("codededup.bubble.dismiss", () => this.clearBubble()),
      vscode.commands.registerCommand(
        "codededup.bubble.dismissCluster",
        (id: string) => {
          this.dismissedClusters.add(id);
          this.clearBubble();
        },
      ),
    );
  }

  dispose(): void {
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    for (const d of this.disposables) d.dispose();
  }

  private onEdit(event: vscode.TextDocumentChangeEvent): void {
    const cfg = vscode.workspace.getConfiguration("codededup");
    if (!cfg.get<boolean>("liveBubble.enabled", true)) return;
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document !== event.document) return;
    if (event.contentChanges.length === 0) return;
    const lastChange = event.contentChanges[event.contentChanges.length - 1];
    if (!lastChange) return;

    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.debounceTimer = setTimeout(() => {
      this.probe(editor, lastChange).catch(() => this.clearBubble());
    }, DEBOUNCE_MS);
  }

  private async probe(
    editor: vscode.TextEditor,
    change: vscode.TextDocumentContentChangeEvent,
  ): Promise<void> {
    const client = this.clientOf();
    if (!client) return;
    const doc = editor.document;
    const start = change.range.start;
    const endOffset = doc.offsetAt(start) + change.text.length;
    const end = doc.positionAt(endOffset);
    const range = new vscode.Range(start, end);
    const startByte = utf8ByteOffset(doc, start);
    const endByte = utf8ByteOffset(doc, end);

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), BUDGET_MS);
    try {
      const clusters = await client.sendRequest<ReportCluster[]>(
        "codededup/duplicatesFindSimilar",
        { path: doc.uri.fsPath, start_byte: startByte, end_byte: endByte },
      );
      clearTimeout(timeout);
      this.render(editor, range, clusters);
    } catch {
      clearTimeout(timeout);
      this.clearBubble();
    }
  }

  private render(
    editor: vscode.TextEditor,
    range: vscode.Range,
    clusters: ReportCluster[],
  ): void {
    const report = this.store.current.report;
    if (!report) return;
    const best = clusters
      .filter((c) => c.signals.fused >= FUSED_THRESHOLD)
      .filter((c) => !this.dismissedClusters.has(c.id))
      .sort((a, b) => b.weight - a.weight)[0];
    if (!best) {
      this.clearBubble();
      return;
    }
    if (this.active?.clusterId === best.id && this.active.range.isEqual(range)) return;

    const severities = indexedSeverity(report.clusters);
    const severity = severities.get(best.id) ?? "faint";
    const mode = vscode.workspace
      .getConfiguration("codededup")
      .get<string>("liveBubble.mode", "inline");
    const lineEnd = editor.document.lineAt(range.end.line).range.end;
    const anchor = new vscode.Range(lineEnd, lineEnd);

    if (mode === "ghost") {
      this.bubbleDecoration.dispose;
      editor.setDecorations(this.bubbleDecoration, []);
      editor.setDecorations(this.ghostDecoration, [
        {
          range: editor.document.lineAt(range.end.line).range,
          renderOptions: { after: { contentText: ghostText(best, severity) } },
        },
      ]);
    } else {
      editor.setDecorations(this.ghostDecoration, []);
      editor.setDecorations(this.bubbleDecoration, [
        {
          range: anchor,
          hoverMessage: bubbleHover(best),
          renderOptions: {
            after: {
              contentText: inlineText(best, severity),
              color: SEVERITY_COLOR[severity],
              fontStyle: "normal",
              fontWeight: "600",
              fontFamily: FONT.mono,
            },
          },
        },
      ]);
    }

    this.inlayProvider.set(editor.document.uri, range, best);
    this.active = { editor, clusterId: best.id, range };
  }

  private clearBubble(): void {
    const editor = this.active?.editor;
    if (editor) {
      editor.setDecorations(this.bubbleDecoration, []);
      editor.setDecorations(this.ghostDecoration, []);
    }
    this.inlayProvider.clear();
    this.active = null;
  }
}

class BubbleInlayProvider implements vscode.InlayHintsProvider {
  private readonly changeEmitter = new vscode.EventEmitter<void>();
  readonly onDidChangeInlayHints = this.changeEmitter.event;
  private current: { uri: vscode.Uri; range: vscode.Range; cluster: ReportCluster } | null = null;

  set(uri: vscode.Uri, range: vscode.Range, cluster: ReportCluster): void {
    this.current = { uri, range, cluster };
    this.changeEmitter.fire();
  }

  clear(): void {
    this.current = null;
    this.changeEmitter.fire();
  }

  provideInlayHints(document: vscode.TextDocument, range: vscode.Range): vscode.InlayHint[] {
    if (!this.current) return [];
    if (document.uri.toString() !== this.current.uri.toString()) return [];
    if (!range.contains(this.current.range.start)) return [];
    const strip = signalStrip(this.current.cluster);
    const hint = new vscode.InlayHint(this.current.range.end, strip, vscode.InlayHintKind.Type);
    hint.paddingLeft = true;
    hint.tooltip = bubbleHover(this.current.cluster);
    return [hint];
  }
}

function inlineText(cluster: ReportCluster, severity: Severity): string {
  const canonical = cluster.occurrences[0];
  const count = cluster.occurrences.length;
  const verdict = verdictOf(cluster.signals);
  const location = canonical ? ` · ${shortPath(canonical.path)}` : "";
  return `  ${SEVERITY_DOT[severity]} ${verdict} × ${count}${location}`;
}

function ghostText(cluster: ReportCluster, severity: Severity): string {
  return `  └─ ${SEVERITY_DOT[severity]} ${verdictOf(cluster.signals)}  ${signalStrip(cluster)}  × ${cluster.occurrences.length}`;
}

function signalStrip(cluster: ReportCluster): string {
  const bar = (v: number) => BARS[Math.min(BARS.length - 1, Math.max(0, Math.round(v * (BARS.length - 1))))];
  const s = cluster.signals;
  return `${bar(s.structural)}${bar(s.token_jaccard)}${bar(s.embedding_cos)}`;
}

const BARS = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"] as const;

function shortPath(p: string): string {
  const slash = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return slash >= 0 ? p.slice(slash + 1) : p;
}

function bubbleHover(cluster: ReportCluster): vscode.MarkdownString {
  const md = new vscode.MarkdownString();
  md.isTrusted = true;
  md.supportHtml = true;
  md.appendMarkdown(`**${verdictOf(cluster.signals)}** — ${cluster.interpretation}\n\n`);
  md.appendMarkdown(`\`structural\` ${cluster.signals.structural.toFixed(2)}  `);
  md.appendMarkdown(`\`jaccard\` ${cluster.signals.token_jaccard.toFixed(2)}  `);
  md.appendMarkdown(`\`embedding\` ${cluster.signals.embedding_cos.toFixed(2)}  `);
  md.appendMarkdown(`\`fused\` ${cluster.signals.fused.toFixed(2)}\n\n`);
  const openArgs = encodeURIComponent(JSON.stringify([cluster.id]));
  const dismissArgs = encodeURIComponent(JSON.stringify([cluster.id]));
  md.appendMarkdown(
    `[Open cluster](command:codededup.openCluster?${openArgs}) · ` +
      `[Compare](command:codededup.compareWithCanonical?${openArgs}) · ` +
      `[Dismiss for session](command:codededup.bubble.dismissCluster?${dismissArgs})`,
  );
  return md;
}

function utf8ByteOffset(doc: vscode.TextDocument, position: vscode.Position): number {
  const text = doc.getText(new vscode.Range(new vscode.Position(0, 0), position));
  return Buffer.byteLength(text, "utf8");
}
