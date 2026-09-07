// [VSIX-PAIR-COMPARE] The native diff's title names both endpoints and the
// engine's verdict on exactly that pair. The verdict is read from the
// `deslop/pairCompare` reply; the extension calculates nothing of its own
// ([FUSED-PAIR-SIGNALS]).

import * as path from "node:path";
import type { LanguageClient } from "vscode-languageclient/node";

import { logError, logWarn } from "../logging";
import { kindTitle, type PairComparison, type PairEndpoint, type PairEvidence } from "../types/report";

/** The LSP request that measures one explicit pair. */
export const PAIR_COMPARE_METHOD = "deslop/pairCompare";
/** Verdict for two ranges that are the same bytes. */
export const IDENTICAL_BYTES_VERDICT = "Identical bytes";
/** Verdict for two ranges whose only difference is indentation. */
export const INDENTATION_ONLY_VERDICT = "Differs only by indentation";
/** Sits between the two endpoint names in a diff title. */
export const ENDPOINT_SEPARATOR = " vs ";
/** Sits between the endpoint names and the verdict. */
export const VERDICT_SEPARATOR = ": ";

const NO_CLIENT_MESSAGE = "compare: no language client, so the diff opens without an engine verdict";
const REQUEST_FAILED_CONTEXT = "compare: deslop/pairCompare";

/** The verdict the diff title shows for one measured pair. */
export function pairVerdict(evidence: PairEvidence): string {
  if (evidence.text_identity === "byte_identical") return IDENTICAL_BYTES_VERDICT;
  if (evidence.text_identity === "indentation_only") return INDENTATION_ONLY_VERDICT;
  return evidence.classification ? kindTitle(evidence.classification) : evidence.explanation;
}

/** `Alpha.cs vs Beta.cs: Nearly identical code`; with no evidence, only the names. */
export function compareTitle(left: PairEndpoint, right: PairEndpoint, evidence: PairEvidence | undefined): string {
  const names = `${path.basename(left.path)}${ENDPOINT_SEPARATOR}${path.basename(right.path)}`;
  return evidence ? `${names}${VERDICT_SEPARATOR}${pairVerdict(evidence)}` : names;
}

/** Asks the engine for the exact pair's evidence. A missing or failing engine yields no verdict, logged, and the diff still opens on the saved bytes. */
export async function measurePair(
  client: LanguageClient | undefined,
  left: PairEndpoint,
  right: PairEndpoint,
): Promise<PairEvidence | undefined> {
  if (!client) {
    logWarn(NO_CLIENT_MESSAGE);
    return undefined;
  }
  try {
    const comparison = await client.sendRequest<PairComparison>(PAIR_COMPARE_METHOD, { left, right });
    return comparison.evidence;
  } catch (err) {
    logError(err, REQUEST_FAILED_CONTEXT);
    return undefined;
  }
}
