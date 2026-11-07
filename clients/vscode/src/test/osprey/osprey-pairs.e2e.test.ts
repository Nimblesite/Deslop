// [VSIX-PAIR-COMPARE] [LANG-CAND-OSPREY] Pair comparison over an Osprey
// workspace: both Osprey flavors (`.osp` and the `.ospml` layout flavor)
// travel the real engine → LSP → native diff path, and every diff title
// repeats the engine's verdict on exactly the pair it shows.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";
import type { ExtensionApi } from "../../extension";
import { compareTitle, ENDPOINT_SEPARATOR, IDENTICAL_BYTES_VERDICT } from "../../compare/title";
import { kindTitle, type Report, type ReportCluster, type ReportOccurrence } from "../../types/report";
import {
  comparePairExplicitly,
  compareWithCanonicalOneClick,
  diffDocuments,
  linesWithoutIndentation,
  pairEvidence,
} from "../compare.helpers";
import { fixtureRoot, REPORT_GET_METHOD, waitFor, waitForReport } from "../suite/helpers";

const OSP_LEFT = "journal.osp";
const OSP_RIGHT = "ledger.osp";
const ML_LEFT = "describe.ospml";
const ML_RIGHT = "explain.ospml";
const HANDLER_LEFT = "drafts.osp";
const HANDLER_RIGHT = "notes.osp";
const FIXTURE_FILES = [HANDLER_LEFT, OSP_LEFT, ML_LEFT, ML_RIGHT, OSP_RIGHT, HANDLER_RIGHT];
const EXPECTED_CLUSTERS = 3;
const PAIR_SIZE = 2;
const OSPREY_LANGUAGE = "osprey";
const OSPREY_ML_LANGUAGE = "osprey-ml";
const IDENTICAL_KIND = "identical";
const LOOSELY_SIMILAR_KIND = "loosely_similar";
const BYTE_IDENTICAL_TEXT = "byte_identical";
const DIFFERENT_TEXT = "different";
const FULL_SCORE = 1;
const REPORT_TIMEOUT_MS = 30_000;
// Literals that differ between the two handler copies; the diff must show both sides of each.
const DRAFTS_ONLY_TEXT = ["saveDraft", "\"offline\""];
const NOTES_ONLY_TEXT = ["saveNote", "\"unavailable\""];

function settledReport(client: LanguageClient): Promise<Report> {
  return waitFor(async () => {
    const report = await client.sendRequest<Report>(REPORT_GET_METHOD);
    return report.clusters.length >= EXPECTED_CLUSTERS ? report : undefined;
  }, REPORT_TIMEOUT_MS);
}

function names(cluster: ReportCluster): string[] {
  return cluster.occurrences.map((occurrence) => path.basename(occurrence.path)).sort();
}

function clusterOf(report: Report, left: string, right: string): ReportCluster {
  const cluster = report.clusters.find((candidate) => names(candidate).join() === [left, right].sort().join());
  assert.ok(cluster, `a cluster pairs ${left} with ${right}: ${report.clusters.map(names).join(" | ")}`);
  return cluster;
}

// The exact bytes the engine reported, read from the fixture on disk.
function sourceOf(occurrence: ReportOccurrence): string {
  const absolute = path.isAbsolute(occurrence.path) ? occurrence.path : path.join(fixtureRoot(), occurrence.path);
  return fs.readFileSync(absolute).subarray(occurrence.start_byte, occurrence.end_byte).toString("utf8");
}

function wholeFile(name: string): string {
  return fs.readFileSync(path.join(fixtureRoot(), name), "utf8");
}

async function assertIdenticalFilesCompare(
  client: LanguageClient,
  cluster: ReportCluster,
  language: string,
): Promise<void> {
  assert.equal(cluster.kind, IDENTICAL_KIND, `${names(cluster).join()} are verbatim copies`);
  assert.equal(cluster.occurrences.length, PAIR_SIZE);
  for (const occurrence of cluster.occurrences) assert.equal(occurrence.language, language);
  const { diff, canonical, peer, evidence } = await compareWithCanonicalOneClick(client, cluster);
  assert.equal(evidence.text_identity, BYTE_IDENTICAL_TEXT, "the engine sees the same bytes");
  assert.equal(evidence.classification, IDENTICAL_KIND);
  assert.equal(evidence.admitted, true, "a verbatim copy is an admitted pair");
  assert.equal(evidence.structural, FULL_SCORE, "verbatim copies share their whole structure");
  assert.equal(evidence.token_jaccard, FULL_SCORE, "verbatim copies share every token");
  assert.ok(diff.label.endsWith(IDENTICAL_BYTES_VERDICT), `title says identical bytes: ${diff.label}`);
  const shown = await diffDocuments(diff.input);
  assert.equal(shown.original, shown.modified, "the diff shows no difference");
  assert.equal(shown.original, sourceOf(canonical), "the left side is the canonical's reported bytes");
  assert.equal(shown.modified, sourceOf(peer), "the right side is the peer's reported bytes");
  assert.equal(shown.original.trim(), wholeFile(path.basename(canonical.path)).trim(), "the clone is the whole file");
}

