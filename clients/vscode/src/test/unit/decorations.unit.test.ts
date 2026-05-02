// Unit: decorations helpers. Runs under vscode-test so the real vscode
// module is available for TextDocument/Position wiring.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { hoverFor, byteRangeToRange } from "../../decorations/manager";
import { ReportCluster, ReportOccurrence } from "../../types/report";

function cluster(): ReportCluster {
  return {
    id: "cluster-1",
    weight: 10,
    size: 4,
    canonical_node_count: 5,
    bucket: "same_behavior",
    signals: { structural: 0.1, token_jaccard: 0.2, embedding_cos: 0.9, fused: 0.95 },
    occurrences: [
      { path: "/a.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/b.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    summary: "summary",
    interpretation: "interp",
  };
}

suite("decorations helpers", () => {
  test("hoverFor renders the shared card design without raw AI data", () => {
    const md = hoverFor(cluster());
    const text = md.value;
    // Category label and count visible.
    assert.match(text, /Same behavior, different code/);
    assert.match(text, /×\s*2/, "instance count must be visible");
    // Canonical section present.
    assert.match(text, /Canonical/, "canonical section must be shown");
    // Action links present.
    assert.match(text, /command:deslop.openCluster/);
    assert.match(text, /command:deslop.compareWithCanonical/);
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
    const occurrence: ReportOccurrence = {
      path: "/x",
      start_byte: 0,
      end_byte: 9999,
      hidden: false,
    };
    assert.equal(byteRangeToRange(doc, occurrence), null);
  });

  test("byteRangeToRange maps bytes to positions inside the document", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "hello\nworld\n",
      language: "plaintext",
    });
    const occurrence: ReportOccurrence = {
      path: "/x",
      start_byte: 0,
      end_byte: 5,
      hidden: false,
    };
    const range = byteRangeToRange(doc, occurrence);
    assert.ok(range);
    assert.equal(range.start.line, 0);
    assert.equal(range.start.character, 0);
    assert.equal(range.end.line, 0);
    assert.equal(range.end.character, 5);
  });
});
