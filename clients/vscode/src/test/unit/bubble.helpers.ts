// Shared live-surface test scaffolding: a decoration-capturing editor and
// a signal-explicit cluster builder. Every bubble suite asserts against
// the same rendered strings instead of restating the harness.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { LiveBubble } from "../../bubble/live";
import { ReportStore } from "../../reportStore";
import { Bucket, Report, ReportCluster } from "../../types/report";
import { reportWithClusters } from "./report.helpers";

export interface ClusterSignalOptions {
  // Engine-routed wire bucket. `resolveBucket` prefers it over
  // re-deriving one from the signal triple.
  bucket?: Bucket;
  structural?: number;
  token?: number;
  embedding?: number;
  occurrenceTotal?: number;
}

// Builds a two-occurrence cluster whose fused confidence is explicit, so
// a test can stage the exact [FUSION-CONTENT-GATE] band it is asserting.
export function bubbleCluster(
  id: string,
  weight: number,
  fused: number,
  options: ClusterSignalOptions = {},
): ReportCluster {
  const total = options.occurrenceTotal ?? 2;
  return {
    id,
    weight,
    size: total,
    canonical_node_count: 4,
    bucket: options.bucket ?? "identical",
    signals: {
      structural: options.structural ?? 1,
      token_jaccard: options.token ?? 1,
      embedding_cos: options.embedding ?? 0,
      fused,
    },
    occurrences: [
      { path: "/tmp/A.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/tmp/B.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    occurrences_total: total,
    occurrences_truncated: false,
    summary: "",
    interpretation: "interp",
  };
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

function fakeDocument(file: string): vscode.TextDocument {
  return {
    uri: vscode.Uri.file(file),
    lineAt: () => ({
      range: new vscode.Range(
        new vscode.Position(0, 0),
        new vscode.Position(0, 10),
      ),
    }),
  } as unknown as vscode.TextDocument;
}

export interface BubbleFixture {
  store: ReportStore;
  capture: BubbleCapture;
  bubble: LiveBubble;
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
    bubble: new LiveBubble(store, () => undefined),
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
