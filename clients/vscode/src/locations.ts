// Human editor locations for report byte ranges. The core report keeps
// bytes as the machine contract; VSIX surfaces derive line/column here.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import {
  OccurrenceDisplayLocation,
  Report,
  ReportOccurrence,
} from "./types/report";

export function occurrenceDisplayLocation(
  occurrence: ReportOccurrence,
): OccurrenceDisplayLocation | undefined {
  const source = readOccurrenceSource(occurrence.path);
  if (!source) return undefined;
  const position = positionForByte(source, occurrence.start_byte);
  return {
    line: position.line,
    column: position.column,
    label: `${occurrence.path}:${position.line}:${position.column}`,
    description: `line ${position.line}, column ${position.column}`,
    commandTitle: `Open ${path.basename(occurrence.path)} at ${position.line}:${position.column}`,
  };
}

export function reportWithDisplayLocations(report: Report): Report {
  return {
    ...report,
    clusters: report.clusters.map((cluster) => ({
      ...cluster,
      occurrences: cluster.occurrences.map(withDisplayLocation),
    })),
  };
}

function withDisplayLocation(occurrence: ReportOccurrence): ReportOccurrence {
  const displayLocation = occurrenceDisplayLocation(occurrence);
  return displayLocation ? { ...occurrence, displayLocation } : occurrence;
}

function readOccurrenceSource(occurrencePath: string): string | undefined {
  try {
    return fs.readFileSync(resolveOccurrencePath(occurrencePath), "utf8");
  } catch {
    return undefined;
  }
}

function resolveOccurrencePath(occurrencePath: string): string {
  if (path.isAbsolute(occurrencePath)) return occurrencePath;
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return root ? path.join(root, occurrencePath) : occurrencePath;
}

function positionForByte(source: string, byte: number): { line: number; column: number } {
  const buffer = Buffer.from(source, "utf8");
  const safeByte = Math.min(Math.max(byte, 0), buffer.length);
  const prefix = buffer.slice(0, safeByte).toString("utf8");
  const line = prefix.split("\n").length;
  const lastNewline = prefix.lastIndexOf("\n");
  const columnOffset = lastNewline === -1 ? prefix.length : prefix.length - lastNewline - 1;
  return { line, column: columnOffset + 1 };
}
