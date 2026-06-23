# VSIX UI-Thread Performance — Plan

**Symptom.** On a large codebase the editor UI slows to a crawl. CPU/memory are *not*
saturated, which points squarely at synchronous work on the **extension-host thread**
(the VS Code "main" thread). It is worst *whenever a change happens* — the classic
signature of an un-debounced edit handler doing report-sized work on every document
edit. Deslop reacts to **file changes** (the LSP's file watcher), so the VSIX must do
no report-sized work per edit on the UI thread.

**Server is not the bottleneck.** The Rust LSP already coalesces work: a debounce of
250 ms quiet / 2000 ms cap (`crates/deslop-core/src/live/debouncer.rs`), emits clusters
**pre-sorted worst-first** (`crates/deslop-core/src/report.rs` `reweigh_by_visible_occurrences`),
ships per-file metrics, and computes occurrence line numbers
(`crates/deslop-core/src/report_render.rs:115-127`, `docs/models/live-ipc.td:175-176`).
The flood is entirely **local to the extension host**.

---

## Verified root causes (file:line)

### RC1 — Decoration redraw was edit-driven and unbounded
- `clients/vscode/src/decorations/manager.ts` — `onDidChangeTextDocument(() => this.redrawAll())`
  fired on **every document edit**, with **no debounce**.
- `redrawAll()` redrew **every visible editor**, not just the changed one.
- `redraw()` ran `indexedSeverity(report.clusters)` (O(clusters)) **per editor** and iterated
  **all clusters × all occurrences**.
- `byteRangeToRange()` — **per occurrence in the active file**: `document.getText()` (whole doc) +
  `Buffer.from(text)` (whole doc) + 2× `slice().toString()` + 2× `positionAt()`.
- `redraw()` read `this.store.current.visibleReport`; `current` touches **every** signal, so the
  effect also re-fired on lifecycle/embedding ticks.

  → Per edit ≈ `O(occurrences-in-file × document-size)` allocations on the main thread.

  **Resolution:** decorations no longer subscribe to `onDidChangeTextDocument` at all. They are
  driven by the report signal (a file-change-driven analysis update — an unsaved edit reaches them
  via the dirty projection on `visibleReport`) and by editor-visibility changes, coalesced through
  a trailing debounce, with the document buffer built once per redraw. The VSIX does **zero**
  report-sized work per edit.

### RC2 — Webview push does synchronous file I/O per occurrence
- `clients/vscode/src/locations.ts:44-46` — `readOccurrenceSource()` = `fs.readFileSync` **per
  occurrence**, no per-file dedupe.
- `:29-37` `reportWithDisplayLocations` maps every cluster × occurrence and reads each file,
  **re-run on every push**.
- `clients/vscode/src/webview/panels.ts:127` report/duplication panels push on every
  `visibleReport` change; `:144` the cluster panel effect re-runs on **every store change**
  (lifecycle, embedding progress, pending model — not just report).
- The LSP **already** sends `start_line`/`end_line`; the client recomputes them by reading files.

### RC3 — Redundant re-sort on delta apply
- `clients/vscode/src/reportStore.ts:129` — re-sorts **all** clusters by weight on every delta,
  though the LSP emits worst-first and the delta preserves that order. O(n log n) per update.

---

## Fix strategy

Maps directly to the three asks: **move to the LSP where possible**, **reduce TS time**,
**batch with breaks**, and **don't flood TS on every change**.

### Phase 1 — Stop the per-keystroke decoration freeze (`manager.ts`) — no wire change
- Route **all** redraws through **one trailing debounce (~60 ms)**: a keystroke burst schedules a
  single flush (the "breaks between work").
- **Targeted redraw**: a text-document change schedules only the editors showing *that* document;
  report / visible-editor changes schedule a full pass.
- Build the document's byte→UTF-16 converter **once per editor-redraw** (single `getText()` +
  `Buffer.from`) and reuse it for all occurrences — replaces the per-occurrence whole-doc buffer.
