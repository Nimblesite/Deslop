// Shared live-surface test scaffolding: a decoration-capturing editor and
// a signal-explicit cluster builder. Every bubble suite asserts against
// the same rendered strings instead of restating the harness.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { BudgetScheduler, LiveBubble } from "../../bubble/live";
import { ReportStore } from "../../reportStore";
import { Bucket, Report, ReportCluster } from "../../types/report";
import { repoMetrics, reportWithClusters } from "./report.helpers";
import { occurrence, wireCluster } from "../cluster.helpers";
import { signalsWith } from "../signals.helpers";

export interface ClusterSignalOptions {
  // Engine-routed wire bucket. `resolveBucket` prefers it over
  // re-deriving one from the signal triple.
  bucket?: Bucket;
  structural?: number;
  token?: number;
  /** The engine's shape reading. Defaults to the stronger shape axis,
   * which is what the engine stamps, but a suite pinning the bubble's
   * shape bar sets it outright. */
  shape?: number;
  embedding?: number;
  /** The engine's fused-gate verdict. Set when a suite stages a cluster
   * on a specific side of the reportable line. */
  meetsFusedGate?: boolean;
  /** The engine's global worst-first rank, when the suite stages more
   * than one candidate and pins which wins. */
  rank?: number;
  occurrenceTotal?: number;
}

/** The confidence the engine's reportable cutoff sits at today
 * (`deslop-core::pair::FUSED_THRESHOLD`). Restated here so these
 * fixtures can stage clusters on either side of it — production code
 * reads the engine's own `meets_fused_gate` verdict and owns no copy of
 * this number ([FUSION-CONTENT-GATE]). */
export const ENGINE_FUSED_CUTOFF = 0.85;

// Builds a two-occurrence cluster whose fused confidence is explicit, so
// a test can stage the exact [FUSION-CONTENT-GATE] band it is asserting.
export function bubbleCluster(
  id: string,
  weight: number,
  fused: number,
  options: ClusterSignalOptions = {},
): ReportCluster {
  const total = options.occurrenceTotal ?? 2;
  const bucket = options.bucket ?? "identical";
  const structural = options.structural ?? 1;
  const token = options.token ?? 1;
  return wireCluster({
    id,
    rank: options.rank ?? 1,
    weight,
    size: total,
    bucket,
    signals: signalsWith(bucket, {
      structural,
      token_jaccard: token,
      shape: options.shape ?? Math.max(structural, token),
      embedding_cos: options.embedding ?? 0,
      fused,
    }),
    meets_fused_gate: options.meetsFusedGate ?? fused >= ENGINE_FUSED_CUTOFF,
    occurrences: [
      occurrence("/tmp/A.cs", 0, 10),
      occurrence("/tmp/B.cs", 0, 10),
    ],
    occurrences_total: total,
    occurrence_count: total,
    interpretation: "interp",
  });
}

// The probe-shaped cluster the live-surface suites drive renders with:
// an embedding-bearing default whose occurrence total is only set when a
// test is pinning the report-vs-probe count contract.
export function probeCluster(
  id: string,
  weight: number,
  fused: number,
  occurrenceTotal?: number,
): ReportCluster {
  const built = bubbleCluster(id, weight, fused, {
    embedding: 0.5,
    occurrenceTotal: occurrenceTotal ?? 2,
  });
  return { ...built, occurrences_total: occurrenceTotal ?? 0 };
}

// A one-cluster snapshot whose canonical entry carries five occurrences,
// so a probe claiming a different count is visibly wrong.
export function probeReport(): Report {
  return reportWithClusters(
    [probeCluster("c-a", 10, 0.95, 5)],
    {},
    {
      analysed_loc: 10,
      duplicated_loc: 2,
      duplication_percent: 20,
      duplicated_files: 2,
    },
  );
}

export function span(startChar: number): vscode.Range {
  return new vscode.Range(
    new vscode.Position(0, startChar),
    new vscode.Position(0, startChar + 4),
  );
}

