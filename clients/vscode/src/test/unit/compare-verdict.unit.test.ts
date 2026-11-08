// [VSIX-PAIR-COMPARE] #607 #608 in isolation: the diff opens before the
// engine's verdict, the verdict re-titles that one tab, one pair is measured
// once however often it is clicked, a failing or absent engine never loses the
// diff, and every path that cannot compare tells the user why. The real-LSP
// path is driven end to end in suite/compare-*.e2e.test.ts; this suite scripts
// the verdict so each branch is reached deterministically.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { comparePairEndpoints, compareWithCanonicalTarget } from "../../commands/compare";
import { parseCompareUri } from "../../compare/provider";
import { compareTitle, PAIR_COMPARE_METHOD } from "../../compare/title";
import { ReportStore } from "../../reportStore";
import type { PairEvidence, ReportCluster, ReportOccurrence } from "../../types/report";
import { occurrence, wireCluster } from "../cluster.helpers";
import { activateExtension } from "../suite/helpers";
import { reportWithClusters } from "./report.helpers";
import { storeWith } from "./report-store.helpers";
import { tempFile } from "./temp-file.helpers";

const FIXTURE_PREFIX = "deslop-verdict-";
const FIXTURE_FILENAME = "verdict.ts";
const CLUSTER_ID = "verdict-cluster";
const LONE_CLUSTER_ID = "lone-cluster";
const CLOSE_EDITORS = "workbench.action.closeAllEditors";
const UTF8 = "utf8";
const SOURCE_PARTS = ["const alpha = 1;\n", "const beta = 2;\n", "const gamma = 3;\n"];
const CANONICAL_INDEX = 0;
const FIRST_PEER_INDEX = 1;
const SECOND_PEER_INDEX = 2;
const NO_DIFFS = 0;
const ONE_DIFF = 1;
const ONE_REQUEST = 1;
const TWO_REQUESTS = 2;
const NO_MESSAGES = 0;
const ONE_MESSAGE = 1;
const CONCURRENT_CLICKS = 4;
const VERDICTS: readonly PairEvidence[] = [
  { text_identity: "byte_identical", classification: "identical", explanation: "same bytes" } as PairEvidence,
  { text_identity: "indentation_only", classification: "identical", explanation: "indentation" } as PairEvidence,
  { text_identity: "different", classification: "nearly_identical", explanation: "renamed" } as PairEvidence,
];

/** A language client whose pair-compare replies the test decides. */
interface ScriptedClient {
  readonly client: LanguageClient;
  readonly requests: () => number;
  /** Delivers every pending reply. */
  readonly answer: (evidence: PairEvidence) => void;
  /** Fails every pending reply. */
  readonly fail: () => void;
}

function scriptedClient(): ScriptedClient {
  const pending: Array<{ resolve: (value: unknown) => void; reject: (error: Error) => void }> = [];
  let asked = 0;
  const sendRequest = (method: string): Promise<unknown> => {
    assert.equal(method, PAIR_COMPARE_METHOD, "compare asks only for the pair verdict");
    asked += 1;
    return new Promise((resolve, reject) => {
      pending.push({ resolve, reject });
    });
  };
  return {
    client: { sendRequest } as unknown as LanguageClient,
    requests: () => asked,
    answer: (evidence) => pending.splice(0).forEach(({ resolve }) => resolve({ evidence })),
    fail: () => pending.splice(0).forEach(({ reject }) => reject(new Error("engine unavailable"))),
  };
}

interface Fixture {
  readonly store: ReportStore;
  readonly cluster: ReportCluster;
  readonly lone: ReportCluster;
}

function rangesIn(file: string): ReportOccurrence[] {
  let start = 0;
  return SOURCE_PARTS.map((text) => {
    const end = start + Buffer.byteLength(text, UTF8);
    const range = occurrence(file, start, end);
    start = end;
    return range;
  });
}

