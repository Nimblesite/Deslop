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
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [{ path: file, start_byte: startByte, end_byte: startByte + 9, hidden: false }],
    summary: "",
    interpretation: "",
  };
}

function report(clusters: ReportCluster[]): Report {
  return {
    report_schema_version: 1,
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
    },
    schema_doc: "",
    action_hints: [],
    embedding_provenance: null,
    clusters,
  };
}