export async function setBubbleMode(mode: "inline" | "ghost"): Promise<void> {
  await vscode.workspace
    .getConfiguration("deslop")
    .update("liveBubble.mode", mode, vscode.ConfigurationTarget.Workspace);
}

export interface DecorationCall {
  texts: string[];
  hovers: (vscode.MarkdownString | undefined)[];
}

export interface BubbleCapture {
  editor: vscode.TextEditor;
  calls: DecorationCall[];
  // Text of the bubble currently on screen, or undefined when cleared.
  visible(): string | undefined;
  // Hover card attached to the visible bubble, if any.
  visibleHover(): vscode.MarkdownString | undefined;
  // Every non-empty decoration text rendered so far, oldest first.
  history(): string[];
}

function decorationShape(option: unknown): {
  renderOptions?: { after?: { contentText?: string } };
  hoverMessage?: vscode.MarkdownString;
} {
  return option as {
    renderOptions?: { after?: { contentText?: string } };
    hoverMessage?: vscode.MarkdownString;
  };
}

function collect(options: readonly unknown[]): DecorationCall {
  const texts: string[] = [];
  const hovers: (vscode.MarkdownString | undefined)[] = [];
  for (const option of options) {
    const shape = decorationShape(option);
    const text = shape.renderOptions?.after?.contentText;
    if (text !== undefined) texts.push(text);
    hovers.push(shape.hoverMessage);
  }
  return { texts, hovers };
}

// `render` and `clearBubble` each touch both decoration surfaces, so the
// live state is whichever of the last two calls carried content.
function activeCall(calls: DecorationCall[]): DecorationCall {
  for (const call of calls.slice(-2)) {
    if (call.texts.length > 0) return call;
  }
  return { texts: [], hovers: [] };
}

// A single line of known content so `probe()` can compute ranges and UTF-8
// byte offsets deterministically against the capturing editor.
const FAKE_DOCUMENT_LINE = "0123456789abcdefghijklmnopqrstuvwxyz";

function fakeDocument(file: string): vscode.TextDocument {
  return {
    uri: vscode.Uri.file(file),
    version: 1,
    offsetAt: (position: vscode.Position) => position.character,
    positionAt: (offset: number) =>
      new vscode.Position(0, Math.min(offset, FAKE_DOCUMENT_LINE.length)),
    getText: (range?: vscode.Range) =>
      range
        ? FAKE_DOCUMENT_LINE.slice(range.start.character, range.end.character)
        : FAKE_DOCUMENT_LINE,
    lineAt: () => ({
      range: new vscode.Range(
        new vscode.Position(0, 0),
        new vscode.Position(0, 10),
      ),
    }),
  } as unknown as vscode.TextDocument;
}

// The content-change event `onEdit` hands to `probe`: an insertion of
// `text` at `startChar` on the fake document's single line.
export function editAt(
  startChar: number,
  text: string,
): vscode.TextDocumentContentChangeEvent {
  const start = new vscode.Position(0, startChar);
  return {
    range: new vscode.Range(start, start),
    rangeOffset: startChar,
    rangeLength: 0,
    text,
  };
}

export interface DeferredProbeRequest {
  params: { path: string; start_byte: number; end_byte: number };
  token: vscode.CancellationToken;
  resolve(clusters: ReportCluster[]): void;
  reject(error: Error): void;
}

// A fake LSP client whose findSimilar responses are deferred promises the
// test settles by hand — the deterministic way to stage out-of-order probe
// completions. No timers, no sleeps: the test holds each probe() promise
// and awaits it only after settling that probe's deferred response.
export function deferredProbeClient(): {
  client: LanguageClient;
  requests: DeferredProbeRequest[];
} {
  const requests: DeferredProbeRequest[] = [];
  const client = {
    sendRequest: (
      _method: string,
      params: DeferredProbeRequest["params"],
      token: vscode.CancellationToken,
    ) =>
      new Promise((resolve, reject) => {
        requests.push({ params, token, resolve, reject });
      }),
  } as unknown as LanguageClient;
  return { client, requests };
}