async function withFixture(run: (fixture: Fixture) => Promise<void>): Promise<void> {
  const { dir, file } = tempFile(FIXTURE_PREFIX, FIXTURE_FILENAME);
  fs.writeFileSync(file, SOURCE_PARTS.join(""), UTF8);
  const ranges = rangesIn(file);
  const cluster = wireCluster({ id: CLUSTER_ID, occurrences: ranges });
  const lone = wireCluster({ id: LONE_CLUSTER_ID, occurrences: ranges.slice(CANONICAL_INDEX, FIRST_PEER_INDEX) });
  const store = storeWith(reportWithClusters([cluster, lone]), CANONICAL_INDEX);
  try {
    await vscode.commands.executeCommand(CLOSE_EDITORS);
    await run({ store, cluster, lone });
  } finally {
    await vscode.commands.executeCommand(CLOSE_EDITORS);
    store.dispose();
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function diffTabs(): vscode.Tab[] {
  return vscode.window.tabGroups.all.flatMap((group) => group.tabs)
    .filter((tab) => tab.input instanceof vscode.TabInputTextDiff);
}

/** The one open diff compares `left` with `right` under `title`. */
function assertOneDiff(left: ReportOccurrence, right: ReportOccurrence, title: string): void {
  const tabs = diffTabs();
  assert.equal(tabs.length, ONE_DIFF, "exactly one diff is open");
  const [tab] = tabs;
  assert.ok(tab && tab.input instanceof vscode.TabInputTextDiff);
  assert.equal(tab.label, title);
  const original = parseCompareUri(tab.input.original);
  const modified = parseCompareUri(tab.input.modified);
  assert.deepEqual([original.sourcePath, original.startByte, original.endByte], [left.path, left.start_byte, left.end_byte]);
  assert.deepEqual([modified.sourcePath, modified.startByte, modified.endByte], [right.path, right.start_byte, right.end_byte]);
}

/** Every warning `run` shows the user. */
async function warningsDuring(run: () => Promise<void>): Promise<string[]> {
  const windowApi = vscode.window as unknown as { showWarningMessage: (message: string) => Thenable<undefined> };
  const original = windowApi.showWarningMessage;
  const shown: string[] = [];
  windowApi.showWarningMessage = (message: string) => {
    shown.push(message);
    return Promise.resolve(undefined);
  };
  try {
    await run();
  } finally {
    windowApi.showWarningMessage = original;
  }
  return shown;
}

function rangeAt(cluster: ReportCluster, index: number): ReportOccurrence {
  const range = cluster.occurrences[index];
  assert.ok(range, `the fixture has a range at ${index}`);
  return range;
}

suite("compare verdict (#607 #608)", () => {
  suiteSetup(async () => {
    await activateExtension();
  });

  test("each verdict re-titles the diff that opened before it, for both compare routes", async () => {
    await withFixture(async ({ store, cluster }) => {
      const canonical = rangeAt(cluster, CANONICAL_INDEX);
      for (const peerIndex of [FIRST_PEER_INDEX, SECOND_PEER_INDEX]) {
        const peer = rangeAt(cluster, peerIndex);
        for (const evidence of VERDICTS) {
          for (const route of ["canonical", "pair"] as const) {
            await vscode.commands.executeCommand(CLOSE_EDITORS);
            const engine = scriptedClient();
            const running = route === "canonical"
              ? compareWithCanonicalTarget(store, () => engine.client, cluster.id, peer)
              : comparePairEndpoints(() => engine.client, canonical, peer);
            await waitForDiffCount(ONE_DIFF);
            assertOneDiff(canonical, peer, compareTitle(canonical, peer, undefined));
            assert.equal(engine.requests(), ONE_REQUEST, `${route}: one verdict requested`);
            engine.answer(evidence);
            await running;
            assertOneDiff(canonical, peer, compareTitle(canonical, peer, evidence));
          }
        }
      }
    });
  });

  test("concurrent clicks on one pair measure it once and leave one diff", async () => {
    await withFixture(async ({ store, cluster }) => {
      const canonical = rangeAt(cluster, CANONICAL_INDEX);
      const peer = rangeAt(cluster, FIRST_PEER_INDEX);
      const engine = scriptedClient();
      const clicks = Array.from({ length: CONCURRENT_CLICKS }, (_, index) => index % 2 === 0
        ? compareWithCanonicalTarget(store, () => engine.client, cluster.id, peer)
        : comparePairEndpoints(() => engine.client, canonical, peer));
      await waitForDiffCount(ONE_DIFF);
      assert.equal(engine.requests(), ONE_REQUEST, "one pair is measured once while in flight");
      const [evidence] = VERDICTS;
      assert.ok(evidence);
      engine.answer(evidence);
      await Promise.all(clicks);
      assertOneDiff(canonical, peer, compareTitle(canonical, peer, evidence));

      // Once answered, the next click measures again: verdicts are not cached.
      const again = comparePairEndpoints(() => engine.client, canonical, peer);
      await waitForRequests(engine, TWO_REQUESTS);
      engine.answer(evidence);
      await again;
      assertOneDiff(canonical, peer, compareTitle(canonical, peer, evidence));
    });
  });

  test("a failing or absent engine leaves the diff open under both file names", async () => {
    await withFixture(async ({ store, cluster }) => {
      const canonical = rangeAt(cluster, CANONICAL_INDEX);
      const peer = rangeAt(cluster, FIRST_PEER_INDEX);
      const engine = scriptedClient();
      const failing = compareWithCanonicalTarget(store, () => engine.client, cluster.id, peer);
      await waitForDiffCount(ONE_DIFF);
      engine.fail();
      await failing;
      assertOneDiff(canonical, peer, compareTitle(canonical, peer, undefined));

      await vscode.commands.executeCommand(CLOSE_EDITORS);
      await comparePairEndpoints(() => undefined, peer, canonical);
      assertOneDiff(peer, canonical, compareTitle(peer, canonical, undefined));
    });
  });

  test("a diff the user closed before the verdict stays closed", async () => {
    await withFixture(async ({ store, cluster }) => {
      const peer = rangeAt(cluster, FIRST_PEER_INDEX);
      const engine = scriptedClient();
      const running = compareWithCanonicalTarget(store, () => engine.client, cluster.id, peer);
      await waitForDiffCount(ONE_DIFF);
      await vscode.commands.executeCommand(CLOSE_EDITORS);
      const [evidence] = VERDICTS;
      assert.ok(evidence);
      engine.answer(evidence);
      await running;
      assert.equal(diffTabs().length, NO_DIFFS, "the verdict never reopens a closed diff");
    });
  });

  test("every compare that cannot run says why and opens nothing", async () => {
    await withFixture(async ({ store, cluster, lone }) => {
      const canonical = rangeAt(cluster, CANONICAL_INDEX);
      const engine = scriptedClient();
      const cases: Array<[string, () => Promise<void>]> = [
        ["a cluster with no second location", () => compareWithCanonicalTarget(store, () => engine.client, { cluster: lone })],
        ["a cluster row with no locations list", () => compareWithCanonicalTarget(store, () => engine.client, { cluster: { id: CLUSTER_ID } })],
        ["a cluster row that is not a cluster", () => compareWithCanonicalTarget(store, () => engine.client, { cluster: CLUSTER_ID })],
        ["an id no report holds and no row", () => compareWithCanonicalTarget(store, () => engine.client, "missing-cluster")],
        ["an empty report", () => compareWithCanonicalTarget(new ReportStore(), () => engine.client, CLUSTER_ID, canonical)],
        ["the canonical against itself", () => compareWithCanonicalTarget(store, () => engine.client, CLUSTER_ID, canonical)],
        ["a fractional end", () => comparePairEndpoints(() => engine.client, canonical, { ...canonical, end_byte: 2.5 })],
        ["an end that is text", () => comparePairEndpoints(() => engine.client, canonical, { ...canonical, end_byte: "9" })],
        ["the same location twice", () => comparePairEndpoints(() => engine.client, canonical, { ...canonical })],
      ];
      for (const [what, run] of cases) {
        const shown = await warningsDuring(run);
        assert.equal(diffTabs().length, NO_DIFFS, `${what}: no diff opens`);
        assert.equal(shown.length, ONE_MESSAGE, `${what}: the user is told why, once: ${JSON.stringify(shown)}`);
        const [message = ""] = shown;
        assert.ok(message.startsWith("Deslop") || message.includes(FIXTURE_FILENAME), `${what}: ${message}`);
      }
      assert.equal(engine.requests(), NO_MESSAGES, "nothing that cannot compare asks the engine");
    });
  });
});

async function waitForDiffCount(count: number): Promise<void> {
  await until(() => diffTabs().length === count, `expected ${count} diff tab(s), found ${diffTabs().length}`);
}

async function waitForRequests(engine: ScriptedClient, count: number): Promise<void> {
  await until(() => engine.requests() === count, `expected ${count} verdict request(s), found ${engine.requests()}`);
}

/** Polls a condition VS Code settles asynchronously (tab model updates). */
async function until(condition: () => boolean, failure: string): Promise<void> {
  const ATTEMPTS = 100;
  const INTERVAL_MS = 50;
  for (let attempt = 0; attempt < ATTEMPTS; attempt += 1) {
    if (condition()) return;
    await new Promise<void>((resolve) => {
      setTimeout(resolve, INTERVAL_MS);
    });
  }
  assert.fail(failure);
}
