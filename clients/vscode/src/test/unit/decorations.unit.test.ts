// Unit: decorations helpers. Runs under vscode-test so the real vscode
// module is available for TextDocument/Position wiring.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { hoverFor, byteRangeToRange } from "../../decorations/manager";
import { ReportCluster, ReportOccurrence } from "../../types/report";
import { occurrence, wireCluster } from "../cluster.helpers";

function cluster(): ReportCluster {
  return wireCluster({
    id: "cluster-1",
    mass: 10,
        canonical_node_count: 2,
        occurrences: [
      occurrence("/a.cs", 0, 10),
      occurrence("/b.cs", 0, 10),
    ],
    occurrence_count: 4,
  });
}

suite("decorations helpers", () => {
  test("hoverFor renders the shared card design without raw AI data", () => {
    const md = hoverFor(cluster());
    const text = md.value;
    // Category label and count visible. The count is the engine's
    // `occurrence_count` — the fixture carries four copies and ships two
    // of them, exactly as the live wire does when it caps the list — so
    // this also pins that the hover shows the cluster's real size rather
    // than the length of the slice it happens to hold. The hover and the
    // hover provider used to answer this question with two different
    // helpers ([PRINCIPLES-ONE-CALCULATION]).
    assert.match(text, /Same behavior, different code/);
    assert.equal(cluster().occurrences.length, 2, "fixture: only two occurrences travelled");
    assert.match(text, /×\s*4/, "instance count must be the engine's, not the carried list's");
    // Canonical section present.
    assert.match(text, /Canonical/, "canonical section must be shown");
    // Action links present.
    assert.match(text, /command:deslop.openCluster/);
    // [VSIX-PAIR-COMPARE] Decorations cannot supply two endpoints, so no
    // compare link may render on a decoration.
    assert.doesNotMatch(text, /command:deslop\.compareWithCanonical/);
    assert.doesNotMatch(text, /command:deslop\.comparePair/);
    // No raw AI data.
    assert.doesNotMatch(text, /Type-/);
    assert.doesNotMatch(text, /structural/i);
    assert.doesNotMatch(text, /jaccard/i);
    assert.doesNotMatch(text, /embedding/i);
    assert.doesNotMatch(text, /fused/i);
  });

  test("byteRangeToRange returns null when range exceeds the document", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "abc\ndef\n",
      language: "plaintext",
    });
    const targetOccurrence: ReportOccurrence = {path: "/x",
      start_byte: 0,
      end_byte: 9999,
      hidden: false, start_line: 1, end_line: 2};
    assert.equal(byteRangeToRange(doc, targetOccurrence), null);
  });

  test("byteRangeToRange maps bytes to positions inside the document", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "hello\nworld\n",
      language: "plaintext",
    });
    const targetOccurrence: ReportOccurrence = {path: "/x",
      start_byte: 0,
      end_byte: 5,
      hidden: false, start_line: 1, end_line: 2};
    const range = byteRangeToRange(doc, targetOccurrence);
    assert.ok(range);
    assert.equal(range.start.line, 0);
    assert.equal(range.start.character, 0);
    assert.equal(range.end.line, 0);
    assert.equal(range.end.character, 5);
  });
});
