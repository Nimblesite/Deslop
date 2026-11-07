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
import { kindTitle, type PairEndpoint, type PairEvidence, type Report, type ReportCluster } from "../../types/report";
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
const CLONED_FILES = [OSP_LEFT, OSP_RIGHT, ML_LEFT, ML_RIGHT];
const EXPECTED_CLUSTERS = 2;
const PAIR_SIZE = 2;
const OSPREY_LANGUAGE = "osprey";
const OSPREY_ML_LANGUAGE = "osprey-ml";
const IDENTICAL_KIND = "identical";
const CLONE_KINDS = ["identical", "nearly_identical", "structural_only", "loosely_similar", "same_behavior"] as const;
const BYTE_IDENTICAL_TEXT = "byte_identical";
const DIFFERENT_TEXT = "different";
const CONTENT_REJECTION = "rejected: pair fails content corroboration";
const FULL_SCORE = 1;
const REPORT_TIMEOUT_MS = 30_000;
// The handler files' retained extents: drafts' `//!` line comment is not part of
// its first declaration (byte 51 on), notes' `///` doc comment is (byte 0 on).
const DRAFTS_WHOLE: PairEndpoint = { path: HANDLER_LEFT, start_byte: 51, end_byte: 380 };
const NOTES_WHOLE: PairEndpoint = { path: HANDLER_RIGHT, start_byte: 0, end_byte: 393 };
// notes' effect + function, without its `let disconnected = handler …`.
const NOTES_WITHOUT_HANDLER: PairEndpoint = { path: HANDLER_RIGHT, start_byte: 0, end_byte: 298 };
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
function sourceOf(occurrence: PairEndpoint): string {
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

function assertRefused(evidence: PairEvidence, what: string): void {
  assert.equal(evidence.admitted, false, `${what} is not admitted: ${evidence.explanation}`);
  assert.ok(!evidence.classification, `${what} carries no clone kind: ${String(evidence.classification)}`);
  assert.equal(evidence.text_identity, DIFFERENT_TEXT, `${what} differ by more than whitespace`);
  assert.equal(evidence.explanation, CONTENT_REJECTION, `${what} fails content corroboration`);
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

  test("the Osprey workspace analyses every file of both flavors and reports only its two verbatim copies", () => {
    assert.equal(report.files_analysed, FIXTURE_FILES.length, "every .osp and .ospml fixture is analysed");
    assert.deepEqual(report.files_skipped, [], "no Osprey file is skipped");
    assert.equal(report.clusters.length, EXPECTED_CLUSTERS, `clusters: ${report.clusters.map(names).join(" | ")}`);
    assert.equal(clusterOf(report, OSP_LEFT, OSP_RIGHT).kind, IDENTICAL_KIND);
    assert.equal(clusterOf(report, ML_LEFT, ML_RIGHT).kind, IDENTICAL_KIND);
    assert.deepEqual(report.clusters.flatMap(names).sort(), [...CLONED_FILES].sort(), "each copy sits in exactly one pair");
    // [FUSED-CONTENT-GATE-RENAME] The handlers keep fewer names than they rename
    // and preserve no literal: a different vocabulary over one shape, not a copy.
    for (const cluster of report.clusters) {
      for (const handler of [HANDLER_LEFT, HANDLER_RIGHT]) {
        assert.ok(!names(cluster).includes(handler), `${handler} is in no cluster: ${names(cluster).join()}`);
      }
    }
  });

  test("one click compares identical .osp copies and the diff shows the same bytes", async () => {
    await assertIdenticalFilesCompare(client, clusterOf(report, OSP_LEFT, OSP_RIGHT), OSPREY_LANGUAGE);
  });

  test("one click compares identical layout-flavor .ospml copies and the diff shows the same bytes", async () => {
    await assertIdenticalFilesCompare(client, clusterOf(report, ML_LEFT, ML_RIGHT), OSPREY_ML_LANGUAGE);
  });

  test("comparing the two renamed effect handlers whole shows both files under the engine's refusal", async () => {
    const diff = await comparePairExplicitly(DRAFTS_WHOLE, NOTES_WHOLE);
    const evidence = await pairEvidence(client, DRAFTS_WHOLE, NOTES_WHOLE);
    assertRefused(evidence, "the whole handlers");
    assert.equal(evidence.structural, FULL_SCORE, "the handlers share their whole normalised shape");
    assert.equal(evidence.token_jaccard, FULL_SCORE, "the handlers share every normalised token");
    assert.equal(evidence.rename_consistency, 0, "no literal survives, so the rename axis is zero (#134 stride family)");
    assert.equal(diff.label, compareTitle(DRAFTS_WHOLE, NOTES_WHOLE, evidence), "the title is the engine's verdict");
    assert.ok(diff.label.endsWith(CONTENT_REJECTION), `title carries the refusal: ${diff.label}`);
    for (const kind of CLONE_KINDS) assert.ok(!diff.label.endsWith(kindTitle(kind)), `no clone kind claimed: ${diff.label}`);
    const shown = await diffDocuments(diff.input);
    assert.equal(shown.original, sourceOf(DRAFTS_WHOLE), "the left side is drafts' whole retained extent");
    assert.equal(shown.modified, sourceOf(NOTES_WHOLE), "the right side is notes' whole retained extent");
    assert.notDeepEqual(linesWithoutIndentation(shown.original), linesWithoutIndentation(shown.modified));
    for (const text of DRAFTS_ONLY_TEXT) assert.ok(shown.original.includes(text) && !shown.modified.includes(text), `only drafts shows ${text}`);
    for (const text of NOTES_ONLY_TEXT) assert.ok(shown.modified.includes(text) && !shown.original.includes(text), `only notes shows ${text}`);
  });

  test("one whole handler file against the other's effect and function is refused in both orders", async () => {
    // [FUSED-SHARED-SUBTREE-CORE] A lopsided view's aligned core is the refused
    // whole pair minus a declaration; if the core is not a copy the wider view
    // cannot be one (`cluster_extent_alignment`: one function against two).
    const forward = await pairEvidence(client, DRAFTS_WHOLE, NOTES_WITHOUT_HANDLER);
    assertRefused(forward, "drafts whole against notes without its handler");
    const diff = await comparePairExplicitly(NOTES_WITHOUT_HANDLER, DRAFTS_WHOLE);
    const reversed = await pairEvidence(client, NOTES_WITHOUT_HANDLER, DRAFTS_WHOLE);
    assertRefused(reversed, "the same lopsided pair reversed");
    assert.equal(reversed.structural, forward.structural, "structure is symmetric");
    assert.equal(reversed.token_jaccard, forward.token_jaccard, "token overlap is symmetric");
    assert.equal(diff.label, compareTitle(NOTES_WITHOUT_HANDLER, DRAFTS_WHOLE, reversed));
    assert.ok(
      diff.label.startsWith(`${HANDLER_RIGHT}${ENDPOINT_SEPARATOR}${HANDLER_LEFT}`),
      `the reversed title leads with notes: ${diff.label}`,
    );
    const shown = await diffDocuments(diff.input);
    assert.equal(shown.original, sourceOf(NOTES_WITHOUT_HANDLER));
    assert.equal(shown.modified, sourceOf(DRAFTS_WHOLE));
    assert.ok(!shown.original.includes(NOTES_ONLY_TEXT[1] ?? ""), "the notes side stops before its handler");
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });
});
