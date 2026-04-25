# Autofix — Extract Method (true Type-1) — implementation plan

> **Spec is the source of truth.** Behavioural shape, preconditions, emitter rules, destination policy, and caveats live in [autofix-extract.md](../specs/autofix-extract.md) under `[AUTOFIX-EXTRACT-*]`. This file describes only **how** to build it.

## Scope

A v1 LSP `textDocument/codeAction` of kind `refactor.extract` that:

- Offers an *"Extract identical code to shared method"* action on true Type-1 clusters.
- Returns one `WorkspaceEdit` per eligible cluster: insert the helper at a fixed destination, replace each occurrence with a call.
- Never offers itself for Type-2, cross-file, cross-class, mid-expression, or single-occurrence clusters.

Non-goals (deferred): Type-2, semantic-model integration, cross-file refactors, destination prompts, user-supplied method names, type inference, instance-method (`impl fn` / non-static) extraction.

## Hard dependencies

1. **Issue [#42](https://github.com/Nimblesite/Deslop/issues/42)** — `ClusterKind::Identical` must split true Type-1 from Type-2 before this work can land. Without #42 the action is unsafe to offer ([AUTOFIX-EXTRACT-DEPENDENCIES]). **Block this work behind #42.**
2. The `LanguageParser` trait gains free-variable + emitter methods — the **single extension point** rule still holds.
3. `LiveApi` exposes `clusters_intersecting(uri, range)` (or the LSP layer adapts existing methods to that shape). Confirm before Phase 3; add if missing.

## File layout

New code in `deslop-core` lives behind a `refactor` module so the LSP layer stays thin:

- `crates/deslop-core/src/refactor/mod.rs` — public surface: `ExtractMethodPlan`, `compute_plan(cluster, parser) -> Result<Option<ExtractMethodPlan>, RefactorError>`. Spec ID comments referencing `[AUTOFIX-EXTRACT-*]`.
- `crates/deslop-core/src/refactor/free_vars.rs` — language-agnostic walk skeleton driven by the per-language node-kind tables.
- `crates/deslop-core/src/refactor/emit.rs` — language-agnostic `WorkspaceEdit` assembly given a per-language emitter result.
- Per-language emitter implementations live next to the existing parser implementations in the language plugin file (e.g. `crates/deslop-core/src/languages/csharp.rs`). The `LanguageParser` trait gains `binding_node_kinds`, `identifier_reference_kinds`, `emit_extract_method`.

LSP wiring:

- `crates/deslop-lsp/src/code_action.rs` — **new**. `textDocument/codeAction` handler. Forwards to `LiveApi::clusters_intersecting`, calls `refactor::compute_plan` per eligible cluster, returns the resulting `CodeAction`s. Stays under 200 lines.
- `crates/deslop-lsp/src/backend.rs` — advertise `codeActionProvider` with `codeActionKinds: ["refactor.extract"]` and dispatch to the new module.

Trait extension:

- The `LanguageParser` trait gains the new methods in the same PR that adds the trait change; existing C# / Rust / Python implementations get empty placeholders that compile and return "no extraction available" until their phase lands.

## Phases

Each phase ends with green CI and at least one passing E2E test.

**Phase 0 — wait for [#42](https://github.com/Nimblesite/Deslop/issues/42).** Track the issue. No code in this plan is mergeable until #42 lands the Type-1 vs Type-2 split.

**Phase 1 — `refactor::free_vars` end-to-end on C#.** Implement the walk, the C# node-kind table, and an E2E test that opens a fixture and asserts the free-var list for a known cluster through a public `deslop-core` API. No LSP wiring yet.

**Phase 2 — C# emitter + `WorkspaceEdit` assembly.** Generate the method header, body, and call-site replacements. E2E asserts the textual `WorkspaceEdit` matches a golden.

**Phase 3 — LSP code action.** Wire `textDocument/codeAction` to `compute_plan`. E2E spawns the real `deslop-lsp` binary and asserts the action appears, the edit applies, and the resulting buffer matches a golden.

**Phase 4 — Rust + Python.** Same shape; per-language emitter and node-kind table. New E2E goldens including the Rust `DeslopTodo` alias and Python PEP 8 spacing.

**Phase 5 — Negative-path coverage.** E2E proves the action is **not** offered on Type-2, cross-file, cross-class, single-occurrence, or mid-expression clusters per [AUTOFIX-EXTRACT-TESTING].

## Implementation notes

- The LSP layer must not emit code or walk the AST itself. All refactor logic stays in `deslop-core::refactor` so the same engine could later back the JetBrains plugin or a CLI command.
- Emitter output is **deterministic** — same cluster id, same byte-for-byte output. Required for golden tests.
- `compute_plan` returns `Result<Option<ExtractMethodPlan>, RefactorError>`. `Ok(None)` means the cluster failed preconditions — never an error. Errors are reserved for "we tried to compute and the parse tree was missing".
- No `unwrap` / `expect` in production paths per CLAUDE.md. Boundary errors flow through `thiserror` in `deslop-core`.
- The trait gains methods for **two distinct purposes** — free-variable analysis and method emission. Keep them as separate trait methods so a future language can implement free-var analysis (cheap) before emitter support (expensive).
- Files stay under 500 lines. If `csharp.rs` is close to the limit, split the emitter into a sibling file before adding more.

## Verification

1. `make ci` — green.
2. `make test` — all phases' E2E tests in scope; coverage threshold maintained per `coverage-thresholds.json`.
3. Manual end-to-end in a real VS Code window: open a fixture with a known Type-1 cluster, position cursor inside an occurrence, see the code action, apply it, confirm the resulting buffer matches the goldens.

---

## TODO

- [ ] Block this plan on issue [#42](https://github.com/Nimblesite/Deslop/issues/42). No PR merges until #42 ships the `kind_detail` Type-1 vs Type-2 split.
- [ ] Confirm `LiveApi::clusters_intersecting(uri, range)` exists; add it if missing. Spec ID comment: `[AUTOFIX-EXTRACT-CODE-ACTION]`.
- [ ] Extend `LanguageParser` with `binding_node_kinds`, `identifier_reference_kinds`, `emit_extract_method`. Update existing C# / Rust / Python implementations to compile (empty placeholders OK in the trait-change PR).
- [ ] Add `crates/deslop-core/src/refactor/` module: `mod.rs`, `free_vars.rs`, `emit.rs`. Public surface `compute_plan(cluster, parser) -> Result<Option<ExtractMethodPlan>, RefactorError>`. Spec ID comments referencing `[AUTOFIX-EXTRACT-*]`.
- [ ] Implement the free-variable walk in `free_vars.rs` driven by per-language node-kind tables. E2E test against a C# fixture proves the free-var list for a known cluster. Spec ID comment: `[AUTOFIX-EXTRACT-FREE-VARS]`.
- [ ] Implement the C# emitter in the C# language plugin. E2E asserts the generated `WorkspaceEdit` matches a golden. Spec ID comment: `[AUTOFIX-EXTRACT-EMITTER-CSHARP]`.
- [ ] Add `code_action.rs` to `crates/deslop-lsp/src/`. Advertise `codeActionProvider` with `codeActionKinds: ["refactor.extract"]` in `backend.rs`. Spec ID comment: `[AUTOFIX-EXTRACT-CODE-ACTION]`.
- [ ] LSP E2E: real binary, fixture workspace, code action computed, applied, post-apply buffer matches golden. Spec ID comments: `[AUTOFIX-EXTRACT-WORKSPACE-EDIT]`, `[AUTOFIX-EXTRACT-CODE-ACTION]`.
- [ ] Repeat emitter + golden + E2E for Rust including the `DeslopTodo` type alias. Spec ID comment: `[AUTOFIX-EXTRACT-EMITTER-RUST]`.
- [ ] Repeat emitter + golden + E2E for Python with PEP 8 spacing. Spec ID comment: `[AUTOFIX-EXTRACT-EMITTER-PYTHON]`.
- [ ] Negative-path E2E suite per [AUTOFIX-EXTRACT-TESTING]: Type-2, cross-file, cross-class, single-occurrence, mid-expression — each must produce no code action.
- [ ] Update [PLAN.md](PLAN.md) — move this plan from "Remaining" to "Implemented" once Phase 5 is green.
