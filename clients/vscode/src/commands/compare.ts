// [VSIX-PAIR-COMPARE] The two compare commands: a row against its current
// canonical, and two explicit endpoints. Both open the occurrence-range diff,
// titled with the engine's verdict on exactly that pair.

import * as vscode from "vscode";

import { buildCompareUri, CompareEndpointRef } from "../compare/provider";
import { compareTitle, measurePair } from "../compare/title";
import { ReportStore } from "../reportStore";
import { ClusterNode } from "../tree/providers";
import { ClientFactory, isObject, isString, occurrenceFromCommandTarget } from "./deps";

const CANONICAL_OCCURRENCE_INDEX = 0;
const FIRST_PEER_INDEX = 1;
const DIFF_COMMAND = "vscode.diff";

// [VSIX-PAIR-COMPARE] A row compares its exact range with the current canonical.
export async function compareWithCanonicalTarget(
  store: ReportStore, clientOf: ClientFactory, target: unknown, occurrence?: unknown,
): Promise<void> {
  const selected = occurrenceFromCommandTarget(occurrence ?? target);
  if (occurrence !== undefined && !selected) return;
  const clusterId = isString(target) ? target : target instanceof ClusterNode ? target.cluster.id : undefined;
  const cluster = store.current.report?.clusters.find((candidate) => clusterId
    ? candidate.id === clusterId
    : selected && candidate.occurrences.some((peer) => sameEndpoint(peer, selected)));
  const canonical = cluster?.occurrences[CANONICAL_OCCURRENCE_INDEX];
  const peer = selected ?? cluster?.occurrences[FIRST_PEER_INDEX];
  if (!cluster || !canonical || !peer) return;
  if (!cluster.occurrences.some((candidate) => sameEndpoint(candidate, peer))) return;
  await comparePairEndpoints(clientOf, canonical, peer);
}

// [VSIX-PAIR-COMPARE] Both comparison routes share the occurrence-range diff,
// titled with the engine's verdict on exactly these two endpoints.
export async function comparePairEndpoints(clientOf: ClientFactory, left: unknown, right: unknown): Promise<void> {
  const leftEndpoint = compareEndpoint(left);
  const rightEndpoint = compareEndpoint(right);
  if (!leftEndpoint || !rightEndpoint || sameEndpoint(leftEndpoint, rightEndpoint)) return;
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

async function openCompareDiff(
  clientOf: ClientFactory, a: CompareEndpointRef, b: CompareEndpointRef,
): Promise<void> {
  const evidence = await measurePair(clientOf(), a, b);
  await vscode.commands.executeCommand(
    DIFF_COMMAND,
    buildCompareUri(a, "a"),
    buildCompareUri(b, "b"),
    compareTitle(a, b, evidence),
  );
}
