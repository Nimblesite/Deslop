// Live duplication bubble — [VSIX-LIVE-BUBBLE].
// Fires after every coalesced buffer edit. Calls deslop/duplicatesFindSimilar
// on the most-recently-touched range; if fused >= 0.85, renders:
//   primary: after-text decoration (severity dot + bucket label + count + canonical)
//   secondary: inlay hint with a 3-bar signal strip
// Ghost-line mode uses a CodeLens on a phantom line.

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";
import type { LanguageClient } from "vscode-languageclient/node";

import { clusterHoverMarkdown, clusterSlug } from "../clusterHover";
import { COLOR, DESLOP_SEVERITY_COLOR, SEVERITY_DOT } from "../design";
import { shortPath } from "../pathUtils";
import { ReportStore } from "../reportStore";
import { clusterSeverity, indexedSeverity } from "../severity";
import { ANALYSED_LANGUAGE_IDS } from "../types/languages";
import {
  FUSED_THRESHOLD,
  ReportCluster,
  Severity,
  bucketLabels,
  isActNow,
  occurrenceCount,
  resolveBucket,
} from "../types/report";

export { shortPath } from "../pathUtils";

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
        ANALYSED_LANGUAGE_IDS.map((language) => ({ language })),
        this.inlayProvider,
      ),
      // effect() tracks store.report (read inside clearRemovedActiveCluster).
      // Clears the bubble automatically when the active cluster disappears.
      { dispose: effect(() => this.clearRemovedActiveCluster()) },
      vscode.workspace.onDidChangeTextDocument((e) => this.onEdit(e)),
      vscode.window.onDidChangeActiveTextEditor(() => this.clearBubble()),
    );
    this.tryRegister("deslop.bubble.dismiss", () => this.dismiss());
    this.tryRegister("deslop.bubble.dismissCluster", (id) =>
      this.dismissCluster(String(id)),
    );
  }

  // Clears the active bubble without suppressing the cluster — the next
  // probe may paint it again. The `deslop.bubble.dismiss` command is a
  // thin wrapper over this.
  dismiss(): void {
    this.clearBubble();
  }

  // Suppresses `clusterId` from every future render on this instance and
  // clears the active bubble. The `deslop.bubble.dismissCluster` command
  // is a thin wrapper over this. Exposed as a method because command
  // registration is idempotent across instances (see `tryRegister`): only
  // the first `LiveBubble` in a process owns the command id, so driving
  // dismissal through `executeCommand` would target that instance rather
  // than this one.
  dismissCluster(clusterId: string): void {
    this.dismissedClusters.add(clusterId);
    this.clearBubble();
  }

  // Idempotent registration so multiple LiveBubble instances (real + tests)
  // can co-exist without "command already exists" throws.
  private tryRegister(
    id: string,
    handler: (...args: unknown[]) => unknown,
  ): void {
    try {
      this.disposables.push(vscode.commands.registerCommand(id, handler));
    } catch {
      // already registered by an earlier instance
    }
  }

  dispose(): void {
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    for (const d of this.disposables) d.dispose();
  }

  private onEdit(event: vscode.TextDocumentChangeEvent): void {
    const cfg = vscode.workspace.getConfiguration("deslop");
    if (!cfg.get<boolean>("liveBubble.enabled", true)) return;
    const editor = vscode.window.activeTextEditor;
    if (editor?.document !== event.document) return;
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

    // [VSIX-STATE-DIRTY] Everything the answer is only valid *for*, captured
    // before the request goes out. A `findSimilar` response describes one
    // range of one document at one report generation; by the time it resolves
    // any of the three may have moved on, and the reply then describes a world
    // that no longer exists. Retraction tombstones cannot cover this on their
    // own — a full snapshot settles and clears them, so a probe older than the
    // snapshot comes back to an empty ledger and repaints a cluster the
    // snapshot authoritatively omitted.
    const dispatchedAt = {
      generation: this.store.current.generation,
      uri: doc.uri.toString(),
      version: doc.version,
    };
    const cancellation = new vscode.CancellationTokenSource();
    const timeout = setTimeout(() => cancellation.cancel(), BUDGET_MS);
    try {
      const clusters = await client.sendRequest<ReportCluster[]>(
        "deslop/duplicatesFindSimilar",
        { path: doc.uri.fsPath, start_byte: startByte, end_byte: endByte },
        cancellation.token,
      );
      if (this.hasMovedOn(doc, dispatchedAt)) return;
      this.render(editor, range, clusters);
    } catch {
      this.clearBubble();
    } finally {
      clearTimeout(timeout);
      cancellation.dispose();
    }
  }

  // True when the world the probe asked about is no longer the current one.
  // Takes the document it was dispatched against rather than reading the
  // active editor: the reply is only valid for *that* buffer, and by the time
  // it lands the user may have focused another one entirely.
  // Public for the test harness, which drives the async race directly.
  hasMovedOn(
    doc: vscode.TextDocument,
    dispatchedAt: { generation: number; uri: string; version: number },
  ): boolean {
    return (
      this.store.current.generation !== dispatchedAt.generation ||
      doc.uri.toString() !== dispatchedAt.uri ||
      doc.version !== dispatchedAt.version
    );
  }

  // Public for test harness only — production call sites go through `probe()`.
  render(
    editor: vscode.TextEditor,
    range: vscode.Range,
    clusters: ReportCluster[],
  ): void {
    // [VSIX-STATE-DIRTY]: bubble is a surface — derive from the visible
    // projection so an in-progress edit dismisses the bubble immediately.
    const report = this.store.current.visibleReport;
    if (!report) return;
    const best = bestBubbleCluster(
      report.clusters,
      clusters,
      this.dismissedClusters,
      this.store.current.retractedClusters,
    );
    if (!best) {
      this.clearBubble();
      return;
    }
    if (this.active?.clusterId === best.id && this.active.range.isEqual(range))
      return;

    const severities = indexedSeverity(report.clusters);
    const severity = severities.get(best.id) ?? "faint";
    const mode = vscode.workspace
      .getConfiguration("deslop")
      .get<string>("liveBubble.mode", "inline");
    const lineEnd = editor.document.lineAt(range.end.line).range.end;
    const anchor = new vscode.Range(lineEnd, lineEnd);

    if (mode === "ghost") {
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
              // [SEVERITY-COLOR] Colour is the bucket channel; the dot inside
              // `inlineText` is the percentile channel. The bubble carries both
              // facts at once — a demoted family topping the report is a grey
              // `●●`, never the crimson that means "safe to extract".
              color: DESLOP_SEVERITY_COLOR[clusterSeverity(best)],
              fontStyle: "normal",
              fontWeight: "600",
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

  private clearRemovedActiveCluster(): void {
    // Read the signal unconditionally so the effect always tracks it,
    // even when there is no active bubble yet. [VSIX-STATE-DIRTY]: the
    // bubble must clear when an edit hides the cluster from the visible
    // projection, even if the LSP still has it canonically.
    const report = this.store.current.visibleReport;
    const active = this.active;
    if (!active || !report) return;
    const stillPresent = report.clusters.some(
      (cluster) => cluster.id === active.clusterId,
    );
    if (!stillPresent) this.clearBubble();
  }
}

function bestBubbleCluster(
  reportClusters: ReportCluster[],
  probeClusters: ReportCluster[],
  dismissedClusters: Set<string>,
  retractedClusters: ReadonlySet<string>,
): ReportCluster | undefined {
  const byId = new Map(reportClusters.map((cluster) => [cluster.id, cluster]));
  return probeClusters
    .filter((cluster) => !retractedClusters.has(cluster.id))
    .map((cluster) => byId.get(cluster.id) ?? cluster)
    .filter(bubbleAdmits)
    .filter((cluster) => !dismissedClusters.has(cluster.id))
    .sort((a, b) => b.weight - a.weight)[0];
}

// Two gates, because the two populations carry different evidence
// ([VSIX-LIVE-BUBBLE], [FUSION-CONTENT-GATE]). An act-now bucket is the
// engine's own verdict that the user should act, reached with content
// evidence and byte proof this client never sees; the same gate
// deliberately pushes a proven rename's confidence *below*
// `FUSED_THRESHOLD`, so re-testing an act-now cluster against a UI-local
// cutoff withholds precisely the findings this surface exists to show.
// Below the act-now bands no such verdict stands behind the cluster and
// the fused cutoff is the right gate — a weak LSH hint is worth the
// user's attention only once it clears the line.
function bubbleAdmits(cluster: ReportCluster): boolean {
  return (
    isActNow(resolveBucket(cluster)) || cluster.signals.fused >= FUSED_THRESHOLD
  );
}

class BubbleInlayProvider implements vscode.InlayHintsProvider {
  private readonly changeEmitter = new vscode.EventEmitter<void>();
  readonly onDidChangeInlayHints = this.changeEmitter.event;
  private current: {
    uri: vscode.Uri;
    range: vscode.Range;
    cluster: ReportCluster;
  } | null = null;

  set(
    uri: vscode.Uri,
    range: vscode.Range,
    cluster: ReportCluster,
  ): void {
    this.current = { uri, range, cluster };
    this.changeEmitter.fire();
  }

  clear(): void {
    this.current = null;
    this.changeEmitter.fire();
  }

  provideInlayHints(
    document: vscode.TextDocument,
    range: vscode.Range,
  ): vscode.InlayHint[] {
    if (!this.current) return [];
    if (document.uri.toString() !== this.current.uri.toString()) return [];
    if (!range.contains(this.current.range.start)) return [];
    const strip = signalStrip(this.current.cluster);
    const hint = new vscode.InlayHint(
      this.current.range.end,
      strip,
      vscode.InlayHintKind.Type,
    );
    hint.paddingLeft = true;
    hint.tooltip = bubbleHover(this.current.cluster);
    return [hint];
  }
}

// The inline bubble and ghost-line decorations are pure-visual
// surfaces (rendered only in the editor, never scraped by agents), so
// they use `plainTitle` per [CLONE-BUCKETS-DUAL-LABEL].
export interface BubbleRenderParts {
  inline: string;
  ghost: string;
  signalStrip: string;
  hover: vscode.MarkdownString;
}

export function renderBubbleParts(
  cluster: ReportCluster,
  severity: Severity,
): BubbleRenderParts {
  const canonical = cluster.occurrences[0];
  const count = occurrenceCount(cluster);
  const title = bucketLabels(resolveBucket(cluster)).plainTitle;
  const slug = clusterSlug(cluster);
  const location = canonical ? ` · ${shortPath(canonical.path)}` : "";
  const strip = signalStrip(cluster);
  return {
    inline: `  ${SEVERITY_DOT[severity]} ${slug} ${title} × ${count}${location}`,
    ghost: `  └─ ${SEVERITY_DOT[severity]} ${slug} ${title}  ${strip}  × ${count}`,
    signalStrip: strip,
    hover: clusterHoverMarkdown(cluster, { showDismiss: true }),
  };
}

export function inlineText(
  cluster: ReportCluster,
  severity: Severity,
): string {
  return renderBubbleParts(cluster, severity).inline;
}

export function ghostText(
  cluster: ReportCluster,
  severity: Severity,
): string {
  return renderBubbleParts(cluster, severity).ghost;
}

// Three bars: shape, semantic, confidence ([VSIX-LIVE-BUBBLE]).
// `structural` and `token_jaccard` are two views of one normalised
// representation — "summing them says nothing beyond 'the shapes matched'"
// (`deslop-core::buckets::content_gated_signals`) — so drawing both spends
// two of the three slots on a single piece of evidence and leaves none for
// the content-gated confidence. That confidence is the only thing
// separating a verbatim copy from a proven rename: after the #232
// correction both render `structural 1.0 / token_jaccard 1.0`, so a strip
// without `fused` collapses "safe to extract" and "identifiers differ"
// onto the same three glyphs. The shape bar takes the stronger of the two
// shape views; the third bar draws `fused`.
export function signalStrip(cluster: ReportCluster): string {
  const signals = cluster.signals;
  const shape = Math.max(signals.structural, signals.token_jaccard);
  return `${bar(shape)}${bar(signals.embedding_cos)}${bar(signals.fused)}`;
}

// The full block is reserved for an exact 1.0 and nothing else. Rounding
// `value * 7` gave it to everything from 0.929 up, which collapsed the two
// readings the third bar exists to separate: a byte-proven copy renders
// `fused 1.00` and a content-gated near-verbatim clone renders `fused 0.96`,
// and both drew `█`. Proof and near-proof are exactly the distinction a
// glance at this strip is supposed to make, so the top glyph means proof.
function bar(value: number): string {
  if (value >= 1) return BARS[BARS.length - 1] ?? "█";
  const below = BARS.length - 1;
  const index = Math.min(
    below - 1,
    Math.max(0, Math.round(value * (below - 1))),
  );
  return BARS[index] ?? "▁";
}

const BARS = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"] as const;

// Bubble hover: full card with slug, canonical, and dismiss link.
export function bubbleHover(
  cluster: ReportCluster,
): vscode.MarkdownString {
  return renderBubbleParts(cluster, "faint").hover;
}

function utf8ByteOffset(
  doc: vscode.TextDocument,
  position: vscode.Position,
): number {
  const text = doc.getText(
    new vscode.Range(new vscode.Position(0, 0), position),
  );
  return Buffer.byteLength(text, "utf8");
}