export async function resolveProbe(
  request: DeferredProbeRequest | undefined,
  probe: Promise<void>,
  cancellationExpected?: boolean,
  clusters: ReportCluster[] = [probeCluster("c-a", 10, 0.95)],
): Promise<void> {
  assert.ok(request !== undefined, "probe request must exist");
  if (cancellationExpected !== undefined) {
    assert.equal(
      request.token.isCancellationRequested,
      cancellationExpected,
      `request cancellation must be ${cancellationExpected}`,
    );
  }
  request.resolve(clusters);
  await probe;
}

export interface BubbleFixture {
  store: ReportStore;
  capture: BubbleCapture;
  bubble: LiveBubble;
}

export async function openLiveDocument(content: string): Promise<{
  doc: vscode.TextDocument;
  editor: vscode.TextEditor;
  store: ReportStore;
}> {
  const doc = await vscode.workspace.openTextDocument({
    content,
    language: "csharp",
  });
  const editor = await vscode.window.showTextDocument(doc);
  const store = new ReportStore();
  store.setSnapshot(probeReport(), 0);
  return { doc, editor, store };
}

// One assembled live-bubble rig: a store seeded with `snapshot` (pass
// null for the no-report case), the decoration capture, and the bubble
// under test, with the render mode already applied. Every live-surface
// test opens with this — the five-line preamble it replaces was the
// repo's third-worst duplication cluster.
export async function bubbleFixture(
  options: {
    snapshot?: Report | null;
    generation?: number;
    mode?: "inline" | "ghost";
    client?: LanguageClient;
    budget?: BudgetScheduler;
  } = {},
): Promise<BubbleFixture> {
  const store = new ReportStore();
  const snapshot =
    options.snapshot === undefined ? probeReport() : options.snapshot;
  if (snapshot) store.setSnapshot(snapshot, options.generation ?? 0);
  await setBubbleMode(options.mode ?? "inline");
  return {
    store,
    capture: capturingEditor(),
    bubble: new LiveBubble(store, () => options.client, options.budget),
  };
}

// Asserts a bubble is on screen carrying `title`, and returns its text so
// the caller can keep asserting against the same rendered string.
export function assertBubbleShows(
  capture: BubbleCapture,
  title: string,
  context: string,
): string {
  const visible = capture.visible();
  assert.ok(visible !== undefined, `${context}: expected a visible bubble`);
  assert.match(
    visible ?? "",
    new RegExp(title),
    `${context}: expected the ${title} title`,
  );
  return visible ?? "";
}

export function renderFullConfidenceBubble(
  capture: BubbleCapture,
  bubble: LiveBubble,
  startChar: number,
  clusterId: string,
): string {
  bubble.render(capture.editor, span(startChar), [
    probeCluster(clusterId, 10, 0.95),
  ]);
  return assertBubbleShows(
    capture,
    "Identical code",
    `expected ${clusterId} at character ${startChar}`,
  );
}

export function retractCluster(store: ReportStore, clusterId: string): void {
  store.applyDelta({
    from_generation: 1,
    to_generation: 2,
    clusters_added: [],
    clusters_removed: [clusterId],
    clusters_updated: [],
    metrics: repoMetrics({ analysed_loc: 10 }),
    cache_stats: { hits: 0, misses: 0 },
    tool_version: "v2",
  });
}

export function capturingEditor(file = "/tmp/A.cs"): BubbleCapture {
  const calls: DecorationCall[] = [];
  const editor = {
    document: fakeDocument(file),
    setDecorations: (
      _type: vscode.TextEditorDecorationType,
      options: readonly unknown[],
    ) => {
      calls.push(collect(options));
    },
  } as unknown as vscode.TextEditor;
  return {
    editor,
    calls,
    visible: () => activeCall(calls).texts[0],
    visibleHover: () => activeCall(calls).hovers[0],
    history: () => calls.flatMap((call) => call.texts),
  };
}
