// [VSIX-PAIR-COMPARE] Holds the real client's `deslop/pairCompare` replies so a
// test can see what the user sees while the engine is still measuring a pair.
// The real deslop-lsp answers every request; only delivery is held.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import { PAIR_COMPARE_METHOD } from "../compare/title";
import { CLOSE_ALL_EDITORS, waitForDiff } from "./compare.helpers";

/** One compare asks the engine for exactly one verdict. */
export const ONE_VERDICT = 1;
const NO_VERDICTS = 0;
const HELD_FAILURE = "pair compare failed while held";

/** The real client's pair-compare replies, held until released. */
export interface HeldVerdicts {
  /** How many verdicts the extension has asked for since holding began. */
  readonly count: () => number;
  /** Delivers every held reply and restores the client. */
  readonly release: () => void;
}

/** Options for a hold: `fail` delivers each held reply as a rejection. */
export interface HoldOptions {
  readonly fail?: boolean;
}

type SendRequest = (method: string, params: unknown) => Promise<unknown>;

/** Holds every `deslop/pairCompare` reply on `client` until `release`. */
export function holdPairVerdicts(client: LanguageClient, options: HoldOptions = {}): HeldVerdicts {
  const target = client as unknown as { sendRequest: SendRequest };
  const original: SendRequest = target.sendRequest.bind(client);
  const waiting: Array<() => void> = [];
  let asked = NO_VERDICTS;
  let released = false;
  target.sendRequest = async (method: string, params: unknown) => {
    if (method !== PAIR_COMPARE_METHOD) return await original(method, params);
    asked += 1;
    const reply = await original(method, params);
    if (!released) {
      await new Promise<void>((resolve) => {
        waiting.push(resolve);
      });
    }
    if (options.fail) throw new Error(HELD_FAILURE);
    return reply;
  };
  return {
    count: () => asked,
    release: () => {
      released = true;
      target.sendRequest = original;
      waiting.splice(0).forEach((resolve) => resolve());
    },
  };
}

/** How many diff tabs are open across every editor group. */
export function openDiffTabs(): number {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .filter((tab) => tab.input instanceof vscode.TabInputTextDiff).length;
}

/** Runs `command` with its verdicts held and returns the diff it opened while they were held. */
export async function diffOpenedWhileVerdictPending(
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
      ONE_VERDICT,
      `${command}: the engine was asked for exactly one verdict, still undelivered when the diff opened`,
    );
    return diff.input;
  } finally {
    verdicts.release();
    await running;
  }
}
