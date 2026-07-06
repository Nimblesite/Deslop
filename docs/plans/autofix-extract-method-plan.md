# Autofix — Mechanical (Zero-Risk) Deduplication — implementation plan

> **Spec is the source of truth.** Behavioural shape, the mergeability gate, the anti-unification algorithm, preconditions, emitter rules, per-language defaults, destination policy, and caveats live in [autofix-extract.md](../specs/autofix-extract.md) under `[AUTOFIX-*]` (`[AUTOFIX-ZERO-RISK]`, `[AUTOFIX-EXTRACT]`, `[AUTOFIX-EXTRACT-NORTH-STAR]`, `[AUTOFIX-EXTRACT-PRECONDITIONS]`, `[AUTOFIX-EXTRACT-FREE-VARS]`, `[AUTOFIX-EXTRACT-EMITTER]`, `[AUTOFIX-EXTRACT-EMITTER-CSHARP]`, `[AUTOFIX-EXTRACT-EMITTER-RUST]`, `[AUTOFIX-EXTRACT-EMITTER-PYTHON]`, `[AUTOFIX-EXTRACT-DESTINATION]`, `[AUTOFIX-EXTRACT-WORKSPACE-EDIT]`, `[AUTOFIX-EXTRACT-CODE-ACTION]`, `[AUTOFIX-EXTRACT-CAVEATS]`, `[AUTOFIX-EXTRACT-TESTING]`, `[AUTOFIX-EXTRACT-DEPENDENCIES]`, `[AUTOFIX-MERGE]`, `[AUTOFIX-CONSOLIDATE]`). This file describes only **how** to build it. Implementation order: Type-1 verbatim extract → leaf-gap merge → cross-file consolidation; each reuses the `refactor` module the prior tier establishes.

## Scope

The mechanical (no-AI) autofix family — LSP code actions + one MCP tool — over a single shared `deslop-core::refactor` module:

- **Tier 1 — `[AUTOFIX-EXTRACT]` (Type-1 verbatim).** A `refactor.extract` action *"Extract identical code to shared method"* on true Type-1 clusters: one `WorkspaceEdit` inserting the helper + rewriting each occurrence as a call. Single-file, single-class.
- **Tier A — `[AUTOFIX-MERGE]` (leaf-gap Type-2 / constrained Type-3).** A `refactor.rewrite` action that anti-unifies the occurrences into one parameterised helper (differing leaves → parameters, with default values for constant-across-sites positions) and rewrites every site. Mechanical naming, type unification, value-vs-thunk.
- **Tier B — `[AUTOFIX-CONSOLIDATE]` (cross-file identical definitions).** A `refactor.rewrite` action that keeps a canonical copy, `DeleteFile`s the duplicates, and rewrites imports/references everywhere via a per-language import/symbol resolver.

Non-goals (route to the AI fallback, [autofix-extract-ai-plan.md](autofix-extract-ai-plan.md)): structural / control-flow-drift Type-3, Type-4 semantic clones, intent-laden naming. **Python is out of scope for Tiers A/B unless strict type checking (basedpyright / pyright `strict`) is active** ([AUTOFIX-ZERO-RISK]).

## Hard dependencies

