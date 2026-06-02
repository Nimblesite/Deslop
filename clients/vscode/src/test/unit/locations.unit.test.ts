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
});

function writeFixture(): { dir: string; file: string; source: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-locations-"));
  const file = path.join(dir, "ChatProtocol.cs");
  const source = "namespace Demo;\n\npublic class ChatProtocol {\n    void Send() {}\n}\n";
  fs.writeFileSync(file, source, "utf8");
  return { dir, file, source };
}

function cluster(file: string, startByte: number): ReportCluster {
  return {
    id: "issue-8",
    weight: 1,
    size: 1,
    canonical_node_count: 1,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [{ path: file, start_byte: startByte, end_byte: startByte + 9, hidden: false }],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

function report(clusters: ReportCluster[]): Report {
  return {
    tool_version: "test",
    min_nodes: 1,
    files_analysed: 1,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 1 },
    metrics: {
      analysed_loc: 1,
      duplicated_loc: 0,
      duplication_percent: 0,
      clusters_total: clusters.length,
      duplicated_files: 1,
      threshold: { percent: 0, breached: false, source: "none" },
      per_file: [],
    },
    schema_doc: "",
    action_hints: [],
    boilerplate_hints: [],
    embedding_provenance: undefined,
    clusters,
  };
}
