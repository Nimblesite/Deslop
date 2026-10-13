import * as assert from "node:assert/strict";
import type { ReportCluster } from "../../types/report";

const COMPARE_LINK_PATTERN = /command:deslop\.compareWithCanonical\?([^)]*)/;
const SELECTED_PEER_INDEX = 1;
const CLUSTER_ARGUMENT_INDEX = 0;
const PEER_ARGUMENT_INDEX = 1;

/** [VSIX-PAIR-COMPARE] Pin the cluster and exact peer encoded in the rendered link. */
export function assertCanonicalComparison(text: string, candidate: ReportCluster): void {
  const compare = text.match(COMPARE_LINK_PATTERN);
  assert.ok(compare, "a non-canonical occurrence offers an explicit comparison");
  const args = JSON.parse(decodeURIComponent(compare[SELECTED_PEER_INDEX] ?? "")) as unknown[];
  assert.equal(args[CLUSTER_ARGUMENT_INDEX], candidate.id, "the comparison keeps the cluster identity");
  assert.deepEqual(args[PEER_ARGUMENT_INDEX], candidate.occurrences[SELECTED_PEER_INDEX], "the comparison names the selected peer range");
}
