# report-golden — the Phase 0 cold-report golden [PIPELINE-DETERMINISM]

`expected-report.json` is the complete rendered JSON report for a cold `--no-incremental` scan of `src/`, pinned byte-for-byte by `crates/deslop/tests/report_golden.rs`. It is the Phase 0 baseline of `docs/plans/incremental-analysis-plan.md`: every later incremental phase (warm cache, delta re-analysis, live sessions) must reproduce this exact report over this unchanged corpus, so any drift in ranking, spans, cluster ids, metrics arithmetic, or serialisation order fails against this file first.

## The corpus

Five authored Rust files under `src/`, two Type-1 (byte-identical) clusters with different occurrence counts so ranking order is exercised:

- `settle_invoice` — 58-node body, three copies (`alpha.rs`, `beta.rs`, `gamma.rs`), the higher-weight cluster.
- `merge_labels` — 38-node body, two copies (`delta.rs`, `epsilon.rs`), the lower-weight cluster.

Each file also carries one tiny, structurally unique top-level item (const / struct / enum / type alias / static) so the normalised `__file__` nodes differ and no whole-file cluster forms. The fixed flag set is `--no-incremental --embeddings off --min-nodes 16 --notext --nohtml`; 16 sits above the 12-node `settle_invoice` signature subtree (which would otherwise render as a third cluster) and below both clone bodies.

The test never scans this directory in place — it copies `src/` into a throwaway temp root — so no run can drop a `.deslop/` cache here. Editing anything under `src/` invalidates the golden.

## Regenerating

```
DESLOP_BLESS=1 cargo test -p deslop --test report_golden
```

The bless run rewrites `expected-report.json` and then fails on purpose, telling you to re-run without the variable; the plain run must then pass both tests. Regenerating is never the remedy on its own: `committed_golden_satisfies_report_contract` checks the golden against the authored sources themselves (occurrence slices must be byte-identical Type-1 clones inside real corpus files, weights ranked non-increasing, duplicated/analysed LOC and the percentage recomputed independently, cold cache stats zero), so a wrong golden cannot self-certify. Review the diff before committing a re-bless.

## tool_version is embedded

The report deliberately embeds `tool_version` (deslop-core's `CARGO_PKG_VERSION`, `0.0.0-dev` in dev and CI builds). A workspace version bump therefore changes the report bytes and requires re-blessing this golden — with review, like any other re-bless.
