# Decisions with fallback rules

### [DECISION-MIN-NODES] Minimum subtree size

Default `--min-nodes` = **30**. Subtrees below this threshold are excluded from fingerprinting, clustering, and embedding. Rationale: smaller subtrees (`return x;`, single-statement blocks) are noise and dominate the report.

**Tuning procedure** (run once per release cycle against at least three real-world C# repos — production backend, test-heavy library, generated-code-heavy repo):

1. Run with `--min-nodes` at each of `{15, 20, 25, 30, 40, 50}`. Save the JSON reports.
2. For each run, inspect the top-20 clusters by `weight`. Classify each cluster as:
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
