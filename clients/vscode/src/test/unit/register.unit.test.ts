// Unit: pure helpers from commands/register. Runs under vscode-test so the
// real TextDocument + Position are available.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import {
  byteToPosition,
  utf8ByteOffset,
  findClusterContaining,
} from "../../commands/register";
import { ReportCluster } from "../../types/report";
import { occurrence, wireCluster } from "../cluster.helpers";

async function mkDoc(content: string): Promise<vscode.TextDocument> {
  return await vscode.workspace.openTextDocument({ content, language: "plaintext" });
}

function cluster(path: string, start: number, end: number): ReportCluster {
  return wireCluster({
    id: `${path}:${start}:${end}`,
    mass: 1,
        canonical_node_count: 1,
        occurrences: [occurrence(path, start, end)],
  });
}

suite("register helpers", () => {
  test("byteToPosition round-trips through utf8ByteOffset", async () => {
    const doc = await mkDoc("hello\nworld");
    const end = new vscode.Position(1, 3);
    const byte = utf8ByteOffset(doc, end);
    const pos = byteToPosition(doc, byte);
    assert.equal(pos.line, end.line);
    assert.equal(pos.character, end.character);
  });

  test("byteToPosition clamps when byte >= document length", async () => {
    const doc = await mkDoc("abc");
    const pos = byteToPosition(doc, 9999);
    assert.equal(pos.line, 0);
    assert.equal(pos.character, 3);
  });

  test("findClusterContaining returns the matching cluster at cursor", async () => {
    const doc = await mkDoc("hello world");
    const clusters = [
      cluster("/tmp/hello.txt", 0, 5),
      cluster("/tmp/hello.txt", 6, 11),
    ];
    const hit = findClusterContaining(clusters, "/tmp/hello.txt", doc, new vscode.Position(0, 2));
    assert.ok(hit);
    assert.equal(hit.occurrences[0]?.start_byte, 0);
  });

  test("findClusterContaining returns undefined when no cluster overlaps", async () => {
    const doc = await mkDoc("hello");
    const clusters = [cluster("/other", 0, 5)];
    const miss = findClusterContaining(clusters, "/tmp/file", doc, new vscode.Position(0, 0));
    assert.equal(miss, undefined);
  });
});
