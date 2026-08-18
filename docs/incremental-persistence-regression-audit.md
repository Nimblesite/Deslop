# Incremental persisted-processing regression audit

Audit date: 2026-08-17 (re-audited and resolved the same day)

Scope: the persisted per-fingerprint MinHash work in `docs/plans/incremental-analysis-plan.md` — blob integrity, warm/cold report equivalence, signature reuse, the workspace opt-out, and the resource cost of storing full signatures. Cross-run report diffing is out of scope.

## Verdict

**Merge-ready.** Every finding below is closed in code with matching tests, and the two accuracy defects this audit *uncovered* — one false negative and one false positive, both on the anchor-free routing row — are fixed at the root and pinned from both directions. The store binds every blob to the lookup address, refuses malformed, misplaced, or stale data before serving it, bounds every allocation the decode can drive, and deletes nothing it cannot prove is safe to delete.

Nothing here is accepted as a known-bad. Where a bound is a deliberate ceiling rather than an eliminated risk, it is stated as such below.

## The original findings

| Finding | Status | Evidence |
|---|---|---|
| 1. Corrupted signature payloads served as hits | **Fixed** | Blob format v3 carries a BLAKE3 binding digest over the whole payload, verified *before* any payload byte is decoded. `fpcache::tests::a_flipped_signature_byte_fails_the_binding_digest`; E2E self-heal in `cache_blob_integrity.rs` 4/4. |
| 2. Blob not bound to its content address | **Fixed** | The digest is recomputed from the lookup's own address — language, **tool version**, `min_nodes`, source hash, magic, semantic epoch, signature width — before decode. `a_blob_is_never_served_under_a_different_address` covers all four axes including a cross-version relocation; `the_binding_digest_is_stable_per_address_and_distinct_across_addresses` proves the digest is a function of each field, not merely stable. |
| 3. Store-off accounting made `signature_reuse` red | **Fixed** | `ReuseCounters::assert_store_disabled` asserts `{hits: 0, misses: 0}`, every signature built, and skips store-on conservation. `signature_reuse.rs` **4/4** (the plan's earlier "3/3" was stale). |
| 4. Malformed lengths could panic | **Fixed, and hardened past the original ask** | Counts are proven against remaining bytes before any allocation; `MAX_AST_DEPTH` bounds one path and `MAX_DECODED_NODES` (4 M) bounds the whole tree, claimed per node *including the child slots that follow it* so an absurd child count is refused before its `Vec` is reserved; the read buffer is reserved fallibly. The file read is bounded on the read itself — one handle supplies both the length and the bytes, taken one byte past the 256 MiB ceiling so a file another binary grows mid-read is observable and refused, not silently truncated into a valid-looking prefix. |
| 5. RSS and disk regression | **Fixed** | The per-render flatten — an owned copy of every signature (~157 MiB on tokio), plus fingerprints, trees, and the source map, on *every* render — is deleted. The session owns one canonical flat store (`pipeline/session/store.rs`), spliced per change and borrowed by renders. Warm peak RSS 1,609 → **1,495 MB**. Disk is bounded by [PIPELINE-INCREMENTAL-RETENTION] (below). |
| 6. LSH-only equivalence coverage | **Fixed** | `lsh_only_nearmiss_recall.rs` now carries the route-specific matrix: an embeddings-off Python pair with `structural = 0.00` that can only enter through LSH, asserted across cold, fully warm, a mixed pass, and a revert — exact bucket, signal triple, files, spread, ranking, and re-derived `duplication_percent` on every state, reports byte-equal modulo `cache_stats`, and the mixed pass pinned to an **exact** rebuild/reuse split derived from a one-file measurement rather than a hardcoded number. |
| 7. Semantic invalidation under `0.0.0-dev` | **Fixed** | `SEMANTIC_EPOCH`, `MAGIC`, and `SIGNATURE_LEN` are bound into the digest; superseded magics and trailing data are rejected. |
| 8. Architecture-dependent fallback signatures | **Fixed** | Byte offsets widened to `u64` before hashing; pinned against fixed slots. |
| Workspace opt-out | **Fixed** | `[analysis] incremental = false` gates the effective setting and store creation. `session_config().incremental` now reports the *effective* mode via `PipelineSession::effective_incremental`, MCP forwards it unchanged, and `live_session_status.rs` 3/3 pins the toggle-under-opt-out transition and the store never being created. |

## Accuracy defects this audit uncovered

The LSH-only fixture finding 6 asked for did its job twice: it exposed a false negative, and fixing that exposed a false positive. Both are on [CLONE-BUCKETS-ROUTING] row 4, and both are now pinned from opposite sides so neither can be traded for the other.

**False negative — row 4 was implemented for one language (gh #390).** An authored Python pair (`structural = 0.00`, `token_jaccard = 0.9297`, endpoints past every LSH-only survival floor) rendered **zero** duplication: `classify_signals` had no row-4 arm, so the triple fell to `LooselySimilar`, which the renderer hides, while `report_render::is_csharp_lsh_type3_near_miss` patched the row for C# members only. The carve-out is dissolved into the router — row 4 now routes in every language, as the spec always said.

**False positive — row 4 reached an act-now bucket on no evidence.** Six distinct Flutter widgets measure `structural = 0.00, token_jaccard = 0.93` over whole-file spans whose `build` bodies share nothing — the framework-mandated declaration is most of each file — and once row 4 routed in every language they were reported at `fused = 0.93` as "nearly identical, review the locations" (#331). The same door admitted #108's JSON-schema pair at `token_jaccard = 0.96`.

The obvious fix — send row 4 through [FUSION-CONTENT-GATE] like every other route that rests on the normalised representation — is **wrong, and was measured to be wrong before being discarded.** Both of that gate's populations assume the members align position for position, and `structural ≤ 0.01` says the shapes differ. Against the `csharp-type3` fixture — a genuine Type-3 clone with *every* identifier renamed and one extra statement — agreement collapses to 0.19 (the literals) and rename consistency to 0.00, because the extra statement destroys the alignment the rename proof needs. Gating row 4 on content demoted that pair to `structural_only` at `fused = 0.17`: a false negative on the most valuable clone class there is, traded for #331's precision. `cli/detection.rs::detects_type3_clone_in_csharp_fixture` caught it.

`ContentEvidence::substance_varies` fails the same fixture for the same reason — it reads `true` for a consistent rename whose extra statement breaks the alignment — so it is not a narrower substitute either. Any future narrowing needs a discriminator that survives `csharp-type3`.

Row 4 is routed on cluster **spread** instead, and only two shapes are demoted:

1. **A cross-file spread** (3+ members over 3+ files) — the #134 scaffolding pattern arriving through the token door instead of the structural one. This is the widget family. **This is a trade, not a free win:** a genuine clone family that wide is demoted to a hint too, exactly as `is_cross_file_scaffolding` already does for shape-identical spreads, for the same stated reason. It is recorded in the function doc, the taxonomy row, and the PR body rather than left implicit.
2. **An unmeasured cluster**, where the content pass could not compare two members at all. The anchored routes may take one on trust because their Merkle equality is itself proof; row 4 has no such signal, so unmeasured there means *nothing is known*. This is #108. `ContentEvidence` gained a `measured` flag to express it, because "measured full agreement" and "nothing was measured" were previously the same value.

Both demote to `LooselySimilar`, which the renderer hides — never `StructuralOnly`, which would claim a shape match `structural = 0.00` says does not exist. A measured *pair* is left alone even at low agreement: that is the renamed Type-3 clone.

Four suites now pin the four corners, and no two can be satisfied by loosening a threshold: `lsh_only_nearmiss_recall.rs` (genuine LSH-only pair keeps `fused ≥ 0.85`), `cli/detection.rs` (the renamed C# pair still surfaces at `structural = 0.00`), `issue_331_336_shape_only_saturation.rs` (the scaffold family stays below the act-now line), and `issue_98_99_108_120_122_thresholds.rs` (unmeasured noise stays out of the ranked report entirely).

## Retention, and what a bound means here

[PIPELINE-INCREMENTAL-RETENTION] runs after every full store-on pass, the one moment the addressable blob set is exactly known. Under budget it deletes **nothing**: an orphan is the content-addressed set a revert or branch switch full-hits (the equivalence suite asserts exactly that), and a blob under another tool version may belong to a second binary sharing the workspace — an installed VSIX's LSP beside a freshly-built CLI — which two mutually-sweeping binaries would deadlock into permanent rebuild churn. Over the 2 GiB budget, eviction is by class (other-version, then orphan, then live), then oldest-first, path as tie-break. Class outranks age in both directions.

Evicting any blob is correctness-free: the next pass that addresses it misses, rebuilds from source, and self-heals. That is why the budget is a hard bound rather than an accuracy surface.

Two ceilings are deliberate ceilings, not eliminated risks, and both degrade to a plain miss:

- **256 MiB per blob** and **4 M decoded nodes** are far past anything a real source file produces. A file that somehow exceeded either would never cache — it would re-parse every pass. That is a cost, never a wrong answer.
- Both bounds sit *behind* the digest, so ordinary corruption never reaches them: it fails verification first. They exist for a payload whose digest checks out — an encoder bug, or a store an attacker can already write to, in which case they can equally edit the source the tool reads.

## Verification

- `make ci` — full workspace gate (fmt, clippy `-D warnings`, tests with coverage thresholds, build).
- Accuracy suites re-run after every fix in this round: `lsh_only_nearmiss_recall` 2/2 · `issue_331_336_shape_only_saturation` 3/3 · `issue_98_99_108_120_122_thresholds` 1/1 · `cache_retention` 3/3 · `fpcache` unit group 21/21 (14 in `fpcache::tests`, 7 in `fpcache::retention::tests`).
- Incremental suites: `signature_reuse` 4/4 · `cache_blob_integrity` 4/4 · `incremental_equivalence` 6/6 · `incremental_multilang_golden` 3/3 · `incremental_multilang_matrix` 4/4 · `live_session_status` 3/3.
- Real-repo lifecycle proof on `deslop-core/src` (135 files, release): cold 1.03 s (0 hit / 135 miss, 29,120 signatures built) → warm 0.43 s (135 hit / 0 miss, **0 built / 29,120 reused**) → edit one file 0.43 s (134 hit / 1 miss, only 165 signatures rebuilt) → revert full-hits. Cold, warm, and revert are byte-equal to the `--no-incremental` report modulo `cache_stats`; the store survives process death (136 blobs / 34 MiB).
- Pinned tokio benchmark (release, default `min-nodes`, `--embeddings off`): `--no-incremental` 6.45 s / 1,532 MB · cold 6.48 s / 1,550 MB · **warm 3.31 s / 1,495 MB** · one-file edit 3.39 s · revert 3.42 s · store 185.8 MiB / 759 blobs.

## Follow-ups (tracked in the plan, not blockers)

Everything downstream of signatures — band enumeration, pairing, clustering, ranking, metrics, rendering — still recomputes corpus-wide on every pass. Making that cost track the size of the change is the remaining work of [PIPELINE-INCREMENTAL-ANALYSIS] (gh #383); the re-measured attribution in the plan's Phase 3 names banding (~44%) as the next target.
