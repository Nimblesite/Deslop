// [VSIX-PAIR-COMPARE] #607 #608: a compare opens its diff the moment the user
// asks for it. The engine's verdict only labels the diff, so it may never hold
// the diff back, and a report that moved on since the panel was drawn may never
// turn the click into a silent no-op.
//
// Drives the real extension against the real deslop-lsp. The real client's
// `deslop/pairCompare` replies are held — the engine still answers, its answer
// is just not delivered yet — which is exactly what a user sees on a large
// workspace while the engine measures the pair.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import type { ExtensionApi } from "../../extension";
import { PAIR_COMPARE_METHOD } from "../../compare/title";
import type { ReportCluster } from "../../types/report";
import {
  assertDiffEndpoints,
  CLOSE_ALL_EDITORS,
  COMPARE_PAIR,
  COMPARE_WITH_CANONICAL,
  waitForDiff,
} from "../compare.helpers";
import { waitForClusterHolding, waitForReport } from "./helpers";

const ALPHA_FILE = "Alpha.cs";
/** A cluster id no report holds: the id the panel captured before the report moved on. */
const SUPERSEDED_CLUSTER_ID = "0000000000000000";
const NO_HELD_REQUESTS = 0;
const ONE_HELD_REQUEST = 1;

/** The real client's pair-compare replies, held until released. */
interface HeldVerdicts {
  readonly count: () => number;
  readonly release: () => void;
}

/** Holds every `deslop/pairCompare` reply on `client` until `release`. */
function holdPairVerdicts(client: LanguageClient): HeldVerdicts {
  const original = client.sendRequest.bind(client) as (method: string, params: unknown) => Promise<unknown>;
  const waiting: Array<() => void> = [];
  let held = NO_HELD_REQUESTS;
  let released = false;
  (client as unknown as { sendRequest: unknown }).sendRequest = async (method: string, params: unknown) => {
    const reply = await original(method, params);
    if (method !== PAIR_COMPARE_METHOD || released) return reply;
    held += 1;
    await new Promise<void>((resolve) => waiting.push(resolve));
    return reply;
  };
  return {
    count: () => held,
    release: () => {
      released = true;
      (client as unknown as { sendRequest: unknown }).sendRequest = original;
      waiting.splice(0).forEach((resolve) => resolve());
    },
  };
}

/** Runs `command` with its verdicts held and returns the diff it opened while they were held. */
async function diffOpenedWhileVerdictPending(
  client: LanguageClient,
  command: string,
  ...args: unknown[]
): Promise<vscode.TabInputTextDiff> {
  await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
  const verdicts = holdPairVerdicts(client);
  const running = vscode.commands.executeCommand(command, ...args);
  try {
    const diff = await waitForDiff();
    assert.equal(
      verdicts.count(),
      ONE_HELD_REQUEST,
      `${command}: the engine was asked for exactly one verdict, still undelivered when the diff opened`,
    );
    return diff.input;
  } finally {
    verdicts.release();
    await running;
  }
}

suite("compare responsiveness (#607 #608)", () => {
  let api: ExtensionApi;
  let cluster: ReportCluster;

  suiteSetup(async () => {
    api = await waitForReport();
    assert.ok(api.client, "extension must expose the real LanguageClient");
    cluster = await waitForClusterHolding(api.client, ALPHA_FILE);
    assert.ok(cluster.occurrences.length >= 2, "the fixture cluster has a canonical and a peer");
  });

  test("Compare To Canonical opens the diff while the engine's verdict is still pending (#607)", async () => {
    const [canonical, peer] = cluster.occurrences;
    assert.ok(api.client && canonical && peer);
    const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_WITH_CANONICAL, cluster.id, peer);
    assertDiffEndpoints(diff, canonical, peer);
  });

  test("Compare with Selected opens the diff of both picked rows while the verdict is still pending (#608)", async () => {
    const [left, right] = cluster.occurrences;
    assert.ok(api.client && left && right);
    const diff = await diffOpenedWhileVerdictPending(api.client, COMPARE_PAIR, left, right);
    assertDiffEndpoints(diff, left, right);
    const reversed = await diffOpenedWhileVerdictPending(api.client, COMPARE_PAIR, right, left);
    assertDiffEndpoints(reversed, right, left);
  });

  test("Compare To Canonical on a row whose cluster id the report has since replaced still compares that row (#607)", async () => {
    const [canonical, peer] = cluster.occurrences;
    assert.ok(canonical && peer);
    await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
    await vscode.commands.executeCommand(COMPARE_WITH_CANONICAL, SUPERSEDED_CLUSTER_ID, peer);
    const diff = await waitForDiff();
    assertDiffEndpoints(diff.input, canonical, peer);
  });
});
