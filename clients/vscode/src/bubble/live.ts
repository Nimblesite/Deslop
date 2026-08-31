// Live duplication bubble — [VSIX-LIVE-BUBBLE].
// Fires after every coalesced buffer edit. Calls deslop/duplicatesFindSimilar
// on the most-recently-touched range; admission is `bubbleAdmits`: only an
// explicitly eligible duplicate bucket renders — the engine's own verdict, reached with content
// evidence and byte proof this client never sees. Surfaces:
//   primary: after-text decoration (severity dot + bucket label + count + canonical)
//   secondary: inlay hint with a 3-bar signal strip
// Ghost-line mode renders a whole-line after-text decoration instead.

import * as vscode from "vscode";
import { effect } from "@preact/signals-core";
import type { LanguageClient } from "vscode-languageclient/node";

import { COLOR, DESLOP_SEVERITY_COLOR } from "../design";
import { ReportStore } from "../reportStore";
import { clusterSeverity } from "../severity";
import { ANALYSED_LANGUAGE_IDS } from "../types/languages";
import {
  ReportCluster,
  clusterBand,
  isLiveBubbleBucket,
  resolveBucket,
} from "../types/report";
import { bubbleHover, ghostText, inlineText, signalStrip } from "./renderParts";

export { shortPath } from "../pathUtils";
// The pure text renderers live in ./renderParts; re-exported so every
// consumer keeps one import surface for the bubble.
export * from "./renderParts";

const DEBOUNCE_MS = 250;
const BUDGET_MS = 250;

interface ActiveBubble {
  editor: vscode.TextEditor;
  clusterId: string;
  range: vscode.Range;
}

// [VSIX-STATE-DIRTY] Everything a `findSimilar` answer is only valid *for*,
// captured before the request goes out: the store revision (client-owned
// monotonic token — the wire generation can read the same value twice across
// out-of-order snapshot completions, ABA), the document, and its version. By
// the time the reply resolves any of the three may have moved on, and the
// reply then describes a world that no longer exists. Retraction tombstones
// cannot cover this on their own — a full snapshot settles and clears them,
// so a probe older than the snapshot comes back to an empty ledger and would
// repaint a cluster the snapshot authoritatively omitted.
interface DispatchedAt {
  revision: number;
  uri: string;
  version: number;
}

// One in-flight probe: its supersession epoch (only the newest probe may
// touch the UI — success *or* failure), the cancellation source that a newer
// probe or `dispose()` fires, the budget deadline, and the world it was
// dispatched against.
interface ProbeDispatch {
  epoch: number;
  doc: vscode.TextDocument;
  range: vscode.Range;
  cancellation: vscode.CancellationTokenSource;
  // Set when the budget deadline fired: the edit cycle is skipped outright.
  // Cancellation alone cannot enforce the deadline — `$/cancelRequest` is
  // advisory and a server that ignores it still resolves the promise — so
  // the expiry is recorded and `isSuperseded` rejects the late completion.
  expired: boolean;
  budget: vscode.Disposable;
  dispatchedAt: DispatchedAt;
}

/**
 * Schedules the probe-budget deadline. Injectable so tests fire the
 * deadline deterministically instead of sleeping ([VSIX-LIVE-BUBBLE]).
 */
export type BudgetScheduler = (expire: () => void, ms: number) => vscode.Disposable;

const realBudgetScheduler: BudgetScheduler = (expire, ms) => {
  const handle = setTimeout(expire, ms);
  return { dispose: () => clearTimeout(handle) };
};

export class LiveBubble implements vscode.Disposable {
  private readonly bubbleDecoration: vscode.TextEditorDecorationType;
  private readonly ghostDecoration: vscode.TextEditorDecorationType;
  private readonly inlayProvider: BubbleInlayProvider;
  private readonly disposables: vscode.Disposable[] = [];
  private active: ActiveBubble | null = null;
  private dismissedClusters = new Set<string>();
  private debounceTimer: NodeJS.Timeout | undefined;
  // Bumped on every probe dispatch (and on dispose): a completion whose
  // epoch no longer matches has been superseded and may not touch the UI.
  private probeEpoch = 0;
  // Cancellation source of the newest in-flight probe, so supersession and
  // dispose() can cancel the request instead of letting it stall on.
  private inflight: vscode.CancellationTokenSource | null = null;

