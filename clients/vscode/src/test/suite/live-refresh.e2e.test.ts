// E2E: prove the report store (and therefore the tree) refreshes
// automatically when a watched-extension file changes on disk —
// without any editor interaction. Mirrors the AI-agent / CI / git
// path that triggers `LiveWatcher` → scheduler → `deslop/reportChanged`.
//
// Covers [LSP-PUSH-NOTIFICATIONS] and [VSIX-REACTIVITY-INVARIANT].

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import type { ReportCluster } from "../../types/report";

import { activateExtension, deleteRange, fixtureRoot, openFixture, waitFor } from "./helpers";

const SHIPPING_FILE = "Shipping.cs";
const REPORT_FOR_RANGE = "deslop/reportForRange";
const INSIDE_OCCURRENCE_BYTE = 1;
const REPORT_READY_TIMEOUT_MS = 15_000;

suite("live tree refresh", () => {
  let fixtureDir: string;

  suiteSetup(async () => {
    fixtureDir = fixtureRoot();
    const api = await activateExtension();
    const store = api.reportStore;
    assert.ok(store, "reportStore must be exposed on ExtensionApi");
    await waitFor(() => (store.current.report ? true : undefined), 30_000);
  });

  test("editing a watched file on disk advances the report generation", async () => {
    const api = await activateExtension();
    const store = api.reportStore;
    assert.ok(store);
    const initialGeneration = store.current.generation;
    const initialReport = store.current.report;
    assert.ok(initialReport, "initial report must be loaded");

    const targetFile = path.join(fixtureDir, "Alpha.cs");
    const original = fs.readFileSync(targetFile, "utf8");
    try {
      const mutated = `${original}\n// live-refresh marker ${Date.now()}\n`;
      fs.writeFileSync(targetFile, mutated, "utf8");

      const newGeneration = await waitFor(() => {
        const g = store.current.generation;
        return g > initialGeneration ? g : undefined;
      }, 15_000);

      assert.ok(
        newGeneration > initialGeneration,
        `generation must advance: initial=${initialGeneration}, new=${newGeneration}`,
      );
      const updatedReport = store.current.report;
      assert.ok(updatedReport, "report must remain populated after live refresh");
    } finally {
      fs.writeFileSync(targetFile, original, "utf8");
      // Let the second re-analysis settle so other tests see a stable state.
      await waitFor(() => {
        const r = store.current.report;
        return r ? true : undefined;
      }, 10_000);
    }
  });

  test("editing a file via the editor advances the report generation", async () => {
    const api = await activateExtension();
    const store = api.reportStore;
    assert.ok(store);
    const initialGeneration = store.current.generation;

    const uri = vscode.Uri.file(path.join(fixtureDir, "Beta.cs"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc);
    await editor.edit((b) =>
      b.insert(new vscode.Position(0, 0), `// editor edit ${Date.now()}\n`),
    );
    await doc.save();

    const newGeneration = await waitFor(() => {
      const g = store.current.generation;
      return g > initialGeneration ? g : undefined;
    }, 15_000);
    assert.ok(
      newGeneration > initialGeneration,
      `generation must advance after editor edit: initial=${initialGeneration}, new=${newGeneration}`,
    );

    // Restore to keep other tests deterministic.
    await deleteRange(editor, 0, 0, 1, 0);
    await doc.save();
  });

  // [LIVE-OPEN-BUFFERS] [VSIX-LIVE-BUBBLE] The LSP must not answer a query
  // against unsaved editor text with a cluster calculated from the older
  // bytes still on disk.
  test("an unsaved deleted clone is absent from reportForRange", async () => {
    const api = await activateExtension();
    assert.ok(api.client, "the real LSP must be running");
    const editor = await openFixture(SHIPPING_FILE);
    const cluster = await waitFor(() => api.reportStore?.current.report?.clusters.find((candidate) =>
      candidate.occurrences.some((item) => path.basename(item.path) === SHIPPING_FILE),
    ), REPORT_READY_TIMEOUT_MS);
    const occurrence = cluster.occurrences.find((item) => path.basename(item.path) === SHIPPING_FILE);
    assert.ok(occurrence, "the first Shipping method is a reported clone");
    const start = editor.document.positionAt(occurrence.start_byte);
    const end = editor.document.positionAt(occurrence.end_byte);
    const range = new vscode.Range(start, end);
    const original = editor.document.getText(range);
    await editor.edit((builder) => builder.delete(range));
    try {
      assert.ok(editor.document.isDirty, "the changed buffer has not been saved");
      const result = await api.client.sendRequest<ReportCluster[]>(REPORT_FOR_RANGE, {
        path: editor.document.uri.fsPath,
        start_byte: occurrence.start_byte + INSIDE_OCCURRENCE_BYTE,
        end_byte: occurrence.start_byte + INSIDE_OCCURRENCE_BYTE,
      });
      assert.equal(
        result.some((candidate) => candidate.id === cluster.id),
        false,
        "a deleted method must not remain a duplicate in the unsaved buffer",
      );
    } finally {
      await editor.edit((builder) => builder.insert(start, original));
      await editor.document.save();
    }
  });

  // [VSIX-REACTIVITY-TREE] Regression: the LSP watcher previously deduped
  // paths globally instead of per-callback batch, so the second save to the
  // same file never reached the scheduler. The tree therefore froze after
  // one keystroke. Walk three sequential saves and assert each one bumps
  // the generation.
  test("repeated edits to the same file each advance the generation", async () => {
    const api = await activateExtension();
    const store = api.reportStore;
    assert.ok(store);

    const targetFile = path.join(fixtureDir, "Alpha.cs");
    const original = fs.readFileSync(targetFile, "utf8");
    try {
      let last = store.current.generation;
      for (let attempt = 1; attempt <= 3; attempt += 1) {
        const mutated = `${original}\n// repeat-save marker ${attempt} ${Date.now()}\n`;
        fs.writeFileSync(targetFile, mutated, "utf8");
        // The predicate's return type is explicit because the loop assigns this
        // call's result back into `last`, which the predicate itself reads.
        // Inferring the return would have to resolve `next` to resolve `last`,
        // and TypeScript breaks that cycle by falling back to `any`
        // (TS7022/TS7024). Stating the type keeps the generation a number.
        const next = await waitFor((): number | undefined => {
          const g = store.current.generation;
          return g > last ? g : undefined;
        }, 15_000);
        assert.ok(
          next > last,
          `save #${attempt} must advance generation past ${last}, got ${next}`,
        );
        last = next;
      }
    } finally {
      fs.writeFileSync(targetFile, original, "utf8");
      await waitFor(() => {
        const r = store.current.report;
        return r ? true : undefined;
      }, 10_000);
    }
  });
});
