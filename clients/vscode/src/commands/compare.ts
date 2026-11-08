// [VSIX-PAIR-COMPARE] The two compare commands: a row against its current
// canonical, and two explicit endpoints. Both open the occurrence-range diff
// at once and title it with the engine's verdict on exactly that pair when the
// verdict arrives (#607 #608). No click is ever a silent no-op.

import * as path from "node:path";
import * as vscode from "vscode";

import { buildCompareUri, CompareEndpointRef } from "../compare/provider";
import { compareTitle, measurePair } from "../compare/title";
import { logWarn } from "../logging";
import { ReportStore } from "../reportStore";
import type { PairEvidence, ReportCluster, ReportOccurrence } from "../types/report";
import { ClientFactory, isObject, isString, occurrenceFromCommandTarget } from "./deps";

const CANONICAL_OCCURRENCE_INDEX = 0;
const FIRST_PEER_INDEX = 1;
const DIFF_COMMAND = "vscode.diff";
const MALFORMED_ROW = "Deslop cannot compare this row: it does not name a file range. Refresh the report and try again.";
const MISSING_CLUSTER = "Deslop cannot compare: this duplicate group is no longer in the report. Refresh the report and try again.";
const MISSING_SIDE = "Deslop needs two locations to compare. Pick a location, then use Compare with Selected on another.";

/** Verdicts already being measured, by pair, so repeated clicks ask the engine once. */
const pendingVerdicts = new Map<string, Promise<PairEvidence | undefined>>();

// [VSIX-PAIR-COMPARE] A row compares its exact range with the canonical of the
// cluster that holds it. The id the row was drawn with only narrows the search:
// when the report has moved on, the row's own cluster answers (#607).
export async function compareWithCanonicalTarget(
  store: ReportStore, clientOf: ClientFactory, target: unknown, occurrence?: unknown,
): Promise<void> {
  const clusterRow = clusterOfTarget(target);
  const row = occurrence ?? (isString(target) || clusterRow ? undefined : target);
  const selected = occurrenceFromCommandTarget(row);
  if (row !== undefined && !selected) return tell(MALFORMED_ROW);
  const clusterId = isString(target) ? target : clusterRow?.id;
  const cluster = owningCluster(store.current.report?.clusters ?? [], clusterId, selected);
  if (!cluster) return tell(selected ? notInReport(selected) : MISSING_CLUSTER);
  const canonical = cluster.occurrences[CANONICAL_OCCURRENCE_INDEX];
  const peer = selected ?? cluster.occurrences[FIRST_PEER_INDEX];
  if (!canonical || !peer) return tell(MISSING_CLUSTER);
  if (sameEndpoint(canonical, peer)) return tell(isCanonical(canonical));
  await openCompareDiff(clientOf, canonical, peer);
}

/**
 * The cluster a tree cluster row carries. Recognised by shape, as occurrence
 * rows are: a row built by another copy of the tree module is still a row.
 */
function clusterOfTarget(target: unknown): ReportCluster | undefined {
  if (!isObject(target) || !("cluster" in target)) return undefined;
  const { cluster } = target;
  if (!isObject(cluster)) return undefined;
  const candidate = cluster as Partial<ReportCluster>;
  return isString(candidate.id) && Array.isArray(candidate.occurrences) ? (cluster as ReportCluster) : undefined;
}

/** The cluster that holds `row` — the named one when it still does — or, with no row, the named one. */
function owningCluster(
  clusters: readonly ReportCluster[], clusterId: string | undefined, row: ReportOccurrence | undefined,
): ReportCluster | undefined {
  const named = clusterId ? clusters.find((candidate) => candidate.id === clusterId) : undefined;
  if (!row) return named;
  const holds = (cluster: ReportCluster): boolean => cluster.occurrences.some((peer) => sameEndpoint(peer, row));
  return named && holds(named) ? named : clusters.find(holds);
}

