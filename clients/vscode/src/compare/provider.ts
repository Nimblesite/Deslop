// Virtual-document source for the "Compare selected occurrences" diff editor.
// Each side of `vscode.diff` is a `deslop-compare:` URI that names one
// occurrence (path + byte range + side). The provider reads the file and
// returns exactly the clone bytes — never the whole file — so same-file
// pairs show the two distinct regions instead of the file vs. itself.

import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as vscode from "vscode";

import { resolveWorkspacePath } from "../pathUtils";

export const COMPARE_SCHEME = "deslop-compare";

export type CompareSide = "a" | "b";

export interface CompareCoordinates {
  readonly sourcePath: string;
  readonly startByte: number;
  readonly endByte: number;
  readonly side: CompareSide;
  readonly pairLabel: string;
}

export function registerCompareProvider(context: vscode.ExtensionContext): void {
  context.subscriptions.push(vscode.workspace.registerTextDocumentContentProvider(COMPARE_SCHEME, new CompareContentProvider()));
}

// Builds a distinct URI per (pair, side). All coordinates live in the
// URI query string so the provider can decode without shared in-process
// state — the extension bundle and the tsc-built test copy each load
// their own module instance, so a shared Map would never work.
export function buildCompareUri(
  occurrence: CompareEndpointRef,
  side: CompareSide,
): vscode.Uri {
  const filename = path.basename(occurrence.path) || "occurrence";
  const query = new URLSearchParams({
    path: occurrence.path,
    start: String(occurrence.start_byte),
    end: String(occurrence.end_byte),
    side,
    pair: COMPARE_PAIR_LABEL,
  }).toString();
  // `Uri.parse` preserves the already-encoded query; `Uri.from({ query })`
  // re-encodes it and double-escapes the percent-signs.
  return vscode.Uri.parse(`${COMPARE_SCHEME}:/${COMPARE_PAIR_LABEL}/${side}/${filename}?${query}`);
}

// [VSIX-PAIR-COMPARE] The diff is between two user-selected endpoints; the
// label names the pair, not a cluster.
const COMPARE_PAIR_LABEL = "selected-pair";

export interface CompareEndpointRef {
  readonly path: string;
  readonly start_byte: number;
  readonly end_byte: number;
}

export function parseCompareUri(uri: vscode.Uri): CompareCoordinates {
  if (uri.scheme !== COMPARE_SCHEME) {
    throw new Error(`expected ${COMPARE_SCHEME} URI, got ${uri.scheme}`);
  }
  const params = new URLSearchParams(uri.query);
  const sourcePath = params.get("path") ?? "";
  const startByte = Number(params.get("start") ?? "0");
  const endByte = Number(params.get("end") ?? "0");
  const side = params.get("side") === "b" ? "b" : "a";
  const pairLabel = params.get("pair") ?? "";
  return { sourcePath, startByte, endByte, side, pairLabel };
}

export class CompareContentProvider implements vscode.TextDocumentContentProvider {
  async provideTextDocumentContent(uri: vscode.Uri): Promise<string> {
    const coords = parseCompareUri(uri);
    const sourcePath = resolveWorkspacePath(coords.sourcePath);
    const source = await readCompareSource(sourcePath, coords);
    if (source.kind === "unavailable") return source.text;
    const buffer = source.buffer;
    const start = clamp(coords.startByte, 0, buffer.length);
    const end = clamp(coords.endByte, start, buffer.length);
    return buffer.subarray(start, end).toString("utf8");
  }
}

type CompareSource =
  | { readonly kind: "content"; readonly buffer: Buffer }
  | { readonly kind: "unavailable"; readonly text: string };

async function readCompareSource(
  sourcePath: string,
  coords: CompareCoordinates,
): Promise<CompareSource> {
  try {
    return { kind: "content", buffer: await fs.readFile(sourcePath) };
  } catch (err) {
    return { kind: "unavailable", text: compareUnavailableText(coords, err) };
  }
}

function compareUnavailableText(coords: CompareCoordinates, err: unknown): string {
  const reason = err instanceof Error ? err.message : String(err);
  return [
    "Deslop could not load this compare occurrence.",
    "",
    `Pair: ${coords.pairLabel || "unknown"}`,
    `Side: ${coords.side.toUpperCase()}`,
    `Path: ${coords.sourcePath || "unknown"}`,
    "",
    "Refresh the Deslop report and try Compare again. The file may have moved or been deleted.",
    "",
    `Details: ${reason}`,
  ].join("\n");
}

function clamp(n: number, lo: number, hi: number): number {
  if (Number.isNaN(n)) return lo;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
}