1. **Issue [#42](https://github.com/Nimblesite/Deslop/issues/42) — shipped** (PR #63). The split is **not** a `kind_detail` field: `report_bucket_kind` demotes signal-`Identical` clusters whose member slices are not whitespace-canonicalised byte-equivalent to `NearlyIdentical` (`[CLONE-BUCKETS-IDENTICAL]`). The bucket label alone is *not* the refactor gate, because the nested-cluster collapse (`[PIPELINE-CLUSTER-EXACT]` #50) keeps the outer method-level Type-2 view of the renamed-methods-with-identical-bodies case; the authoritative Type-1 gate is the refactor layer's own byte-equivalence proof on the effective rewrite spans ([AUTOFIX-EXTRACT-PRECONDITIONS] rules 1 and 5, including whole-function body narrowing).
2. The `LanguageParser` trait gains free-variable + emitter methods — the **single extension point** rule still holds.
3. `LiveApi` exposes `clusters_intersecting(uri, range)` (or the LSP layer adapts existing methods to that shape). Confirm before Phase 3; add if missing.

## File layout

New code in `deslop-core` lives behind a `refactor` module so the LSP layer stays thin:

- `crates/deslop-core/src/refactor/mod.rs` — public surface: `ExtractMethodPlan`, `compute_plan(cluster, parser) -> Result<Option<ExtractMethodPlan>, RefactorError>`. Spec ID comments referencing `[AUTOFIX-EXTRACT-*]`.
- `crates/deslop-core/src/refactor/free_vars.rs` — language-agnostic walk skeleton driven by the per-language node-kind tables.
- `crates/deslop-core/src/refactor/emit.rs` — language-agnostic `WorkspaceEdit` assembly given a per-language emitter result.
- Per-language emitter implementations live next to the existing parser implementations in the language plugin file (e.g. `crates/deslop-core/src/lang/csharp.rs`). The `LanguageParser` trait gains `binding_node_kinds`, `identifier_reference_kinds`, `emit_extract_method`.

LSP wiring:

- `crates/deslop-lsp/src/code_action.rs` — **new**. `textDocument/codeAction` handler. Forwards to `LiveApi::clusters_intersecting`, calls `refactor::compute_plan` per eligible cluster, returns the resulting `CodeAction`s. Stays under 200 lines.
- `crates/deslop-lsp/src/backend.rs` — advertise `codeActionProvider` with `codeActionKinds: ["refactor.extract"]` and dispatch to the new module.

Trait extension:

- The `LanguageParser` trait gains the new methods in the same PR that adds the trait change, as **default methods returning "no extraction available"** — every existing implementation (C#, Rust, Python, Dart, JS/TS, F#, PHP) compiles unchanged until its phase overrides them.

## Phases

Each phase ends with green CI and at least one passing E2E test.

**Phase 0 — satisfied.** [#42](https://github.com/Nimblesite/Deslop/issues/42) shipped in PR #63: the Type-1 vs Type-2 split is the `[CLONE-BUCKETS-IDENTICAL]` byte-equivalence routing in `report_bucket_kind` (no `kind_detail` field).

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

- [x] ~~Block this plan on issue [#42](https://github.com/Nimblesite/Deslop/issues/42).~~ Shipped in PR #63 — the split is the `[CLONE-BUCKETS-IDENTICAL]` byte-equivalence routing (`report_bucket_kind` demotes non-byte-equivalent clusters to `NearlyIdentical`); there is no `kind_detail` field. Spec updated to match.
- [x] ~~Confirm `LiveApi::clusters_intersecting(uri, range)` exists~~ — `LiveApi::report_for_range(path, start_byte, end_byte)` already serves exactly this shape; the LSP layer reads the lock-free snapshot via `report_for_range_in`. No new `LiveApi` method needed. Spec ID comment: `[AUTOFIX-EXTRACT-CODE-ACTION]`.
- [x] Extend `LanguageParser` with `binding_node_kinds`, `identifier_reference_kinds`, `emit_extract_method` **plus `extract_scope_kinds`** (container/frame kinds needed by preconditions rules 4–5 — same single-extension-point rationale). All four have defaults, so every existing language compiles unchanged.
- [x] Add `crates/deslop-core/src/refactor/` module: `mod.rs`, `free_vars.rs`, `emit.rs`, plus `preconditions.rs` and `tables.rs`. Public surface `compute_plan(cluster, source, parser) -> Result<Option<ExtractMethodPlan>, RefactorError>`. Spec ID comments referencing `[AUTOFIX-EXTRACT-*]`.
- [x] Implement the free-variable walk in `free_vars.rs` driven by per-language node-kind tables. E2E test against a C# fixture proves the free-var list for a known cluster (`crates/deslop-core/tests/refactor_extract.rs`). Spec ID comment: `[AUTOFIX-EXTRACT-FREE-VARS]`.
- [x] Implement the C# emitter in the C# language plugin. E2E asserts the applied plan matches the golden `crates/deslop-lsp/tests/fixtures/code_action/InvoiceMath.applied.cs`. Spec ID comment: `[AUTOFIX-EXTRACT-EMITTER-CSHARP]`.
- [x] Add `code_action.rs` to `crates/deslop-lsp/src/`. Advertise `codeActionProvider` with `codeActionKinds: ["refactor.extract"]` in `backend.rs`. Spec ID comment: `[AUTOFIX-EXTRACT-CODE-ACTION]`.
- [x] LSP E2E: real binary, fixture workspace, code action computed, applied, post-apply buffer matches golden (`crates/deslop-lsp/tests/code_action.rs`). Spec ID comments: `[AUTOFIX-EXTRACT-WORKSPACE-EDIT]`, `[AUTOFIX-EXTRACT-CODE-ACTION]`.
- [x] Repeat emitter + golden + E2E for Rust including the `DeslopTodo` type alias (snake_case helper name — spec amended to avoid `non_snake_case` warnings). Spec ID comment: `[AUTOFIX-EXTRACT-EMITTER-RUST]`.
- [x] Repeat emitter + golden + E2E for Python with PEP 8 spacing. Spec ID comment: `[AUTOFIX-EXTRACT-EMITTER-PYTHON]`.
- [x] Negative-path E2E suite per [AUTOFIX-EXTRACT-TESTING]: for the *verbatim* `[AUTOFIX-EXTRACT]` action, Type-2 / cross-file / cross-class / single-occurrence / mid-expression (plus truncated, hidden, overlapping, loose-bucket, table-less-language) each produce no plan and no code action (`crates/deslop-core/tests/refactor_extract_negative.rs`, `crates/deslop-lsp/tests/code_action.rs`).

### 🟢 Tier A — mechanical call-site merge ([AUTOFIX-MERGE])
- [x] Add in-process `PipelineSession::subtree_at_range` + `source_bytes_for` over `per_file`/`.sources` (plus `AnalysisSession::pipeline()` for live access); not on the wire. E2E retrieves a cluster's occurrence subtrees (`crates/deslop-core/tests/refactor_ast_access.rs`). Spec ID `[AUTOFIX-MERGE]`.
- [x] `refactor::merge::gate` — first-order syntactic lgg over the normalised forests with the store/coalesce rule (identical per-site tuples share a slot) plus a residual byte proof. Spec ID `[AUTOFIX-MERGE-ANTIUNIFY]`.
- [x] `refactor::merge::compute_merge_plan -> MergePlan { verdict: Mechanical | AiOrHuman }` — skeleton equality, Baker rename lifting, Baxter similarity ≥ 0.95, DIFF_LEAVES/PARAM_ARITY budgets, leaf-only differences. Spec ID `[AUTOFIX-MERGE-GATE]`.
- [x] `refactor::merge::safety` — the A–F checklist (reuses the free-var walk; boundary scan, declared-inside-read-after, write-in-span, declared-type unification; thunk-needing holes refuse in v1). Spec ID `[AUTOFIX-MERGE-SAFETY]`.
- [x] Mechanical name derivation (modal-candidate else positional) + default-value computation (trailing, modal, C# only) + type unification (no `object`/`DeslopTodo` guessing — refuse if no type unifies). Spec IDs `[AUTOFIX-MERGE-NAMES]`, `[AUTOFIX-MERGE-DEFAULTS]`.
- [x] Per-language merge emitters with the type-safety backstop: C# (defaults as trailing optional params), Rust (no defaults; the E2E **compiles the merged file with `rustc`**), Dart (goldens; no Dart toolchain in CI). Python always refuses in v1 (strict-typing detection not yet wired, [AUTOFIX-ZERO-RISK]). E2E goldens per language + negative (`ai_or_human`) fixtures.
- [x] `merge-plan { id } -> MergePlan` MCP tool cloning the `cluster-by-id` path (`tools/mod.rs`, `handlers.rs`, `schemas.rs`, `backend/mod.rs`, `backend/state.rs` → `ipc_call("merge/plan")`); prevention-first ≤200-char description; read-only. `MergePlan` lives in `docs/models/live-ipc.td`. E2E: `crates/deslop-mcp/tests/merge_plan.rs`. Spec ID `[AUTOFIX-MERGE-MCP]`.
- [x] `refactor.rewrite` LSP code action: `codeActionProvider { resolveProvider: true }`; offer (edit omitted, `data.cluster_id`) → `codeAction/resolve` attaches the annotated `WorkspaceEdit` (`changeAnnotations`; version ids null until the LSP tracks buffer versions); refusals resolve to `disabled.reason`. The compile assertion runs on the identical engine output in `rust_leafgap_merges_and_compiles` (`rustc --emit=metadata`). Spec ID `[AUTOFIX-MERGE-CODE-ACTION]`.

### 🔵 Tier B — cross-file consolidation ([AUTOFIX-CONSOLIDATE])
- [x] Consolidation resolver + gate (`crates/deslop-core/src/refactor/consolidate.rs`): v1 covers Rust sibling modules — visible canonical, byte-equivalent whole definitions, single-definition duplicates, reference detection driving the `use crate::<module>::<name>;` rewrite (the Schäfer invariant holds by construction: the only same-file definition is removed and the import re-binds every reference). Other languages refuse with a reason. Spec IDs `[AUTOFIX-CONSOLIDATE]`, `[AUTOFIX-CONSOLIDATE-GATE]`.
- [x] Multi-file edits + import rewrites; E2E consolidates a cross-file duplicate and the workspace **compiles** (`rustc`), negative fixtures refuse (`crates/deslop-core/tests/refactor_consolidate.rs`). `DeleteFile` for would-empty duplicates refuses in v1 (needs the `mod`-declaration rewrite; spec notes the follow-up). Spec ID `[AUTOFIX-CONSOLIDATE-EDIT]`.

- [x] Update [PLAN.md](PLAN.md) — Tier 1, Tier A, and Tier B (v1) are implemented; PLAN.md entry updated accordingly.
