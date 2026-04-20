// Virtual-document source for the "Compare occurrences" diff editor.
// Each side of `vscode.diff` is a `deslop-compare:` URI that names one
// occurrence. The provider reads the file and returns exactly the clone
// bytes — never the whole file — so same-file clusters show the two
// distinct regions instead of the file vs. itself.

import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as vscode from "vscode";

import { ReportOccurrence } from "../types/report";

export const COMPARE_SCHEME = "deslop-compare";

export type CompareSide = "a" | "b";

export interface CompareCoordinates {
  readonly sourcePath: string;
  readonly startByte: number;
  readonly endByte: number;
  readonly side: CompareSide;
  readonly clusterId: string;
}

// Single-process coordinate table keyed by the URI string. The provider
// resolves content through this map instead of round-tripping the full
// absolute path through URI encoding, which used to double-encode on
// reopen and break `openTextDocument`.
const entries = new Map<string, CompareCoordinates>();

export function registerCompareProvider(context: vscode.ExtensionContext): void {
  const provider = new CompareContentProvider();
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider(COMPARE_SCHEME, provider),
  );
}

// Builds a distinct URI per (cluster, side) and records the byte-range
// under that URI so the provider can slice the file at resolve time.
export function buildCompareUri(
  occurrence: ReportOccurrence,
  side: CompareSide,
  clusterId: string,
): vscode.Uri {
  const filename = path.basename(occurrence.path) || "occurrence";
  const uri = vscode.Uri.from({
    scheme: COMPARE_SCHEME,
    path: `/${clusterId}/${side}/${filename}`,
  });
  entries.set(uri.toString(), {
    sourcePath: occurrence.path,
    startByte: occurrence.start_byte,
    endByte: occurrence.end_byte,
    side,
    clusterId,
  });
  return uri;
}

// Exported for tests.
export function lookupCompareCoordinates(uri: vscode.Uri): CompareCoordinates | undefined {
  return entries.get(uri.toString());
}

class CompareContentProvider implements vscode.TextDocumentContentProvider {
  async provideTextDocumentContent(uri: vscode.Uri): Promise<string> {
    const coords = entries.get(uri.toString());
    if (!coords) throw new Error(`no compare coordinates registered for ${uri.toString()}`);
    const buffer = await fs.readFile(coords.sourcePath);
    const start = clamp(coords.startByte, 0, buffer.length);
    const end = clamp(coords.endByte, start, buffer.length);
    return buffer.subarray(start, end).toString("utf8");
  }
}

function clamp(n: number, lo: number, hi: number): number {
  if (Number.isNaN(n)) return lo;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
}
