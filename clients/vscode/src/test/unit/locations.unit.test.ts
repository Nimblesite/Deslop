// Unit: occurrence byte ranges are projected into human editor locations
// before VSIX UI surfaces render them.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import {
  occurrenceDisplayLocation,
  reportWithDisplayLocations,
} from "../../locations";
import { Report, ReportCluster } from "../../types/report";
import { reportWithClusters } from "./report.helpers";
import { bucketSignals } from "../signals.helpers";
import { occurrence, wireCluster } from "../cluster.helpers";

suite("occurrence display locations", () => {
  test("projects a byte offset to one-indexed line and column", () => {
    const fixture = writeFixture();
    try {
      const startByte = fixture.source.indexOf("void Send");
      const location = occurrenceDisplayLocation({
        path: fixture.file,
        start_byte: startByte,
        end_byte: startByte + "void Send".length,
        hidden: false,
      });
      assert.deepEqual(location, {
        line: 4,
        column: 5,
        label: `${fixture.file}:4:5`,
        description: "line 4, column 5",
        commandTitle: "Open ChatProtocol.cs at 4:5",
      });
    } finally {
      fs.rmSync(fixture.dir, { recursive: true, force: true });
    }
  });

  test("adds display locations to every webview report occurrence", () => {
    const fixture = writeFixture();
    try {
      const startByte = fixture.source.indexOf("void Send");
      const enriched = reportWithDisplayLocations(
        report([cluster(fixture.file, startByte)]),
      );
      assert.equal(
        enriched.clusters[0]?.occurrences[0]?.displayLocation?.label,
        `${fixture.file}:4:5`,
      );
    } finally {
      fs.rmSync(fixture.dir, { recursive: true, force: true });
    }
  });

  test("first-line byte yields column=byte+1 (no preceding newline branch)", () => {
    // Covers the `lastNewline === -1` branch of positionForByte: when the
    // byte lands on the first source line, column is derived from the raw
    // prefix length, not from the offset-past-lastNewline formula.
    const fixture = writeFixture();
    try {
      // Byte 7 lands at 'D' of 'Demo' on line 1 ('namespace Demo;').
      const location = occurrenceDisplayLocation({
        path: fixture.file,
        start_byte: 7,
        end_byte: 11,
        hidden: false,
      });
      assert.equal(location?.line, 1, "first line must be line 1");
      assert.equal(location?.column, 8, "column is one-indexed past the prefix");
    } finally {
      fs.rmSync(fixture.dir, { recursive: true, force: true });
    }
  });

  test("missing source file produces no display location", () => {
    // Covers the `readFileSync` catch → return undefined branch and
    // therefore the early-return in occurrenceDisplayLocation.
    const location = occurrenceDisplayLocation({
      path: "/definitely/not/a/real/path/XYZ.cs",
      start_byte: 0,
      end_byte: 1,
      hidden: false,
    });
    assert.equal(location, undefined);
  });

  test("reads each source file once per pass, not once per occurrence (VSIX-PERF)", () => {
    const source = "namespace Demo;\n\npublic class ChatProtocol {\n    void Send() {}\n}\n";
    const file = "/repo/ChatProtocol.cs";
    const reads = new Map<string, number>();
    const reader = (occurrencePath: string): string | undefined => {
      reads.set(occurrencePath, (reads.get(occurrencePath) ?? 0) + 1);
      return occurrencePath === file ? source : undefined;
    };
    const startByte = source.indexOf("void Send");
    const shared = cluster(file, startByte);
    // Two occurrences in the SAME file — the old shape read it twice.
    shared.occurrences = [
      { path: file, start_byte: startByte, end_byte: startByte + 4, hidden: false },
      { path: file, start_byte: startByte + 5, end_byte: startByte + 9, hidden: false },
    ];

    const enriched = reportWithDisplayLocations(report([shared]), reader);
    const occurrences = enriched.clusters[0]?.occurrences ?? [];
    assert.equal(occurrences.length, 2, "both occurrences survive the enrichment pass");
    assert.ok(
      occurrences.every((item) => item.displayLocation?.label.startsWith(file)),
      "every occurrence in the shared file is enriched with a display location",
    );
    assert.equal(reads.get(file), 1, "the shared source file is read exactly once for the whole pass");
  });
});

function writeFixture(): { dir: string; file: string; source: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-locations-"));
  const file = path.join(dir, "ChatProtocol.cs");
  const source = "namespace Demo;\n\npublic class ChatProtocol {\n    void Send() {}\n}\n";
  fs.writeFileSync(file, source, "utf8");
  return { dir, file, source };
}

function cluster(file: string, startByte: number): ReportCluster {
  return wireCluster({
    id: "issue-8",
    weight: 1,
    size: 1,
    canonical_node_count: 1,
    bucket: "identical",
    signals: bucketSignals("identical"),
    occurrences: [occurrence(file, startByte, startByte + 9)],
  });
}

function report(clusters: ReportCluster[]): Report {
  return reportWithClusters(
    clusters,
    { tool_version: "test", min_nodes: 1, cache_stats: { hits: 0, misses: 1 } },
    { analysed_loc: 1, duplicated_files: 1 },
  );
}