  constructor(
    private readonly store: ReportStore,
    private readonly clientOf: () => LanguageClient | undefined,
    private readonly scheduleBudget: BudgetScheduler = realBudgetScheduler,
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
    // Strand any in-flight probe: its epoch can never match again, and the
    // cancel tells the server to stop working on a dead question.
    this.probeEpoch += 1;
    this.inflight?.cancel();
    this.inflight = null;
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

  // Public for the test harness, which dispatches probes and settles their
  // deferred responses out of order to drive the supersession races
  // deterministically. Production dispatch goes through `onEdit`.
  async probe(
    editor: vscode.TextEditor,
    change: vscode.TextDocumentContentChangeEvent,
  ): Promise<void> {
    const client = this.clientOf();
    if (!client) return;
    const dispatch = this.beginProbe(editor.document, change);
    try {
      const clusters = await client.sendRequest<ReportCluster[]>(
        "deslop/duplicatesFindSimilar",
        requestParams(editor.document, dispatch.range),
        dispatch.cancellation.token,
      );
      if (this.isSuperseded(dispatch)) return;
      this.render(editor, dispatch.range, clusters);
    } catch {
      // A failure may only clear the world it was asked about: a stalled
      // probe rejecting (or acknowledging its cancellation) after a newer
      // probe rendered must not erase that newer bubble.
      if (this.isSuperseded(dispatch)) return;
      this.clearBubble();
    } finally {
      this.endProbe(dispatch);
    }
  }

  // Claims a new probe epoch and cancels the previous in-flight request:
  // supersession both stops the stale work and — via the epoch — makes its
  // eventual completion inert even if it settles after cancellation.
  private beginProbe(
    doc: vscode.TextDocument,
    change: vscode.TextDocumentContentChangeEvent,
  ): ProbeDispatch {
    this.inflight?.cancel();
    const cancellation = new vscode.CancellationTokenSource();
    this.inflight = cancellation;
    this.probeEpoch += 1;
    const dispatch: ProbeDispatch = {
      epoch: this.probeEpoch,
      doc,
      range: probeRange(doc, change),
      cancellation,
      expired: false,
      budget: { dispose: () => undefined },
      dispatchedAt: {
        revision: this.store.current.revision,
        uri: doc.uri.toString(),
        version: doc.version,
      },
    };
    dispatch.budget = this.scheduleBudget(() => expireProbe(dispatch), BUDGET_MS);
    return dispatch;
  }

  // True when this completion may no longer touch the UI: its budget
  // deadline fired, a newer probe was dispatched (or the bubble disposed)
  // since, or the world it asked about has moved on. Both the success and
  // the failure path gate on this.
  private isSuperseded(dispatch: ProbeDispatch): boolean {
    return (
      dispatch.expired ||
      dispatch.epoch !== this.probeEpoch ||
      this.hasMovedOn(dispatch.doc, dispatch.dispatchedAt)
    );
  }

  private endProbe(dispatch: ProbeDispatch): void {
    dispatch.budget.dispose();
    if (this.inflight === dispatch.cancellation) this.inflight = null;
    dispatch.cancellation.dispose();
  }

  // True when the world the probe asked about is no longer the current one.
  // Compares the store *revision*, not the wire generation: the revision is
  // client-owned and strictly monotonic, so a snapshot relabelled with an
  // older generation (ABA) still invalidates the answer. Takes the document
  // it was dispatched against rather than reading the active editor: the
  // reply is only valid for *that* buffer, and by the time it lands the user
  // may have focused another one entirely.
  hasMovedOn(doc: vscode.TextDocument, dispatchedAt: DispatchedAt): boolean {
    return (
      this.store.current.revision !== dispatchedAt.revision ||
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

    const severity = clusterBand(best);
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
    // Worst first is the engine's ranking, tie-break included.
    .sort((a, b) => a.rank - b.rank)[0];
}

// One gate: the bucket ([VSIX-LIVE-BUBBLE], [FUSED-CONTENT-GATE]). An
// eligible bucket is the engine's own duplicate verdict,
// reached with content evidence and byte proof this client never sees.
// There is no second admission path: the fused gate is gone from the wire,
// and no UI-local threshold stands in for it — an ineligible cluster
// simply has no engine verdict behind it and does not render. The
// threshold constant exists once, in Rust, and this client never mirrors
// it.
function bubbleAdmits(cluster: ReportCluster): boolean {
  return isLiveBubbleBucket(resolveBucket(cluster));
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

// The probe's budget deadline fired: record the expiry so the completion
// is rejected even if the server ignores the advisory cancellation, and
// still cancel so a compliant server stops working on a dead question.
function expireProbe(dispatch: ProbeDispatch): void {
  dispatch.expired = true;
  dispatch.cancellation.cancel();
}

// The probed range: from the edit's start through the inserted text.
function probeRange(
  doc: vscode.TextDocument,
  change: vscode.TextDocumentContentChangeEvent,
): vscode.Range {
  const start = change.range.start;
  const end = doc.positionAt(doc.offsetAt(start) + change.text.length);
  return new vscode.Range(start, end);
}

// Wire params for deslop/duplicatesFindSimilar: byte offsets, because the
// LSP indexes by UTF-8 bytes while VS Code positions are UTF-16 based.
function requestParams(
  doc: vscode.TextDocument,
  range: vscode.Range,
): { path: string; start_byte: number; end_byte: number } {
  return {
    path: doc.uri.fsPath,
    start_byte: utf8ByteOffset(doc, range.start),
    end_byte: utf8ByteOffset(doc, range.end),
  };
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
