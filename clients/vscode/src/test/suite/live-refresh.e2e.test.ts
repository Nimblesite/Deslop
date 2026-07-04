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

import { activateExtension } from "./helpers";

async function waitFor<T>(
  predicate: () => T | undefined,
  timeoutMs: number,
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = predicate();
    if (value !== undefined) return value;
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 100);
    });
  }
  throw new Error(`waitFor timed out after ${timeoutMs}ms`);
}

suite("live tree refresh", () => {
  let fixtureDir: string;

  suiteSetup(async () => {
    const fixture = process.env["DESLOP_TEST_FIXTURE"];
    assert.ok(fixture, "fixture path must be set");
    fixtureDir = fixture;
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
    await editor.edit((b) =>
      b.delete(new vscode.Range(new vscode.Position(0, 0), new vscode.Position(1, 0))),
    );
    await doc.save();
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
        const next = await waitFor(() => {
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
