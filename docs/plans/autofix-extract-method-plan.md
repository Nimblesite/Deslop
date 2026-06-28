# Autofix — Mechanical (Zero-Risk) Deduplication — implementation plan

> **Spec is the source of truth.** Behavioural shape, the mergeability gate, the anti-unification algorithm, preconditions, emitter rules, per-language defaults, destination policy, and caveats live in [autofix-extract.md](../specs/autofix-extract.md) under `[AUTOFIX-*]` (`[AUTOFIX-ZERO-RISK]`, `[AUTOFIX-EXTRACT]`, `[AUTOFIX-EXTRACT-NORTH-STAR]`, `[AUTOFIX-EXTRACT-PRECONDITIONS]`, `[AUTOFIX-EXTRACT-FREE-VARS]`, `[AUTOFIX-EXTRACT-EMITTER]`, `[AUTOFIX-EXTRACT-EMITTER-CSHARP]`, `[AUTOFIX-EXTRACT-EMITTER-RUST]`, `[AUTOFIX-EXTRACT-EMITTER-PYTHON]`, `[AUTOFIX-EXTRACT-DESTINATION]`, `[AUTOFIX-EXTRACT-WORKSPACE-EDIT]`, `[AUTOFIX-EXTRACT-CODE-ACTION]`, `[AUTOFIX-EXTRACT-CAVEATS]`, `[AUTOFIX-EXTRACT-TESTING]`, `[AUTOFIX-EXTRACT-DEPENDENCIES]`, `[AUTOFIX-MERGE]`, `[AUTOFIX-CONSOLIDATE]`). This file describes only **how** to build it. Implementation order: Type-1 verbatim extract → leaf-gap merge → cross-file consolidation; each reuses the `refactor` module the prior tier establishes.

## Scope

The mechanical (no-AI) autofix family — LSP code actions + one MCP tool — over a single shared `deslop-core::refactor` module:

- **Tier 1 — `[AUTOFIX-EXTRACT]` (Type-1 verbatim).** A `refactor.extract` action *"Extract identical code to shared method"* on true Type-1 clusters: one `WorkspaceEdit` inserting the helper + rewriting each occurrence as a call. Single-file, single-class.
- **Tier A — `[AUTOFIX-MERGE]` (leaf-gap Type-2 / constrained Type-3).** A `refactor.rewrite` action that anti-unifies the occurrences into one parameterised helper (differing leaves → parameters, with default values for constant-across-sites positions) and rewrites every site. Mechanical naming, type unification, value-vs-thunk.
- **Tier B — `[AUTOFIX-CONSOLIDATE]` (cross-file identical definitions).** A `refactor.rewrite` action that keeps a canonical copy, `DeleteFile`s the duplicates, and rewrites imports/references everywhere via a per-language import/symbol resolver.

Non-goals (route to the AI fallback, [autofix-extract-ai-plan.md](autofix-extract-ai-plan.md)): structural / control-flow-drift Type-3, Type-4 semantic clones, intent-laden naming. **Python is out of scope for Tiers A/B unless strict type checking (basedpyright / pyright `strict`) is active** ([AUTOFIX-ZERO-RISK]).

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

**Phase 5 — Negative-path coverage (Tier 1).** E2E proves the *verbatim* `[AUTOFIX-EXTRACT]` action is **not** offered on Type-2, cross-file, cross-class, single-occurrence, or mid-expression clusters per [AUTOFIX-EXTRACT-TESTING] — those route to `[AUTOFIX-MERGE]` / `[AUTOFIX-CONSOLIDATE]`, not to AI.

**Phase 6 — AST access (Tier A prerequisite).** Add in-process `AnalysisSession::subtree_at_range(file_id, byte_range)` and `source_bytes_for(file_id)` reading the in-memory `PipelineSession.per_file` / `.sources`; **never serialised to the wire**. E2E asserts a known cluster's occurrence subtrees are retrievable through a public `deslop-core` API. Spec ID `[AUTOFIX-MERGE]`.

