// Human editor locations for report byte ranges. The core report keeps
// bytes as the machine contract; VSIX surfaces derive line/column here.
//
// [VSIX-PERF] A report enrichment pass reads each source file at most once and
// reuses it for every occurrence in that file, instead of one synchronous
// fs.readFileSync per occurrence — the old shape was O(occurrences) blocking
// reads on the extension-host thread for every webview push.

import * as fs from "node:fs";
import * as path from "node:path";

import {
  OccurrenceDisplayLocation,
  Report,
  ReportOccurrence,
} from "./types/report";
import { resolveWorkspacePath } from "./pathUtils";

export function occurrenceDisplayLocation(
  occurrence: ReportOccurrence,
): OccurrenceDisplayLocation | undefined {
  return displayLocationFrom(occurrence, readOccurrenceSource(occurrence.path));
}

// `readSource` is injectable so the per-file memo can be exercised deterministically
// in tests; production always uses the filesystem read.
export function reportWithDisplayLocations(
  report: Report,
  readSource: (occurrencePath: string) => string | undefined = readOccurrenceSource,
): Report {
  const sources = new Map<string, string | undefined>();
  const sourceFor = (occurrencePath: string): string | undefined => {
    if (!sources.has(occurrencePath)) sources.set(occurrencePath, readSource(occurrencePath));
    return sources.get(occurrencePath);
  };
  return {
    ...report,
    clusters: report.clusters.map((cluster) => ({
      ...cluster,
      occurrences: cluster.occurrences.map((occurrence) =>
        withDisplayLocation(occurrence, sourceFor(occurrence.path)),
      ),
    })),
  };
}

function withDisplayLocation(occurrence: ReportOccurrence, source: string | undefined): ReportOccurrence {
  const displayLocation = displayLocationFrom(occurrence, source);
  return displayLocation ? { ...occurrence, displayLocation } : occurrence;
}

function displayLocationFrom(
  occurrence: ReportOccurrence,
  source: string | undefined,
): OccurrenceDisplayLocation | undefined {
  if (source === undefined) return undefined;
  const position = positionForByte(source, occurrence.start_byte);
  return {
    line: position.line,
    column: position.column,
    label: `${occurrence.path}:${position.line}:${position.column}`,
    description: `line ${position.line}, column ${position.column}`,
    commandTitle: `Open ${path.basename(occurrence.path)} at ${position.line}:${position.column}`,
  };
}

function readOccurrenceSource(occurrencePath: string): string | undefined {
  try {
    return fs.readFileSync(resolveWorkspacePath(occurrencePath), "utf8");
  } catch {
    return undefined;
  }
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