// [VSIX-PAIR-COMPARE] Both comparison routes share the occurrence-range diff,
// titled with the engine's verdict on exactly these two endpoints.
export async function comparePairEndpoints(clientOf: ClientFactory, left: unknown, right: unknown): Promise<void> {
  const leftEndpoint = compareEndpoint(left);
  const rightEndpoint = compareEndpoint(right);
  if (!leftEndpoint || !rightEndpoint) return tell(MISSING_SIDE);
  if (sameEndpoint(leftEndpoint, rightEndpoint)) return tell(sameLocation(leftEndpoint));
  await openCompareDiff(clientOf, leftEndpoint, rightEndpoint);
}

function compareEndpoint(value: unknown): CompareEndpointRef | undefined {
  if (!isObject(value)) return undefined;
  const candidate = value as Partial<CompareEndpointRef>;
  if (typeof candidate.path !== "string" || candidate.path.length === 0) return undefined;
  if (typeof candidate.start_byte !== "number" || !Number.isInteger(candidate.start_byte)) return undefined;
  if (typeof candidate.end_byte !== "number" || !Number.isInteger(candidate.end_byte)) return undefined;
  return { path: candidate.path, start_byte: candidate.start_byte, end_byte: candidate.end_byte };
}

function sameEndpoint(left: CompareEndpointRef, right: CompareEndpointRef): boolean {
  return (
    left.path === right.path &&
    left.start_byte === right.start_byte &&
    left.end_byte === right.end_byte
  );
}

/**
 * Opens the diff now, titled with both file names, then relabels it with the
 * engine's verdict once that arrives. The verdict never holds the diff back,
 * and one pair is measured once however often it is clicked (#607 #608).
 */
async function openCompareDiff(
  clientOf: ClientFactory, a: CompareEndpointRef, b: CompareEndpointRef,
): Promise<void> {
  const left = buildCompareUri(a, "a");
  const right = buildCompareUri(b, "b");
  const verdict = measureOnce(clientOf, a, b, `${left.toString()}\n${right.toString()}`);
  await vscode.commands.executeCommand(DIFF_COMMAND, left, right, compareTitle(a, b, undefined));
  const evidence = await verdict;
  if (evidence) await retitle(left, right, compareTitle(a, b, evidence));
}

/** The engine's verdict on the pair, asked for once while a request is in flight. */
function measureOnce(
  clientOf: ClientFactory, a: CompareEndpointRef, b: CompareEndpointRef, key: string,
): Promise<PairEvidence | undefined> {
  const inFlight = pendingVerdicts.get(key);
  if (inFlight) return inFlight;
  const measured = measurePair(clientOf(), a, b).finally(() => pendingVerdicts.delete(key));
  pendingVerdicts.set(key, measured);
  return measured;
}

/**
 * VS Code keeps an open diff's title when the same diff is opened again, so
 * the verdict replaces the tab in its own group. A diff the user has closed
 * meanwhile stays closed.
 */
async function retitle(left: vscode.Uri, right: vscode.Uri, title: string): Promise<void> {
  const open = vscode.window.tabGroups.all.flatMap((group) => group.tabs).find((tab) =>
    tab.input instanceof vscode.TabInputTextDiff &&
    tab.input.original.toString() === left.toString() &&
    tab.input.modified.toString() === right.toString());
  if (!open || open.label === title) return;
  const column = open.group.viewColumn;
  await vscode.window.tabGroups.close(open, true);
  await vscode.commands.executeCommand(DIFF_COMMAND, left, right, title, { viewColumn: column, preview: open.isPreview });
}

/** Shows `message` and records it: a compare that cannot run always says why. */
function tell(message: string): void {
  logWarn("compare: not opened", { message });
  void vscode.window.showWarningMessage(message);
}

function notInReport(row: CompareEndpointRef): string {
  return `Deslop cannot compare ${path.basename(row.path)}: that location is no longer in any duplicate group. Refresh the report and try again.`;
}

function isCanonical(row: CompareEndpointRef): string {
  return `${path.basename(row.path)} is the canonical copy of this group. Pick another location to compare with it.`;
}

function sameLocation(row: CompareEndpointRef): string {
  return `Both sides are the same location in ${path.basename(row.path)}. Pick a different second location.`;
}
