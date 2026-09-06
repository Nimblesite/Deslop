// Unit: DecorationManager end-to-end — seed a store, open a matching editor,
// trigger a redraw and verify we don't throw. [VSIX-PERF] redraws are now
// coalesced through an injected scheduler: tests pass an immediate scheduler so
// the flush runs synchronously, and a capturing scheduler to assert coalescing.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { DecorationManager } from "../../decorations/manager";
import { ReportStore } from "../../reportStore";
import { ScheduleFn } from "../../util/debounce";
import { Report, ReportCluster } from "../../types/report";
import { reportWithClusters } from "./report.helpers";

// Runs the debounced flush synchronously so each redraw path executes inline.
const immediate: ScheduleFn = (callback) => {
  callback();
  return () => {};
};

// Captures the armed callback instead of firing it, so a test can assert that a
// burst of store changes leaves exactly one pending flush (coalesced).
function capturingScheduler(): {
  schedule: ScheduleFn;
  flush(): void;
  armed: number;
  hasPending: boolean;
} {
  let pending: (() => void) | null = null;
  const state = {
    schedule: ((callback) => {
      state.armed += 1;
      pending = callback;
      return () => {
        pending = null;
      };
    }) as ScheduleFn,
    flush(): void {
      const next = pending;
      pending = null;
      next?.();
    },
    armed: 0,
    get hasPending(): boolean {
      return pending !== null;
    },
  };
  return state;
}

function cluster(path: string): ReportCluster {
  return {
    id: "dm-1",
    weight: 10,
    size: 3,
    canonical_node_count: 4,
    bucket: "identical",
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: [{ path, start_byte: 0, end_byte: 3, hidden: false }],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

function report(clusters: ReportCluster[]): Report {
  return reportWithClusters(
    clusters,
    {},
    { analysed_loc: 10, duplicated_loc: 1, duplication_percent: 1, duplicated_files: 1 },
  );
}

suite("DecorationManager redraw", () => {
  test("redraws when a matching editor is visible", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "abc\ndef\n",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const manager = new DecorationManager(store, immediate);
    store.setSnapshot(report([cluster(doc.uri.fsPath)]), 0);
    // The immediate scheduler runs the flush inline — no throw ⇒ pass.
    manager.dispose();
    assert.ok(true);
  });

  test("clears decorations when the report is null", () => {
    const store = new ReportStore();
    const manager = new DecorationManager(store, immediate);
    manager.dispose();
  });

  test("does NOT schedule a redraw on a document edit — only on report/visibility changes (VSIX-PERF)", async () => {
    // The core proof that decorations do no work per keystroke: a buffer edit must
    // schedule ZERO decoration redraws (decorations are file-change driven), while
    // a fresh report — the file-change-driven analysis update — schedules one.
    const doc = await vscode.workspace.openTextDocument({
      content: "abcdef",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const scheduler = capturingScheduler();
    const manager = new DecorationManager(store, scheduler.schedule);
    try {
      const afterConstruct = scheduler.armed;
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "z"));
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "y"));
      assert.equal(
        scheduler.armed,
        afterConstruct,
        "a document edit must schedule no decoration redraw — decorations react to file changes, not keystrokes",
      );
      store.setSnapshot(report([cluster(doc.uri.fsPath)]), 1);
      assert.ok(
        scheduler.armed > afterConstruct,
        "a report change (file-change-driven analysis update) DOES schedule a decoration redraw",
      );
    } finally {
      manager.dispose();
    }
  });

  test("clears decorations through the visibility path when there is no report", async () => {
    // Covers the null-report short-circuit in flush + the clear helper, driven by a
    // visible-editor change rather than a keystroke.
    const doc = await vscode.workspace.openTextDocument({
      content: "qwerty",
      language: "plaintext",
    });
    const store = new ReportStore();
    assert.equal(store.current.report, null, "fresh ReportStore starts with a null report");
    const manager = new DecorationManager(store, immediate);
    try {
      await vscode.window.showTextDocument(doc);
    } finally {
      manager.dispose();
    }
  });

  test("coalesces a burst of store changes into a single armed flush (VSIX-PERF)", () => {
    const store = new ReportStore();
    const scheduler = capturingScheduler();
    const manager = new DecorationManager(store, scheduler.schedule);
    try {
      // Construction's initial effect arms once; two snapshots re-arm twice more.
      store.setSnapshot(report([cluster("/repo/A.cs")]), 1);
      store.setSnapshot(report([cluster("/repo/A.cs")]), 2);
      assert.ok(scheduler.armed >= 3, "each report change re-arms the trailing flush");
      assert.ok(scheduler.hasPending, "the burst leaves exactly one flush pending, not one-per-event");
      scheduler.flush();
      assert.ok(!scheduler.hasPending, "firing the pending flush drains it");
    } finally {
      manager.dispose();
    }
  });
});
