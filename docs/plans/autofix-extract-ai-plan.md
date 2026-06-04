# Autofix — AI-assisted Extract Method (Type-2 / Type-3) — implementation plan

> **Spec is the source of truth.** [autofix-extract.md §[AUTOFIX-EXTRACT-AI]](../specs/autofix-extract.md#autofix-extract-ai-ai-assisted-extraction-for-type-2-and-type-3) defines the mechanical scaffold, the AI slot, the MCP tool pair, validation rules, and non-goals. This file describes only **how** to build it.

## Scope

**This is the fallback after `[AUTOFIX-MERGE]`.** Leaf-gap Type-2 / constrained Type-3 clusters are now merged **mechanically** (no AI) by [autofix-extract-method-plan.md](autofix-extract-method-plan.md) Tier A — anti-unification derives parameter names mechanically. The AI path remains only for the residue `[AUTOFIX-MERGE-GATE]` routes to `AiOrHuman`: clusters with structural / control-flow drift (gaps not confined to leaf positions), Type-4 semantic clones, or cases where a generalising parameter **name** materially aids readability. Renamed-identifier Type-2 is no longer a reason to invoke AI.

Two MCP tools — `extract-method-plan` and `extract-method-apply` — handle that residue by combining a mechanical AST-derived scaffold with an AI-filled name slot. The AI picks a method name and one canonical name per parameter slot; Deslop synthesises the final `WorkspaceEdit` deterministically.

Non-goals: Type-2 via LSP `codeAction` (synchronous; AI round-trip is not), AI-generated body code, AI-chosen destination, AI-driven type inference, freeform extract that accepts arbitrary code from the agent.

## Hard dependencies

1. **Type-1 path** ([autofix-extract-method-plan.md](autofix-extract-method-plan.md)) ships first. The slot-substitution layer extends the same emitter rather than forking it.
2. **Issue [#42](https://github.com/Nimblesite/Deslop/issues/42)** — Type-1 / Type-2 split must exist; the AI path eligibility check ([AUTOFIX-EXTRACT-AI-PRECONDITIONS]) reads `kind_detail`.
3. `LanguageParser` slot-alignment method — takes N parse subtrees, returns `Option<SlotMapping>`. Same single extension point as parsing and Type-1 free-vars.

## File layout

Reuses the `refactor` module from the Type-1 work; adds AI-specific entry points alongside.

- `crates/deslop-core/src/refactor/scaffold.rs` — **new**. `ExtractScaffold` type, `compute_scaffold(cluster, parser) -> Result<Option<ExtractScaffold>, RefactorError>`. Spec ID: `[AUTOFIX-EXTRACT-AI-SCAFFOLD]`.
- `crates/deslop-core/src/refactor/slots.rs` — **new**. Per-language slot-alignment driver invoking the trait; produces stable slot order. Spec ID: `[AUTOFIX-EXTRACT-AI-NORTH-STAR]` (mechanical bullet).
- `crates/deslop-core/src/refactor/apply.rs` — **new**. `apply_scaffold(scaffold, names) -> Result<WorkspaceEdit, ApplyError>`. Validation per [AUTOFIX-EXTRACT-AI-VALIDATION]. Idempotent.
- `LanguageParser` gains `align_slots(occurrences) -> Option<SlotMapping>`. Existing C# / Rust / Python parsers implement it.

MCP wiring:

- `crates/deslop-mcp/src/tools.rs` — register the two new tools. Tool descriptions are prompt engineering per [MCP-AGENT-PROMPT-GUIDANCE]; co-locate them with the existing `find-similar` description for review parity.
- `crates/deslop-mcp/src/protocol.rs` — extend the request / response shapes for the new tools.

## Phases

**Phase 0 — wait for Type-1.** No code lands until [autofix-extract-method-plan.md](autofix-extract-method-plan.md) Phase 5 is green.

**Phase 1 — slot alignment for C#.** `LanguageParser::align_slots` plus E2E test with a fixture that has two methods differing only by identifier renames. Asserts a stable slot mapping, count, and per-slot candidate names.

**Phase 2 — `compute_scaffold` end-to-end on C#.** Builds the body with `__deslop_param_<i>` placeholders, the per-site argument lists, and the destination range. E2E asserts the JSON scaffold matches a golden.

**Phase 3 — `apply_scaffold` + validation.** Substitutes AI-chosen names into the scaffold, validates per [AUTOFIX-EXTRACT-AI-VALIDATION], emits the final `WorkspaceEdit`. E2E covers happy path + every rejection branch (invalid identifier, collision, arity mismatch, duplicate, scope collision).

**Phase 4 — MCP tools.** Wire `extract-method-plan` and `extract-method-apply` in the MCP server. E2E spawns the real `deslop-mcp` binary, calls each tool, asserts the JSON contract.

**Phase 5 — Rust + Python.** Slot alignment + scaffold + apply for each language. New goldens.

**Phase 6 — Type-3 admission.** Extend eligibility from Type-2 to slot-alignable Type-3. E2E proves Type-3 clusters where free-var arity differs across occurrences are correctly **rejected** (no scaffold returned), and Type-3 clusters where arity agrees succeed.

## Implementation notes

- The body-string substitution at apply time uses placeholder sentinels chosen so they cannot occur in valid source — e.g. the leading `__deslop_param_` prefix is illegal as part of any user identifier under our naming rules. Validate this once at scaffold time with a grep over the destination file.
- Tool descriptions are prompt engineering, not docs. Phrase them so a host agent picks them up correctly: *"Before applying, call `extract-method-plan` to learn the parameter slot count and candidate names, then call `extract-method-apply` with your chosen names."*
- Apply must be **idempotent and pure**: same inputs → same `WorkspaceEdit` bytes. Required for goldens, required for agent retry semantics.
- All AI input flows through validation. **Never** synthesise an edit from an unvalidated string. The whole point of the bounded-slot design is that the AI cannot inject arbitrary text into the file.
- Files stay under 500 lines. If `tools.rs` is close, split per-tool implementations into siblings.

## Verification

1. `make ci` — green.
2. `make test` — all phases' E2E tests in scope; coverage threshold maintained.
3. Manual end-to-end with a real MCP client (Claude Code): connect to `deslop-mcp`, call `extract-method-plan` against a known Type-2 cluster, pick names, call `extract-method-apply`, apply the returned edit in the editor, confirm the file matches a golden.

---

## TODO

- [ ] Block this plan on the Type-1 path landing ([autofix-extract-method-plan.md](autofix-extract-method-plan.md) Phase 5 green) and on issue [#42](https://github.com/Nimblesite/Deslop/issues/42).
- [ ] Extend `LanguageParser` with `align_slots(occurrences) -> Option<SlotMapping>`. Update C# / Rust / Python implementations to compile (empty placeholder OK in the trait-change PR). Spec ID comment: `[AUTOFIX-EXTRACT-AI-NORTH-STAR]`.
- [ ] Add `crates/deslop-core/src/refactor/scaffold.rs`. Spec ID comments throughout. E2E asserts JSON scaffold against a C# fixture golden.
- [ ] Add `crates/deslop-core/src/refactor/slots.rs`. Spec ID comments. E2E covers slot-alignment success + arity-mismatch rejection.
- [ ] Add `crates/deslop-core/src/refactor/apply.rs`. Spec ID comments. E2E covers every validation branch in [AUTOFIX-EXTRACT-AI-VALIDATION].
- [ ] Add the two new MCP tools to `crates/deslop-mcp/src/tools.rs` and request/response types to `protocol.rs`. Tool descriptions follow [MCP-AGENT-PROMPT-GUIDANCE]. Spec ID comment: `[AUTOFIX-EXTRACT-AI-MCP-TOOLS]`.
- [ ] MCP E2E: real `deslop-mcp` binary, fixture workspace, `extract-method-plan` + `extract-method-apply` round-trip, returned `WorkspaceEdit` matches a golden.
- [ ] Repeat slot-alignment + scaffold + apply + MCP E2E for Rust.
- [ ] Repeat for Python.
- [ ] Extend eligibility from Type-2 to slot-alignable Type-3. E2E covers both the success case and the arity-mismatch rejection. Spec ID comment: `[AUTOFIX-EXTRACT-AI-PRECONDITIONS]`.
- [ ] Update [PLAN.md](PLAN.md) — move this plan to "Implemented" once Phase 6 is green.
