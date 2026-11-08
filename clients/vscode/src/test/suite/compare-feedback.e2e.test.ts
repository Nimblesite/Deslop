// [VSIX-PAIR-COMPARE] #607 #608: no compare click is ever a silent no-op. Every
// path that cannot open a diff tells the user why, and every row the user can
// see compares — even when the cluster id it was drawn with has moved on.
//
// Drives the real extension against the real deslop-lsp. The only stand-in is
// a recorder on `vscode.window.showWarningMessage`, so the test reads exactly
// the message the user would see.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import type { ExtensionApi } from "../../extension";
import type { ReportCluster, ReportOccurrence } from "../../types/report";
import {
  assertDiffEndpoints,
  CLOSE_ALL_EDITORS,
  COMPARE_PAIR,
  COMPARE_WITH_CANONICAL,
  waitForDiff,
} from "../compare.helpers";
import { openDiffTabs } from "../verdict-hold.helpers";
import { waitForClusterHolding, waitForReport } from "./helpers";

const ALPHA_FILE = "Alpha.cs";
const GAMMA_FILE = "Gamma.cs";
const CANONICAL_INDEX = 0;
const FIRST_PEER_INDEX = 1;
const NO_DIFF_TABS = 0;
const ONE_MESSAGE = 1;
/** A byte range the fixture file has, which no cluster reports. */
const UNREPORTED_START = 0;
const UNREPORTED_END = 3;

type ShowWarning = typeof vscode.window.showWarningMessage;

/** Runs `command` with the warning popup recorded, and returns every message shown. */
async function warningsFrom(command: string, ...args: unknown[]): Promise<string[]> {
  await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
  const windowApi = vscode.window as unknown as { showWarningMessage: ShowWarning };
  const original = windowApi.showWarningMessage;
  const shown: string[] = [];
  const record = (message: string): Promise<undefined> => {
    shown.push(message);
    return Promise.resolve(undefined);
  };
  windowApi.showWarningMessage = record;
  try {
    await vscode.commands.executeCommand(command, ...args);
  } finally {
    windowApi.showWarningMessage = original;
  }
  return shown;
}

/** The command opened nothing and told the user, once, in a sentence naming `mentions`. */
function assertToldWhy(shown: string[], what: string, mentions: readonly string[]): void {
  assert.equal(openDiffTabs(), NO_DIFF_TABS, `${what}: no diff opens`);
  assert.equal(shown.length, ONE_MESSAGE, `${what}: the user is told why, exactly once; shown ${JSON.stringify(shown)}`);
  const [message] = shown;
  assert.ok(message && message.trim().length > 0, `${what}: the message says something`);
  for (const word of mentions) assert.ok(message.includes(word), `${what}: the message names ${word}: ${message}`);
}

function canonicalOf(cluster: ReportCluster): ReportOccurrence {
  const canonical = cluster.occurrences[CANONICAL_INDEX];
  assert.ok(canonical, `cluster ${cluster.id} has a canonical occurrence`);
  return canonical;
}

function firstPeerOf(cluster: ReportCluster): ReportOccurrence {
  const peer = cluster.occurrences[FIRST_PEER_INDEX];
  assert.ok(peer, `cluster ${cluster.id} has a peer`);
  return peer;
}

suite("compare feedback (#607 #608)", () => {
  let api: ExtensionApi;
  let renamed: ReportCluster;
  let reindented: ReportCluster;

  suiteSetup(async () => {
    api = await waitForReport();
    assert.ok(api.client, "extension must expose the real LanguageClient");
    renamed = await waitForClusterHolding(api.client, ALPHA_FILE);
    reindented = await waitForClusterHolding(api.client, GAMMA_FILE);
    assert.notEqual(renamed.id, reindented.id, "the fixture clusters are distinct");
  });

  test("Compare To Canonical on a row no cluster reports tells the user, with or without a cluster id (#607)", async () => {
    const canonical = canonicalOf(renamed);
    const unreported: ReportOccurrence = { ...canonical, start_byte: UNREPORTED_START, end_byte: UNREPORTED_END };
    for (const [how, args] of [
      ["with the cluster's id", [renamed.id, unreported]],
      ["with a superseded id", ["0000000000000000", unreported]],
      ["as a bare row", [unreported]],
    ] as const) {
      const shown = await warningsFrom(COMPARE_WITH_CANONICAL, ...args);
      assertToldWhy(shown, how, [path.basename(canonical.path)]);
    }
  });

  test("Compare To Canonical on the canonical row itself tells the user it is the canonical (#607)", async () => {
    for (const cluster of [renamed, reindented]) {
      const canonical = canonicalOf(cluster);
      for (const args of [[cluster.id, canonical], [canonical]]) {
        const shown = await warningsFrom(COMPARE_WITH_CANONICAL, ...args);
        assertToldWhy(shown, `${cluster.id} canonical`, [path.basename(canonical.path)]);
      }
    }
  });

  test("Compare To Canonical with a malformed row tells the user instead of doing nothing (#607)", async () => {
    const canonical = canonicalOf(renamed);
    const malformed: Array<[string, unknown]> = [
      ["a row with no path", { ...canonical, path: "" }],
      ["a row with a fractional start", { ...canonical, start_byte: 1.5 }],
      ["a row with no end", { path: canonical.path, start_byte: canonical.start_byte }],
      ["a string instead of a row", canonical.path],
    ];
    for (const [how, row] of malformed) {
      const shown = await warningsFrom(COMPARE_WITH_CANONICAL, renamed.id, row);
      assertToldWhy(shown, how, []);
    }
  });

  test("Compare To Canonical with another cluster's id compares the row against its own canonical (#607)", async () => {
    for (const [drawnWith, owner] of [[renamed, reindented], [reindented, renamed]] as const) {
      const peer = firstPeerOf(owner);
      await vscode.commands.executeCommand(CLOSE_ALL_EDITORS);
      await vscode.commands.executeCommand(COMPARE_WITH_CANONICAL, drawnWith.id, peer);
      const diff = await waitForDiff();
      assertDiffEndpoints(diff.input, canonicalOf(owner), peer);
    }
  });

  test("Compare with Selected on one row twice tells the user both sides are the same (#608)", async () => {
    for (const cluster of [renamed, reindented]) {
      for (const row of cluster.occurrences) {
        const shown = await warningsFrom(COMPARE_PAIR, row, { ...row });
        assertToldWhy(shown, `${cluster.id} ${row.path}`, [path.basename(row.path)]);
      }
    }
  });

  test("Compare with Selected with a missing or malformed side tells the user (#608)", async () => {
    const left = canonicalOf(renamed);
    const right = firstPeerOf(renamed);
    const broken: Array<[string, unknown, unknown]> = [
      ["no right side", left, undefined],
      ["no left side", undefined, right],
      ["a right side with no path", left, { ...right, path: "" }],
      ["a left side with a negative fraction", { ...left, start_byte: -0.5 }, right],
      ["a right side that is a string", left, right.path],
      ["two empty objects", {}, {}],
    ];
    for (const [how, first, second] of broken) {
      const shown = await warningsFrom(COMPARE_PAIR, first, second);
      assertToldWhy(shown, how, []);
    }
  });
});