- Compute `indexedSeverity` **once per report** (memoized by report identity), not per editor.
- Read `store.visibleReport.value` directly so the effect tracks only the report, not every signal.
- Early-out when the active file contributes no occurrences.

### Phase 2 — Stop the webview push I/O storm (`locations.ts`, `panels.ts`) — reduce TS
- `locations.ts`: read each **unique file at most once per pass** (per-path memo).
  O(occurrences) `readFileSync` → O(unique files). **Done.**
- Narrow the cluster-panel effect to depend on report/visibleReport only (not lifecycle/embedding/
  pending-model), so those ticks stop re-pushing the full feed. **Done.**
- ~~Debounce panel report pushes~~ — **not needed.** After narrowing + the per-file memo, the report/
  duplication effect already tracks only `visibleReport`, which changes at most once per
  server-debounced cycle (or once on the first keystroke per file), and each push is now cheap. Adding
  a debounce here only delays the first paint for no real coalescing benefit.

### Phase 3 — Trim redundant store work (`reportStore.ts`) — **evaluated, skipped**
- `applyDelta`'s O(n·log n) re-sort runs only per **server-debounced** delta with small N, and it is
  *required* to reorder added/updated clusters (a merged cluster can land anywhere by weight). An
  "already sorted?" guard would almost always miss (a fresh add/update is the reason we sort), so it
  would add an O(n) scan and still sort. Not a per-keystroke hotspot — left unchanged.

### Phase 4 — (Follow-up, deferred) Move column computation fully to the LSP
Add `start_col`/`end_col` to `docs/models/live-ipc.td` `ReportOccurrence` so the webview needs
**zero** file reads. Deferred to its own change: the wire `ReportOccurrence` is shared by the
CLI/HTML/Markdown/JSON renderers, so new fields ripple into E2E report snapshots — out of scope for
this hot-fix pass. Tracked here so the architectural move isn't lost.

---

## Verification
- `make _vsix-test` green; VSIX line coverage ≥ **95 %** (gate raised to 95).
- `make lint` (no suppressions) and `npx tsc --noEmit` clean.
- New/updated unit tests: decoration debounce coalescing; single-buffer byte→range correctness;
  per-file memo in `locations` (label still `path:line:col`); cluster-panel effect ignores
  embedding-progress; `applyDelta` preserves worst-first order without a full re-sort.

## Constraints honored
Files < 500 lines, functions < 20 lines, no regex on source, no linter suppressions, aggressively
DRY (one shared debounce util), no test weakened or deleted.

---

## TODO
- [x] **P1.1** Shared trailing-edge `debounce` util (injectable scheduler) — `src/util/debounce.ts`.
- [x] **P1.2** `manager.ts`: coalesce all redraws through one trailing debounce.
- [x] **P1.3** `manager.ts`: targeted redraw — text changes redraw only that document's editors.
- [x] **P1.4** `manager.ts`: build the byte→position buffer once per editor-redraw (`rangeFromBuffer`);
      lazily, only when the editor actually owns an occurrence.
- [x] **P1.5** `manager.ts`: memoize `indexedSeverity` per report; effect tracks `visibleReport` only.
- [x] **P2.1** `locations.ts`: per-path source memo within a pass — no per-occurrence reads.
- [x] **P2.3** `panels.ts`: narrow the cluster-panel effect to report/visibleReport deps.
- [~] **P2.2** `panels.ts`: debounce report pushes — **dropped** (unnecessary after narrowing + memo).
- [~] **P3.1** `reportStore.ts`: re-sort guard — **dropped** (not a hotspot; guard would always miss).
- [ ] **V.1** Add/refresh unit tests for each phase; `make _vsix-test` + `make lint` + `tsc` green; coverage ≥ 95 %.
- [ ] **P4 (deferred)** Add `start_col`/`end_col` to `live-ipc.td`; compute server-side; drop all file reads from `locations.ts`.
