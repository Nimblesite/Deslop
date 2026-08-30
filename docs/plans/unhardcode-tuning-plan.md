# Unhardcoding the tuning levers

Turns every compiled accuracy threshold into a configuration item with the current value as its default. Specified by [`fused.md §FUSION-TUNING-LEVERS`](../specs/fused.md#fusion-tuning-levers) (the levers and their provenance) and [`exclusion.md §[tuning]`](../specs/exclusion.md) (the file surface, validation, precedence, cache key, and report declaration).

**The whole migration is behaviour-preserving.** Every phase lands with the same reports it started with. No default moves in this work stream; a default change is a separate, test-first change with its own corpus measurement.

**The surface is growing while it is being specified.** The last three merged PRs each added compiled levers — #341 five (`support_floor`, `promote_floor`, `literal_table_min_fraction`, `literal_table_min_literals`, `verbatim_member_share_floor`), #346 two (`rename_consistency_discount`, `rename_evidence_min_literals`), #368 two (`saturating_token_floor`, `max_endpoint_node_ratio`) — nine of the twenty-one Tier A levers in three commits. Calibration work is where levers are born, so Phases 0–2 should land before the next calibration PR, or it adds compiled constants this plan then has to chase.

## Phase 0 — pin the defaults before touching anything

The safety net for every later phase, and the only phase that cannot be skipped or reordered.

- E2E over the full fixture set plus the pinned corpus repositories: capture each rendered report as a golden. Every later phase re-runs these and must produce byte-identical output.
- A test asserting each `DEFAULT_*` equals the literal the code ships today — named, per lever, so a silent edit fails a specific assertion rather than a diff.
- A test asserting a `.deslop.toml` carrying an empty `[tuning]` table produces the same report as no `[tuning]` table at all.

Exit: goldens green, the default-value assertions exist and pass.

## Phase 1 — split `config.rs`, add `TuningPolicy`

`crates/deslop-core/src/config.rs` is 974 lines against a 500-line budget, so it splits before it grows: `config/mod.rs`, `config/exclusion.rs`, `config/ranking.rs`, `config/tuning.rs`. Mechanical move, no behaviour change, goldens prove it.

`TuningPolicy` follows `RankingPolicy` exactly — a `Copy` struct of validated values, `Default` carrying the shipped constants, resolved once at load, with `with_global_override` reading the editor channel from `crate::state`. It hangs off the resolved config alongside `ranking_policy()`.

Tests: every range rejection and every cross-key invariant from [EXCLUSION-CONFIG] gets an assertion naming the key and the error, plus a round-trip asserting a fully-populated `[tuning]` table resolves to exactly the values written.

Exit: `TuningPolicy` exists, is validated, and is unused by the pipeline.

## Phase 2 — name the unnamed literals

Nothing inline can be configured, so the seven sites in the [FUSED-TUNING-LEVERS] unnamed table become `DEFAULT_*` constants with provenance doc comments first. The four repetitions of the shape-identical `0.99` collapse to one constant — they are one concept written four times, and today a change to one is a silent divergence.

This phase changes no value and adds no plumbing. Goldens prove it.

## Phase 3 — thread the policy through admission

`PipelineSession` already holds `Arc<ExclusionConfig>`, and `session/render.rs:68`/`:81` are the two admission call sites, so the reach is short. `CandidatePair` already carries per-pair `fused_min_score`, `lsh_only_min_jaccard`, and `lsh_only_node_floor` — the constants only *populate* those fields, so `pair/candidates.rs` sources them from the policy instead and the comparison sites never change.

`candidate_pairs_for_language_policy` and `cluster_by_transitive_closure` take `&TuningPolicy`. The cross-language arm (`candidates.rs:84`–`86`, `:181`–`183`) reads `candidates.cross_language_min_jaccard` from the same policy.

Tests: a fixture where a pair sits between two `fused_threshold` values, asserted to cluster under the lower and not under the default — the first test in this plan that would fail if the plumbing were fake. Same shape for `lsh_only_min_jaccard`, `lsh_only_min_node_count`, and `max_endpoint_node_ratio`, each with the cluster id, occurrence count, file paths, and bucket asserted.

## Phase 4 — content gate and routing

The widest blast radius: `classify_signals`, `is_structural_only_signals`, `lacks_content_support`, `has_saturating_shape_evidence`, and `attach_content_evidence` all take the policy, and `classify_signals` has many callers.

Tests: the routing-line fixtures exist and are asserted by bucket and rank. Each gains a tuned variant asserting a cluster crosses a bucket boundary when and only when its governing key moves — bucket and ranking position all asserted, since content evidence ([FUSED-CONTENT-GATE]) makes them move together.

Watch for the #197 and #331 fixtures specifically: they are what the defaults are calibrated against, so they must hold at defaults and shift predictably off them.

## Phase 5 — candidate generation, ranking, suppression

`embedding/pairs.rs` (`embedding_min_cosine`, `embedding_top_k`, `embedding_exact_pair_limit`) via `run_embedding_pass`; `cluster.rs` Type-4 dampeners; `report.rs::is_low_structure_embedding_mega_cluster`.

The suppression gate is four literals in one boolean and is the easiest place in the codebase to change accuracy by accident — its tuned test asserts the mega-cluster is suppressed at defaults and surfaces when the ceiling is raised, by cluster id.

## Phase 6 — representation tier and the cache key

`[tuning.representation]` becomes configurable *and* joins the fingerprint cache key in the same change ([CONFIG-TUNING-CACHE]). Splitting these is prohibited: a configurable `kgram_width` against a key that ignores it serves stale fingerprints as fresh, which is a false negative manufactured by the cache.

Tests: analyse, change one representation key, re-analyse incrementally, assert a full cache miss and a report identical to a cold run at the new value. Repeat per key. Plus a `minhash_signature_len % lsh_bands != 0` rejection, and an assertion that `rows_per_band` is derived and has no config key.

## Phase 7 — surfaces

`--tune <table>.<key>=<value>` on the CLI; the VSIX settings channel; the `tuning` block in the JSON report and the one-line statement in HTML and text ([CONFIG-TUNING-DECLARED]); precedence asserted end-to-end at all four levels.

The corpus gate asserts it ran at defaults and refuses a baseline recorded under non-default tuning ([CONFIG-TUNING-DECLARED]).

Docs: `.deslop.toml` reference on the site, and `REPORTING-CONTEXT.md` so agents can read the `tuning` block.

## Phase 8 — close the unrecorded provenance gaps

Six defaults carry **unrecorded** provenance — `embedding_top_k`, `embedding_exact_pair_limit`, `type4_embedding_floor`, `low_structural_type4_ceiling`, `low_structural_type4_weight`, `proven_identical_token_floor` — and two more are **derived but unswept**: `literal_table_min_fraction` and `literal_table_min_literals`, where the argument fixes the direction but not the number.

Now that they are levers, each can be swept against the pinned corpus and resolved to a citation, a defect fixture, or a measured operating point recorded in the [FUSED-TUNING-LEVERS] ledger. Any value the sweep shows to be wrong is a defect under the strict accuracy rule — a failing test and a quarantine, not a quiet edit in this plan.

## Ordering

0 → 1 → 2 gate everything. 3, 4, 5 are independent of each other once 2 lands. 6 is independent but must land whole. 7 needs 3–6. 8 needs 7 for the sweep to be reproducible.
