// Unit: compare virtual document URIs and byte-slice provider.

import * as assert from "node:assert/strict";
import * as fs from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  COMPARE_SCHEME,
  CompareContentProvider,
  buildCompareUri,
  parseCompareUri,
} from "../../compare/provider";
import { ReportOccurrence } from "../../types/report";

function occurrence(sourcePath: string, startByte: number, endByte: number): ReportOccurrence {
  return {
    path: sourcePath,
    start_byte: startByte,
    end_byte: endByte,
    start_line: 1,
    end_line: 2,
    hidden: false,
  };
}

async function tempSource(contents: string): Promise<string> {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "deslop-compare-"));
  const file = path.join(dir, "source with spaces.cs");
  await fs.writeFile(file, contents, "utf8");
  return file;
}

function directCompareUri(params: Record<string, string>): vscode.Uri {
  return vscode.Uri.parse(`${COMPARE_SCHEME}:/cluster/a/source?${new URLSearchParams(params)}`);
}

suite("compare provider", () => {
  test("buildCompareUri round-trips coordinates through parseCompareUri", () => {
    const sourcePath = "/tmp/deslop source/example.cs";
    const uri = buildCompareUri(occurrence(sourcePath, 6, 16), "b", "cluster-42");
    const parsed = parseCompareUri(uri);

    assert.equal(uri.scheme, COMPARE_SCHEME);
    assert.equal(parsed.sourcePath, sourcePath);
    assert.equal(parsed.startByte, 6);
    assert.equal(parsed.endByte, 16);
    assert.equal(parsed.side, "b");
    assert.equal(parsed.clusterId, "cluster-42");
  });

  test("parseCompareUri rejects other schemes and defaults missing query params", () => {
    assert.throws(() => parseCompareUri(vscode.Uri.file("/tmp/source.cs")), /expected deslop-compare/);

    const parsed = parseCompareUri(vscode.Uri.parse(`${COMPARE_SCHEME}:/missing-query`));
    assert.deepEqual(parsed, {
      sourcePath: "",
      startByte: 0,
      endByte: 0,
      side: "a",
      clusterId: "",
    });
  });

  test("provider returns only the requested byte range", async () => {
    const file = await tempSource("alpha beta gamma");
    const provider = new CompareContentProvider();
    const text = await provider.provideTextDocumentContent(
      buildCompareUri(occurrence(file, 6, 10), "a", "cluster-1"),
    );

    assert.equal(text, "beta");
  });

  test("provider clamps negative, oversized, inverted, and NaN byte ranges", async () => {
    const contents = "abcdef";
    const file = await tempSource(contents);
    const provider = new CompareContentProvider();

    const oversized = await provider.provideTextDocumentContent(
      buildCompareUri(occurrence(file, -10, 1000), "a", "cluster-1"),
    );
    const inverted = await provider.provideTextDocumentContent(
      buildCompareUri(occurrence(file, 4, 2), "a", "cluster-1"),
    );
    const nanStart = await provider.provideTextDocumentContent(
      directCompareUri({
        path: file,
        start: "NaN",
        end: "3",
        side: "b",
        cluster: "cluster-2",
      }),
    );

    assert.equal(oversized, contents);
    assert.equal(inverted, "");
    assert.equal(nanStart, "abc");
  });

  test("provider returns diagnostic text when the source file is unavailable", async () => {
    const provider = new CompareContentProvider();
    const missing = path.join(os.tmpdir(), "deslop-missing-source.cs");
    const text = await provider.provideTextDocumentContent(
      buildCompareUri(occurrence(missing, 0, 4), "b", "lost-cluster"),
    );

    assert.match(text, /Deslop could not load this compare occurrence/);
    assert.match(text, /Cluster: lost-cluster/);
    assert.match(text, /Side: B/);
    assert.match(text, /Path:/);
    assert.match(text, /Details:/);
  });
});