suite("Osprey pair comparison", () => {
  let api: ExtensionApi;
  let client: LanguageClient;
  let report: Report;

  suiteSetup(async () => {
    api = await waitForReport();
    assert.ok(api.client, "extension must expose the real LanguageClient");
    client = api.client;
    report = await settledReport(client);
  });

  test("the Osprey workspace analyses every file of both flavors and reports their three clone pairs", () => {
    assert.equal(report.files_analysed, FIXTURE_FILES.length, "every .osp and .ospml fixture is analysed");
    assert.deepEqual(report.files_skipped, [], "no Osprey file is skipped");
    assert.equal(report.clusters.length, EXPECTED_CLUSTERS, `clusters: ${report.clusters.map(names).join(" | ")}`);
    assert.equal(clusterOf(report, OSP_LEFT, OSP_RIGHT).kind, IDENTICAL_KIND);
    assert.equal(clusterOf(report, ML_LEFT, ML_RIGHT).kind, IDENTICAL_KIND);
    assert.equal(clusterOf(report, HANDLER_LEFT, HANDLER_RIGHT).kind, LOOSELY_SIMILAR_KIND);
    const reported = report.clusters.flatMap(names).sort();
    assert.deepEqual(reported, [...FIXTURE_FILES].sort(), "each fixture file sits in exactly one pair");
  });

  test("one click compares identical .osp copies and the diff shows the same bytes", async () => {
    await assertIdenticalFilesCompare(client, clusterOf(report, OSP_LEFT, OSP_RIGHT), OSPREY_LANGUAGE);
  });

  test("one click compares identical layout-flavor .ospml copies and the diff shows the same bytes", async () => {
    await assertIdenticalFilesCompare(client, clusterOf(report, ML_LEFT, ML_RIGHT), OSPREY_ML_LANGUAGE);
  });

  test("comparing two renamed effect handlers shows both reported ranges under the engine's verdict", async () => {
    const cluster = clusterOf(report, HANDLER_LEFT, HANDLER_RIGHT);
    const [left, right] = cluster.occurrences;
    assert.ok(left && right, "the handler cluster exposes two endpoints");
    for (const occurrence of cluster.occurrences) assert.equal(occurrence.language, OSPREY_LANGUAGE);
    const diff = await comparePairExplicitly(left, right);
    const evidence = await pairEvidence(client, left, right);
    assert.equal(diff.label, compareTitle(left, right, evidence), "the title is the engine's verdict on this pair");
    assert.ok(diff.label.endsWith(kindTitle(LOOSELY_SIMILAR_KIND)), `title names the pair's kind: ${diff.label}`);
    assert.equal(evidence.text_identity, DIFFERENT_TEXT, "renamed handlers differ by more than whitespace");
    assert.equal(evidence.classification, cluster.kind, "the pair verdict agrees with the cluster's kind");
    assert.equal(evidence.admitted, true, "the reported pair is admitted by the engine");
    assert.ok(evidence.structural > 0 && evidence.structural < FULL_SCORE, `partial structure: ${evidence.structural}`);
    assert.ok(evidence.token_jaccard > 0 && evidence.token_jaccard < FULL_SCORE, `partial tokens: ${evidence.token_jaccard}`);
    const shown = await diffDocuments(diff.input);
    assert.equal(shown.original, sourceOf(left), "the left side is exactly the left reported range");
    assert.equal(shown.modified, sourceOf(right), "the right side is exactly the right reported range");
    assert.notDeepEqual(linesWithoutIndentation(shown.original), linesWithoutIndentation(shown.modified));
    const [draftsSide, notesSide] = path.basename(left.path) === HANDLER_LEFT ? [shown.original, shown.modified] : [shown.modified, shown.original];
    for (const text of DRAFTS_ONLY_TEXT) assert.ok(draftsSide.includes(text) && !notesSide.includes(text), `only drafts shows ${text}`);
    for (const text of NOTES_ONLY_TEXT) assert.ok(notesSide.includes(text) && !draftsSide.includes(text), `only notes shows ${text}`);
  });

  test("reversing the handler pair swaps the sides and keeps the engine's verdict", async () => {
    const [left, right] = clusterOf(report, HANDLER_LEFT, HANDLER_RIGHT).occurrences;
    assert.ok(left && right, "the handler cluster exposes two endpoints");
    const forward = await pairEvidence(client, left, right);
    const diff = await comparePairExplicitly(right, left);
    const reversed = await pairEvidence(client, right, left);
    assert.equal(reversed.classification, forward.classification, "the verdict does not depend on order");
    assert.equal(reversed.text_identity, forward.text_identity);
    assert.equal(reversed.admitted, forward.admitted);
    assert.equal(reversed.structural, forward.structural, "structure is symmetric");
    assert.equal(reversed.token_jaccard, forward.token_jaccard, "token overlap is symmetric");
    assert.equal(diff.label, compareTitle(right, left, reversed));
    assert.ok(
      diff.label.startsWith(`${path.basename(right.path)}${ENDPOINT_SEPARATOR}${path.basename(left.path)}`),
      `the reversed title leads with the right-hand file: ${diff.label}`,
    );
    const shown = await diffDocuments(diff.input);
    assert.equal(shown.original, sourceOf(right));
    assert.equal(shown.modified, sourceOf(left));
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });
});
