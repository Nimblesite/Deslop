# Rename-recall repair plan — #367, #369, #370, #373

One defect family: **a consistently-renamed real duplicate never reaches the report.** Two mechanisms lose it — the token signature drops the pair before it clusters (#367, and its downstream #369/#370), and the noise filters hide the cluster after it forms (#373). Every fix below is a *replacement*: the defective code is deleted, not guarded, not thresholded, not wrapped.

Supersedes `docs/plans/embedding-accuracy-plan.md`, which covered only the embedding half. Nothing references it; it is deleted as part of step 0.

## Measured at HEAD `8dc3d1f47`

| # | Repro | Result |
|---|---|---|
| #367 | Two 1.1 KB TS files, consistent rename + **one** added paren pair (1 node in ~380) | `Found 0 groups`, `clusters_hidden: 0` — pair dies at fusion. Drop the parens → `Found 1 group`. |
| #373 | Same-named `normalize_records` helper copied into two Python modules, locals renamed | `Found 0 groups`, `clusters_hidden: 1`. Log: `structural=1.0 token_jaccard=1.0 rename_consistency=0.94` — a textbook Type-2, hidden by the noise filter. |
| #369 | 3 `#[ignore]`d tests in tree | `pair_size_coherence:124`, `issue_343_sum_clamp_saturation:89`, `lsp_embedding_determinism:36` |
| #370 | 1 `#[ignore]`d test in tree | `embedding_failure_progress:32` — hangs >14 min on the rejected-refresh path |

## Fix 1 — [FUSION-SIGNALS-TOKEN-MULTISET] (#367, root cause)

`crates/deslop-core/src/lsh.rs::minhash_signature` estimates Jaccard over the **set** of distinct k-grams. A repetitive body has a small distinct-gram set, so one inserted node displaces a large share of it: two functions 99.7% identical by node count measure `token_jaccard = 0.664`, `structural = 0` (the paren rehashes every ancestor Merkle), and `bounded_fused < FUSED_THRESHOLD` kills the pair. Nothing downstream can recover it.

**Delete** the set-of-distinct-grams feed into `minhash_signature` (`pipeline/signatures.rs::signature_for_tokens`). **Replace** with a multiset signature — each k-gram occurrence tagged with its per-type ordinal, so multiplicity carries weight and a single inserted node perturbs a handful of features instead of a whole feature class. `estimate_jaccard`, `BANDS`, `ROWS_PER_BAND`, `SIGNATURE_LEN` are unchanged; this is a feature-construction replacement, not a scoring one.

**Not permitted:** lowering `FUSED_THRESHOLD` or `LSH_ONLY_MIN_JACCARD`. Validation runs both directions on the pinned corpus — recall on shape-changing Type-3, and zero new false positives on repetitive scaffolding.

## Fix 2 — [CLONE-NOISE-COPY-PROOF] (#373)

Every noise filter's escape hatch compares **raw source bytes**, so only a *verbatim* copy survives and every renamed copy is hidden. The module header at `cluster_filters/mod.rs:88` claims "a verbatim/renamed copy survives" — the code has never done the renamed half.

**Delete** `enclosing_function_bodies_differ` (`cluster_filters/mod.rs:471`) and all seven `raw_snippet_texts_differ` call sites: `mod.rs:221`, `dart.rs:101`, `dart_data_table.rs:37`, `ecmascript.rs:27`, `python_constants.rs:33`, `rust.rs:422`, `rust.rs:517`. **Replace** with one predicate over the `ContentEvidence` the pipeline already measures (`content.rs` — `agreement`, `rename_consistency`, `literal_fraction`): a cluster whose identifier mapping is bijective and whose literals align is a *proven copy* and is never filtered as noise. `cluster_is_hidden` already holds `cluster.content`; thread it into `is_noise_pattern` rather than re-deriving anything. One predicate, seven call sites, no per-language variants.

## Fix 3 — [FUSION-EMBED-PROVIDER] (#369a)

`crates/deslop/tests/cli/mock_ollama.rs::embed_vector` returns `[sin(len), cos(first_byte), 0.5, -0.5]`: two constant lanes floor every cosine and `sin` aliases over length — a 67-byte and an 865-byte text score 0.99997. That manufactures the two embedding-only false positives #369 names.

**Delete** `embed_vector`. **Replace** with a 128-lane signed shingle signature (L2-normalised indicator of distinct 5-char shingles, sign-folded to 128 lanes — measured separations 0.954 real pair / 0.782 whole-fn vs chunk / 0.107 params vs chunk / 1.0 identical). 4096 lanes is not landable: `exact_embedding_pairs` is O(N²·D) and blew past ten minutes. Keep `CosinePoint`'s precomputed per-point norm; never recompute norms inside the pair loop. Recalibrate the ~88 dependent `embedding_cos` assertions against the honest instrument — equal or stronger discrimination, none deleted or loosened.

## Fix 4 — [PAIR-SIZE-COHERENCE] (#369b)

`pair.rs::survival_decision` applies the LSH-only floors only when `structural <= 0 && embedding_cos <= 0`, so any ε of cosine waives both the 0.90 Jaccard floor and the 40-node floor — *weaker* corroboration promotes a pair.

**Delete** the `&& embedding_cos <= 0` conjunct. **Replace** with: every `structural <= 0` pair must clear `lsh_only_node_floor`; the Jaccard floor is waived only when `embedding_cos >= fused_min_score`, i.e. when the embedding axis independently clears the survival bar. No new constants.

## Fix 5 — [RANK-CATEGORY] un-gate the LSH promotion

`report_render.rs::is_csharp_lsh_type3_near_miss` promotes a language-agnostic evidence profile but requires `language == "csharp"`, so the identical TypeScript pair routes `LooselySimilar` → hidden.

**Delete** the language test and the `csharp` in the function name. Sweep the corpus fixtures: any newly visible cluster is adjudicated as genuine or earns its own filter *with its own fixture* — never a threshold bump. Reaches nothing until Fix 1 lands (the path also needs `token_jaccard >= 0.90`; measured 0.664).

## Fix 6 — [LIVE-EMBEDDING-REFRESH-TERMINAL] (#370)

On the rejected-refresh path the server emits no terminal `deslop/embeddingProgress` frame, so the client blocks forever in the unbounded read — upstream of the test's 20 s timeout. This is a server defect, not a test defect.

**Delete** the path that can exit without publishing a terminal frame. **Replace** with a refresh that publishes exactly one terminal frame (success *or* failure) on every exit, preserving the last-good report. Adding a client-side timeout to the test is prohibited — it would convert a hang into a green run over a server that still hangs in the editor.

## Order and gates

`0 → 1 → 5 → 3 → 4 → 6`; Fix 2 is independent and can land first. After each fix: the four un-ignored tests, then `cargo test --workspace --all-targets --features deslop-core/live`, then the self-scan duplication gate. Un-ignoring is the acceptance criterion — no assertion may be weakened, no `#[ignore]` may be added, and a red test left in the tree beats a softened one.

## Checklist

- [ ] **0.** Promote the four scratchpad repros to committed fixtures (`ts-rename-paren` for #367, `py-renamed-helper` for #373); delete `docs/plans/embedding-accuracy-plan.md`
- [ ] **1a.** Red E2E, no embeddings: rename + one paren pair, `--min-nodes 100` → 1 visible cluster, 2 occurrences spanning the whole function (`start_byte <= 9`, `end_byte >= 1200`), act-now bucket, `clusters_hidden == 0`
- [ ] **1b.** Delete the distinct-gram feed; replace with the ordinal-tagged multiset signature
- [ ] **1c.** Corpus sweep both directions — recall up, zero new false positives, no threshold touched
- [ ] **2a.** Red E2E: same-named helper, consistent rename, two files → 1 visible cluster, `clusters_hidden == 0`, `Nearly identical`; byte-identical control still `Identical`
- [ ] **2b.** Delete `enclosing_function_bodies_differ` + all 7 `raw_snippet_texts_differ` call sites; replace with the single `ContentEvidence` copy-proof predicate
- [ ] **2c.** Correct the `cluster_filters/mod.rs` header claims to match the code
- [ ] **3.** Delete `embed_vector`; replace with the 128-lane shingle signature; recalibrate the ~88 `embedding_cos` assertions
- [ ] **4.** Delete the `embedding_cos <= 0` conjunct in `survival_decision`; node floor always, Jaccard waiver only at `cos >= fused_min_score`
- [ ] **5.** Delete the `csharp` language gate in `is_csharp_lsh_type3_near_miss`; rename; adjudicate every newly visible corpus cluster
- [ ] **6.** Delete the no-terminal-frame exit in the embedding refresh; publish exactly one terminal frame per refresh
- [ ] **7.** Un-ignore all four: `pair_size_coherence:124`, `issue_343_sum_clamp_saturation:89`, `lsp_embedding_determinism:36`, `embedding_failure_progress:32`
- [ ] **8.** Restore the blanked spec IDs at `cluster_filters/mod.rs:444` (`Detects ****:`) and `report_render.rs:355` (dangling `//,`); register `[FUSION-SIGNALS-TOKEN-MULTISET]`, `[CLONE-NOISE-COPY-PROOF]`, `[LIVE-EMBEDDING-REFRESH-TERMINAL]` in `docs/specs/`
- [ ] **9.** `make ci` green, coverage ratchet held, self-scan duplication gate passes
