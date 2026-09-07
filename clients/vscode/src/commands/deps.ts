// What every command receives: the factory that hands it the live language
// client, and the guards that turn a palette, tree, or webview argument into
// the occurrence it names. Shared by the command registry and the compare
// commands so neither carries a second copy.

import type { LanguageClient } from "vscode-languageclient/node";

import type { OccurrenceNode } from "../tree/providers";
import type { ReportOccurrence } from "../types/report";

/** Hands back the running language client, or nothing before it starts. */
export type ClientFactory = () => LanguageClient | undefined;

export function isString(value: unknown): value is string {
  return typeof value === "string";
}

export function isNumber(value: unknown): value is number {
  return typeof value === "number";
}

export function isObject(value: unknown): value is object {
  return typeof value === "object" && value !== null;
}

/** The occurrence a command argument names: a tree node's occurrence or a
 * bare occurrence payload; anything else names none. */
export function occurrenceFromCommandTarget(target: unknown): ReportOccurrence | undefined {
  if (isOccurrenceNode(target)) return target.occurrence;
  return isReportOccurrence(target) ? target : undefined;
}

function isOccurrenceNode(target: unknown): target is OccurrenceNode {
  if (!isObject(target) || !("occurrence" in target)) {
    return false;
  }
  return isReportOccurrence(target.occurrence);
}

function isReportOccurrence(target: unknown): target is ReportOccurrence {
  if (!isObject(target)) return false;
  const occurrence = target as Partial<ReportOccurrence>;
  return (
    isString(occurrence.path) &&
    isNumber(occurrence.start_byte) &&
    isNumber(occurrence.end_byte)
  );
}
