# Fused confidence — remaining work

Tracks what is left of `[FUSION-CONTENT-GATE]`. Requirements live in [`root-cause-fusion.md`](../root-cause-fusion.md); the shipped mechanism is specified in [`fusion.md`](../specs/fusion.md#fusion-content-gate) and pinned by `fused_golden_bands.rs` and `fused_golden_invariants.rs`. Quarantine repairs are planned in [`quarantine-repair-plan.md`](quarantine-repair-plan.md); the real-repository gate that has to settle these claims is planned separately in [`corpus-assertion.md`](corpus-assertion.md).

Completed work has been removed from this document. It is recorded in the commits and in the tests that pin it — restating it here is how this plan came to contradict itself (#395).

Branch: `worktree-fused-score-followups`, PR #392.

## The one measure

Every reported cluster is a real duplicate, and every real duplicate is reported. Everything below is ordered by how much it moves that number, not by how hard it is.

## Landing PR #392

- [ ] Commit and push the staged `cloned_ref_to_slice_refs` fix in `pipeline/signatures.rs` — the merge from `main` collided our F# test with a newer clippy. `make lint` is green locally with it; it is the only red check.
- [ ] Merge. `main` is not branch-protected; the failing check is the only block.
- [ ] `.deslop.toml` carries `max_duplication_percent = 13.65`, raised by #396 rather than earned; #392 measured **12.4978%** by deleting real duplication. Restoring 12.5 is tracked in **#397** and belongs there, not in #392.

## Open bugs

### 1. #339 — F# `token_jaccard` is byte-offset luck

Pinned red in the tree: `signatures::tests::issue_339_sibling_window_signature_is_offset_invariant` parses two F# modules whose shared window is byte-identical at shifted offsets and asserts their signatures match. They fall through to `blake3(hash, byte_range)` and share nothing.

Quarantining `fallback_signature` behind a `panic!` would abort every scan containing an unresolvable range, which is most of them — so the fix is offset-invariant signatures, not quarantine.

**Goes before #336.** Both are F#; fixing #339 changes the token signal #336 is measured against, so taking #336 first means measuring it twice.

- [ ] Offset-invariant sibling-window signatures; the red test goes green for the real reason.
- [ ] Re-measure F# `token_jaccard` on the corpus before touching #336.

### 2. The false-positive cluster

Unblocked by #343's bounded fusion, none verified against it. Three root causes, worth attacking as groups rather than one at a time.

**None is closeable until the corpus can express a precision assertion.** No manifest field today says *"these two things are not duplicates"*, so none of these seven can be pinned on the repository it was reported against — see section A of [`corpus-assertion.md`](corpus-assertion.md).

- [ ] **Assertion-idiom FPs** — #71 (same HTTP verb + status assertion), #103 (pytest `monkeypatch` chains, fixture call-sites), #285 (TDBIN diagnostic tests grouped by assertion idiom). Test scaffolding sharing a verb and an assertion shape.
- [ ] **Data-table / object-literal FPs** — #283 and #284 (unrelated object-literal tables and TDBIN scenarios), #336 (numeric array literals rank #1 on `dotnet/fsharp`). Data-table classification is Dart-only; #336 is the F# instance of that gap.
- [ ] **Helper-call-site FPs** — #79 (call sites distinguished only by literal arguments).

### 3. #344 — carry the confidence to every consumer

| Surface | Today |
|---|---|
| CLI text report (`render/text.rs`) | prints no signals at all |
| LSP diagnostics / code lens (`deslop-lsp`) | no confidence anywhere |
| Autofix gates (`refactor/preconditions.rs`) | bucket pre-filter + byte proof only |

- [ ] `agreement` / `rename_consistency` / `literal_fraction` onto `ReportSignals` in [`live-ipc.td`](../models/live-ipc.td), regenerate — never hand-write. **Population point:** `impl From<PairScore> for ReportSignals` converts the raw triple *before* content is measured and cannot carry them; [`content_gated_signals`](../../crates/deslop-core/src/buckets.rs#L316) holds the `ContentEvidence` and is the one place every rendered cluster passes through. It must stamp all three on **both** paths — today it early-returns unchanged for `Identical` and for non-saturating shapes.
- [ ] Render the three fields everywhere `fused` renders — HTML footer, Markdown, VSIX `SignalStrip`, `HelpBubble`. Until then no black-box test can assert the gate's input, and neither humans nor agents can see *why* a cluster routed where it did.
- [ ] `render/text.rs`, `deslop-lsp`, `refactor/preconditions.rs`.
- [ ] Restore the 17 fixtures #341 softened from maximal to partial renames — the engine carries the originals now, and the golden bands suite proves it per language.
- [ ] `metrics.duplication_percent` / exit-code gate — **delegated** to [weighted-metrics-plan.md](weighted-metrics-plan.md) under [METRICS-REPO-WEIGHTED]; not this plan's work.

### 4. Regression-audit follow-ups

- [ ] **#393** Win32 path semantics for the VS Code user-data dir are never exercised on Windows
- [ ] **#394** no event-matrix regression test for the Dependabot security-gate repair
- [ ] **#395** this plan marked the same work both open and fixed — addressed by the rewrite above; close when it stays true

## Close-outs — evidence first, never on a run CI has not seen

Fixed and pinned; open only because nobody closed them. Each needs one verification pass against the tree, **naming the assertion**, not the run.

- [ ] **#343** sum-then-clamp — `bounded_fused()` at both call sites; `issue_343_sum_clamp_saturation.rs` green
- [ ] **#351** discarded cosines — `add_embedding_pairs` calls `record_cosine` unconditionally; no quarantine panic remains
- [ ] **#372** `f32` cosine drift — fixed by #384 (`cosine_from_parts`, `f64` accumulation); three width-sweep tests on `main`
- [ ] **#345** doc drift — every row done; verify against the tree, then close
- [ ] **#336** close only on a curated F# run, after #339 — the F# curation and the ranking-check repair it depends on are section C and D of [`corpus-assertion.md`](corpus-assertion.md)
- [ ] **#347**, **#331** — corpus-gate close-outs, tracked in [`corpus-assertion.md`](corpus-assertion.md)

## Requirement status ([`root-cause-fusion.md`](../root-cause-fusion.md))

| # | Requirement | Status |
|---|---|---|
| 1 | Give the ensemble an independent member | 🟡 Content evidence is independent and steers routing, fusion and ranking — but it remains a render-stage gate rather than an ensemble member, and the semantic signal is still off by default |
| 2 | Stop clamping away the top of the range | ✅ #343 — `bounded_fused()` never exceeds the strongest single axis, so `fused = 1.0` again requires an axis that measured 1.0 |
| 3 | Preserve some literal information | 🟡 `ContentEvidence` compares raw literal bytes positionally; the fingerprint and token layers still collapse every literal to `__literal__` |

## Order

```
#392 lands ──► #339 (F# signatures) ──► the FP cluster ──► #344 rollout
                                             ▲
              corpus-assertion.md A–E ───────┘  (supplies the pins)
```

The false-positive work waits on the corpus gaining a precision surface: until it has one, a fix and a regression look identical on a real repository.
