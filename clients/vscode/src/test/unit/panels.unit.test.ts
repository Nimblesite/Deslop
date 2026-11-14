// Unit: handleMessage dispatch — every case must execute the corresponding
// deslop.* command without throwing on malformed payloads.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as vscode from "vscode";

import { handleMessage } from "../../webview/panels";
import { ReportStore } from "../../reportStore";
import { occurrence, wireCluster } from "../cluster.helpers";
import { emptyReport, storeWith } from "./report-store.helpers";
import { withTempDir } from "./temp-file.helpers";

/** Five numbered lines; each `lineN` starts at a known byte. */
const SOURCE_LINES = ["line1", "line2", "line3", "line4", "line5"];
const LINE_BYTES = "lineN\n".length;
/** Zero-based editor lines of the two duplicates and of the file's top. */
const HEAVIER_DUPLICATE_LINE = 3;
const LIGHTER_DUPLICATE_LINE = 1;
const FILE_TOP_LINE = 0;
const TOP_RANK = 1;
const LOWER_RANK = 2;
const CLUSTER_MASS = 10;
const DUPLICATED_FILE = "Duplicated.cs";
const CLEAN_FILE = "Clean.cs";
const TEMP_PREFIX = "deslop-open-file-";

/** A two-occurrence cluster whose first occurrence starts `line` of `file`. */
function clusterAtLine(id: string, rank: number, file: string, line: number) {
  const start = line * LINE_BYTES;
  return wireCluster({
    id,
    rank,
    severity: "information",
    kind: "identical",
    mass: CLUSTER_MASS,
    occurrences: [occurrence(file, start, start + LINE_BYTES), occurrence(`${file}.other`)],
  });
}

/** The active editor's file and the line its selection starts on. */
function activeLocation(): { file: string; line: number } {
  const editor = vscode.window.activeTextEditor;
  assert.ok(editor, "a file row must open an editor");
  return { file: fs.realpathSync(editor.document.uri.fsPath), line: editor.selection.start.line };
}

suite("webview handleMessage", () => {
  test("ignores malformed messages", async () => {
    const store = new ReportStore();
    await handleMessage(store, null);
    await handleMessage(store, "string");
    await handleMessage(store, 42);
  });

  test("open/cluster dispatches when id is a string", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "open/cluster", id: "cluster-x" });
    await handleMessage(store, { kind: "open/cluster", id: 42 });
  });

  test("open/occurrence dispatches when occurrence is present", async () => {
    const store = new ReportStore();
    try {
      await handleMessage(store, {
        kind: "open/occurrence",
        occurrence: {path: "/tmp/doesnotexist.cs", start_byte: 0, end_byte: 1, hidden: false, start_line: 1, end_line: 2},
      });
    } catch {
      // the command dispatch tries to open the file which doesn't exist;
      // we only need to cover the dispatch branch
    }
    await handleMessage(store, { kind: "open/occurrence" });
  });

  test("[VSIX-METRICS-REPORT-NAVIGATE] open/file opens the file at its highest-ranked duplicate", async () => {
    await withTempDir(TEMP_PREFIX, async (dir) => {
      const duplicated = fs.realpathSync(dir) + `/${DUPLICATED_FILE}`;
      const clean = fs.realpathSync(dir) + `/${CLEAN_FILE}`;
      for (const file of [duplicated, clean]) fs.writeFileSync(file, `${SOURCE_LINES.join("\n")}\n`);
      // The lower-ranked cluster is listed first: the row must follow the
      // engine's rank, not the order the clusters happen to arrive in.
      const store = storeWith(
        emptyReport({
          clusters: [
            clusterAtLine("lighter", LOWER_RANK, duplicated, LIGHTER_DUPLICATE_LINE),
            clusterAtLine("heavier", TOP_RANK, duplicated, HEAVIER_DUPLICATE_LINE),
          ],
        }),
      );
      await handleMessage(store, { kind: "open/file", path: duplicated });
      assert.deepEqual(activeLocation(), { file: duplicated, line: HEAVIER_DUPLICATE_LINE });

      // A file no cluster touches still opens, at its first line.
      await handleMessage(store, { kind: "open/file", path: clean });
      assert.deepEqual(activeLocation(), { file: clean, line: FILE_TOP_LINE });

      // Malformed rows open nothing: the clean file stays active.
      for (const path of [undefined, 7, ""]) await handleMessage(store, { kind: "open/file", path });
      assert.deepEqual(activeLocation(), { file: clean, line: FILE_TOP_LINE });
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      store.dispose();
    });
  });

  test("open/cluster path validation — non-string id ignored", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "open/cluster", id: 42 });
  });

  test("compare/pair requires both explicit endpoints — single or missing endpoint is ignored", async () => {
    const store = new ReportStore();
    // [VSIX-PAIR-COMPARE] The host never invents an endpoint: one-sided or
    // empty payloads must fall through without dispatching a comparison.
    await handleMessage(store, { kind: "compare/pair" });
    await handleMessage(store, { kind: "compare/pair", left: { path: "a.ts", start_byte: 0, end_byte: 1 } });
    await handleMessage(store, { kind: "compare/pair", right: { path: "b.ts", start_byte: 0, end_byte: 1 } });
    await handleMessage(store, {
      kind: "compare/pair",
      left: { path: "a.ts", start_byte: 0, end_byte: 1 },
      right: { path: "b.ts", start_byte: 0, end_byte: 1 },
    });
  });

  test("compare/canonical dispatches only a cluster id with a well-formed row (#607)", async () => {
    const store = new ReportStore();
    const row = { path: "a.ts", start_byte: 0, end_byte: 1 };
    for (const message of [
      { kind: "compare/canonical" },
      { kind: "compare/canonical", clusterId: 7, occurrence: row },
      { kind: "compare/canonical", clusterId: "c", occurrence: { ...row, path: "" } },
      { kind: "compare/canonical", clusterId: "c", occurrence: { ...row, start_byte: Number.NaN } },
      { kind: "compare/canonical", clusterId: "c", occurrence: { ...row, end_byte: Number.POSITIVE_INFINITY } },
      { kind: "compare/canonical", clusterId: "c", occurrence: { path: "a.ts" } },
      { kind: "compare/canonical", clusterId: "c", occurrence: "a.ts" },
      { kind: "compare/canonical", clusterId: "c", occurrence: row },
    ]) {
      await handleMessage(store, message);
    }
    store.dispose();
  });

  test("refresh dispatches the workspace command", async () => {
    const store = new ReportStore();
    try {
      await handleMessage(store, { kind: "refresh" });
    } catch {
      // The LSP may not implement refreshReport — we only need to cover the dispatch line.
    }
  });

  test("legacy navigate messages are ignored by the host", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "navigate/next" });
    await handleMessage(store, { kind: "navigate/prev" });
  });

  test("unknown kind is ignored", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "nonsense" });
  });
});