**Phase 7 — Tier A merge engine (C# first).** `refactor::anti_unify`, `decide_mergeability` (the gate), the A–F safety checklist, mechanical name derivation + default computation, type unification. E2E asserts the `MergePlan` (template, params, per-site args, defaults, verdict) against a C# fixture golden, plus negative fixtures (control-flow drift, Type-4) returning `ai_or_human`. Spec IDs `[AUTOFIX-MERGE-ANTIUNIFY]`, `[AUTOFIX-MERGE-GATE]`, `[AUTOFIX-MERGE-SAFETY]`, `[AUTOFIX-MERGE-NAMES]`.

**Phase 8 — Tier A surfaces + remaining languages.** `merge-plan` MCP tool (clone the `cluster-by-id` end-to-end path); `refactor.rewrite` code action with lazy resolve + annotated preview + transactional apply ([AUTOFIX-MERGE-MCP], [AUTOFIX-MERGE-CODE-ACTION]). Repeat emitter + golden + E2E for Rust and Dart; Python only under strict typing. **LSP E2E asserts the resulting workspace compiles** (the type-safety backstop).

**Phase 9 — Tier B consolidation.** Per-language import/symbol resolver + reference graph; the Schäfer binding invariant `lookup_after == lookup_before`; the consolidation gate; the `DeleteFile` + import-rewrite `WorkspaceEdit` ([AUTOFIX-CONSOLIDATE], [AUTOFIX-CONSOLIDATE-GATE], [AUTOFIX-CONSOLIDATE-EDIT]). E2E: an identical definition duplicated across two files consolidates to one, every reference rewritten, workspace compiles; negative fixtures (unresolved reference, name collision, visibility break) refuse.

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
- [ ] Negative-path E2E suite per [AUTOFIX-EXTRACT-TESTING]: for the *verbatim* `[AUTOFIX-EXTRACT]` action, Type-2 / cross-file / cross-class / single-occurrence / mid-expression each produce no code action (they belong to `[AUTOFIX-MERGE]` / `[AUTOFIX-CONSOLIDATE]`).

### 🟢 Tier A — mechanical call-site merge ([AUTOFIX-MERGE])
- [ ] Add in-process `AnalysisSession::subtree_at_range` + `source_bytes_for` over `PipelineSession.per_file`/`.sources`; not on the wire. E2E retrieves a cluster's occurrence subtrees. Spec ID `[AUTOFIX-MERGE]`.
- [ ] `refactor::anti_unify(subtrees) -> { template, per_site_substitutions }` — first-order syntactic lgg with the store/coalesce rule. Spec ID `[AUTOFIX-MERGE-ANTIUNIFY]`.
- [ ] `refactor::decide_mergeability(cluster, session) -> Mechanical | AiOrHuman` — skeleton equality, Baker prev-encoding, Baxter Similarity ≥ threshold, DIFF_LEAVES/PARAM_ARITY budgets, leaf-only differences. Spec ID `[AUTOFIX-MERGE-GATE]`.
- [ ] `refactor::safety` — the A–F checklist (reuse the free-var walk; add binding/dependency, value-vs-thunk, type-unification). Spec ID `[AUTOFIX-MERGE-SAFETY]`.
- [ ] Mechanical name derivation (modal-candidate else positional) + default-value computation + type unification (no `object`/`DeslopTodo` guessing — refuse if no type unifies). Spec IDs `[AUTOFIX-MERGE-NAMES]`, `[AUTOFIX-MERGE-DEFAULTS]`.
- [ ] Per-language emitters with the type-safety backstop (C# overload fallback, Rust `Option`/wrapper/builder, Dart nullable `??`, Python `None`-sentinel under strict typing). Reuse the `LanguageParser` emitter trait. E2E goldens per language + negative (`ai_or_human`) fixtures.
- [ ] `merge-plan { cluster_id } -> MergePlan` MCP tool cloning the `cluster-by-id` path (`tools/mod.rs`, `handlers.rs`, `schemas.rs`, `backend/mod.rs`, `backend/state.rs` → `ipc_call("merge/plan")`); prevention-first ≤200-char description; read-only. Add `MergePlan` to `docs/models/live-ipc.td`. Spec ID `[AUTOFIX-MERGE-MCP]`.
- [ ] `refactor.rewrite` LSP code action: advertise `codeActionProvider { resolveProvider: true }`; offer (edit omitted) → `codeAction/resolve` builds the multi-site annotated, versioned, transactional `WorkspaceEdit`. LSP E2E asserts the resulting workspace **compiles**. Spec ID `[AUTOFIX-MERGE-CODE-ACTION]`.

### 🔵 Tier B — cross-file consolidation ([AUTOFIX-CONSOLIDATE])
- [ ] Per-language import/symbol resolver + reference graph; Schäfer binding invariant; consolidation gate (resolvable refs, no collision, no visibility/orphan break, dependents in change set). Spec IDs `[AUTOFIX-CONSOLIDATE]`, `[AUTOFIX-CONSOLIDATE-GATE]`.
- [ ] `WorkspaceEdit` with `DeleteFile`/multi-file `TextDocumentEdit`s + import rewrites; E2E consolidates a cross-file duplicate, workspace compiles; negative fixtures refuse. Spec ID `[AUTOFIX-CONSOLIDATE-EDIT]`.

- [ ] Update [PLAN.md](PLAN.md) — move the relevant tiers from "Remaining" to "Implemented" as each phase goes green (Tier 1 after Phase 5, Tier A after Phase 8, Tier B after Phase 9).
