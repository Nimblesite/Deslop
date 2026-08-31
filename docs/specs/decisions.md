# Decisions with fallback rules

### [DECISION-MIN-NODES] Minimum subtree size

Default `--min-nodes` = **30**. Subtrees below this threshold are excluded from fingerprinting, clustering, and embedding. Rationale: smaller subtrees (`return x;`, single-statement blocks) are noise and dominate the report.

**Tuning procedure** (run once per release cycle against at least three real-world C# repos — production backend, test-heavy library, generated-code-heavy repo):

1. Run with `--min-nodes` at each of `{15, 20, 25, 30, 40, 50}`. Save the JSON reports.
2. For each run, inspect the top-20 clusters by `mass`. Record whether each reported component is genuine duplication, noise, or uncertain without assigning a pair classification to the component.
   - **Signal** — a real duplication a reasonable reviewer would want to deduplicate.
   - **Noise** — boilerplate (single-statement clones, trivial getters, default `ToString` overrides, test-only assertions repeated across many tests).
3. The best default maximises **signal-in-top-20 / 20** across the three corpora. Break ties by picking the lower `--min-nodes` (better Type-3 recall).
4. Also check `clusters.len()` and the `cache_stats.misses` runtime — a default that produces >10 000 clusters or >3x the runtime of the next tier is hostile to the CLI workflow regardless of precision.
5. If the chosen default changes, bump the default in `crates/deslop/src/main.rs`, note the corpus and date in this decision, and re-run `tests/cli.rs` — several fixture-driven tests hard-code `--min-nodes` in their arg strings and will need their values revisited.

**Guardrails** (apply to any ship, with or without a tuning run):

- Never ship a default below **15** (catches fragment-level noise that overwhelms readers) or above **60** (misses entire short methods).
- Always keep the flag user-overridable; document it in `--help`.
- Publish the tuning corpus signature (repo, commit, LOC, language mix) next to any default change so the decision is reproducible.

### [DECISION-CROSS-LANGUAGE] Cross-language clones

Supported but opt-in. The normalization format is identical across languages, so the candidate union can compare fingerprints from different parser language ids without a second pipeline. However, the default user workflow is same-language refactoring, and mixed-language matches are often ports, generated clients, or syntax scaffolding rather than extractable duplication.

Default: cross-language comparison is disabled via [CONFIG-CROSS-LANGUAGE]. Users may enable it for audit scenarios, but the pipeline must not add heuristics, mappings, or type-system bridges until cross-language clone detection becomes an explicit feature goal.

### [DECISION-TYPE3-TWO-PASS] Type-3 recall via AST sibling-extension + token LSH

Ship both passes. Sibling-extension runs first because it is cheaper and produces byte-range-accurate matches. Token LSH runs second and surfaces Type-3 candidates whose structure diverged too far for sibling-extension. Fallback rule: if the LSH pass contributes fewer than 5% additional clusters on three consecutive representative corpora (measured in a calibration run), mark it as a future removal candidate and raise the issue — do not silently disable it.

### [DECISION-LITERALS] Literal & constant duplication is a first-class finding family

**Why it was missing.** The original research grounding sampled one lineage — academic
fragment-clone detection (Baxter, Chilowicz, SourcererCC, NiCad, Roy/Cordy Type-1..4) — whose unit
of analysis is the code fragment. Duplicate-literal findings live in a parallel industrial lineage
(PMD `AvoidDuplicateLiterals`, SonarSource S1192/S109, Checkstyle `MagicNumber`, ESLint
`no-magic-numbers`, goconst) that the landscape survey in [comparison.md](comparison.md) never sampled. Two
pipeline mechanisms then made literals structurally undetectable, each sufficient alone:
[PIPELINE-NORMALIZE-AST] rewrites every literal to `__literal__` before any fingerprint exists, and
[DECISION-MIN-NODES] floors fingerprinting at 30 nodes while a literal is a 1-node leaf. The
exclusion was an inherited blind spot, never a recorded decision — this file held exactly three
decisions and zero mention of literals — and the blind spot was reinforced every time literal-driven
repetition leaked through and was answered with a suppression (`data` demotion, structural-only
demotion, the #61→#169 cluster-filter lineage), encoding "literal repetition = false positive"
instead of "missing finding family".

**The decision.** Ship the value-level family per [literals.md](literals.md): capture literal
identity as a side-channel during the existing walk (never weaken the `__literal__` collapse — Type-2
depends on it), classify **outside** Type-1..4 via the category axis ([CLONE-CATEGORY-REGISTRY]),
and keep [DECISION-MIN-NODES] intact for fragment clones with this family as the documented
carve-out (size floors guard structural matching, not value indexing — micro-clone literature shows
sub-floor fragments carry *more* maintenance burden, not less).

**Evidence-pinned defaults** (sources in [reading-list.md](reading-list.md#read-list-literals)):
duplicate **strings** ship on by
default (S1192 is the only literal rule any major vendor enables by default, across four languages
incl. Dart); magic **numbers** ship opt-in (unanimous vendor verdict: S109 in no default profile,
ESLint rule frozen, clippy's standing refusal, go-mnd opt-in); string threshold 3 occurrences /
5 content chars (Sonar + goconst; PMD's 4 is the outlier); numeric ignore set `{-1, 0, 1, 2}` ∪
`{0.0, 1.0}` (the convergent core of every shipping allowlist); constant-drift ranking grounding is
**transferred by inference** from Engler SOSP 2001 / CP-Miner / Juergens ICSE 2009 — no direct study
of named-constant divergence exists. Defaults only ship enabled while the [LITERAL-CENSUS] gate
holds; the census numbers are recorded here.

**Census tuning procedure** (mirrors the [DECISION-MIN-NODES] procedure): run with default config
over the reference corpora (this repository's crates + the multi-language fixture corpus); classify
each literal finding kind's top 20 results as signal or noise; adjust the [LITERAL-NOISE] floors and ignore
sets and re-run until the [LITERAL-CENSUS] bound holds; record the corpus signature (repo, commit,
LOC, language mix) and per-category counts next to this decision so the calibration is
reproducible.

### [DECISION-MCP-SURFACE] Seven core MCP analysis tools

> **Status: ⏳ Wholesale cutover.** The target contract is the seven-tool core analysis surface in [MCP-TOOLS]. The retired twelve-tool analysis-query surface is not a compatibility mode.

The old surface grew by accretion: overlapping report slicers and duplicate path-scoped calls exposed the same analysis through incompatible shapes. The replacement is exactly `find-similar`, `duplicates`, `compare-pair`, `cluster-by-id`, `rescan`, `session`, and `schema-doc`. `duplicates` owns cluster queries, while `compare-pair` is the only route to exact-endpoint admission evidence. Cluster filters cover cluster-owned language, path, canonical extent, and engine-stamped mass severity only; pair classification and literal finding kind never enter that filter block. Refactor tools specified by [AUTOFIX-MERGE-MCP] and [AUTOFIX-EXTRACT-AI-MCP-TOOLS] remain orthogonal and do not create alternate report or pair-evidence paths. There is no fallback surface.
