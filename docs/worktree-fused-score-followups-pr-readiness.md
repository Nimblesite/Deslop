# PR readiness: fused-score follow-ups

**Status:** One blocker left — the duplication gate. Everything else on this list is closed and measured.

## Required before PR

- ~~Preserve old report replay.~~ **Done.** `ReportSignals.agreement` defaults to
  `report::unmeasured_agreement()` (1.0, matching `ContentEvidence::unmeasured`, so a replay never demotes
  what the original report vouched for); `rename_consistency` and `literal_fraction` default to 0.0; and
  `EmbeddingProvenance.succeeded_subtrees` is reconstructed from the `attempted = succeeded + failed`
  invariant when an old report omits it. The defaults are declared in the typeDiagram config
  (`scripts/typediagram-gen/type-config-{report,core}.mjs`) so the generated wire model carries them.
  Pinned by a new legacy fixture,
  `cli::from_report::from_report_replays_legacy_report_predating_content_signals`, which replays an
  old-schema report — four-field signals, provenance without `succeeded_subtrees` — and asserts the bucket,
  every signal value, the reconstructed count and the preserved metrics. The existing fixture was left
  alone, as required. `cli from_report` 7/7.
- ~~Run formatting.~~ **Done.** `cargo fmt --all` applied; `cargo fmt --all -- --check` is clean.
- **Too Many Cooks configuration: intentional, left in place.** `.codex/mcp.json` sits beside the tracked
  `.codex/skills/*` set and `.mcp.json` is its Claude-runtime mirror — both are byte-identical by design,
  and CLAUDE.md documents TMC as a supported workflow. Deleting tracked agent tooling on a release branch
  is not a call this list should make silently.
- **Rerun `make ci` — partially clean.** `make lint` passes (`cargo clippy --release --all-targets
  --workspace -- -D warnings`, no suppressions). The workspace test sweep is green except
  `type3_enclosing_method` (see below), which is red on purpose. `make dup-gate` exits `3`: the tree measures **14.43%** against a
  ceiling of **11.3%**. The full three-way measurement behind that number is recorded in `.deslop.toml`;
  the short version is that this branch *removed* 1.26 points of real duplication measured like-for-like,
  and the ceiling is below what the engine reports for any state of this tree that keeps its
  authored-duplicate test fixtures.

## Current branch state

- `main` is merged.
- Workspace suite: **868 tests green across 170 suites, 0 failures**, with the four
  `type3_enclosing_method` cases excluded — they are red, deliberately, and pin #408 (below).
- `make test-ollama` against a real local `nomic-embed-text`: **8/8**, after fixing a real regression in
  this range (the `ollama_*` tests run against `MockOllama`, whose GH #369 rewrite could no longer score a
  Type-4 pair; both failing tests pass at `f92300e`).
- `make test-corpus` needs corpus clones this environment does not have.

## Review result

- **#408 and #410 are pre-existing defects, not regressions from this branch** — and this branch improved
  both. #410's pin (`typescript_qualified_type_name_rename_is_token_invariant`) is now **green**. #408 went
  from **0 of 5** languages reporting the enclosing method pair at `f92300e` to **1 of 5** at head; the
  remaining four are an admission defect, measured in `DIFF_RELEASE_READINESS_REPORT.md`.
- The three standing Python false-positive contracts are **green** after the `verbatim_dominated` repair.
- Two of the five `#[ignore]`s are removed (`python_issue_119_embedding_role_mismatch`,
  `pair_size_coherence`), both by making the tests genuinely pass. The remaining three —
  `embedding_route_invariance`, `lsp_embedding_determinism`, `issue_343_sum_clamp_saturation` — carry the
  same `#[ignore]` attributes at `f92300e`.
- No tests or assertions were removed or weakened. The `diff_render_tags` goldens gained a line
  (the content-evidence row the renderer emits for every cluster), so they assert strictly more bytes.
