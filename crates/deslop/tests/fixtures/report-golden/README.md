# report-golden — the Phase 0 cold-report golden [PIPELINE-DETERMINISM]

`expected-report.json` is the complete rendered JSON report for a cold `--no-incremental` scan of `src/`, pinned byte-for-byte by `crates/deslop/tests/report_golden.rs`. It is the baseline every reuse path owes ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]): a warm cache, a spliced live session and a delta re-analysis must all reproduce this exact report over this unchanged corpus, so any drift in ranking, spans, cluster ids, metrics arithmetic, or serialisation order fails against this file first.

## The corpus

Five authored Rust files under `src/`, two Type-1 (byte-identical) clusters with different occurrence counts so ranking order is exercised:

- `settle_invoice` — 72-node declaration (68-node function plus the shared `RegionMarker`), three copies (`alpha.rs`, `beta.rs`, `gamma.rs`), the higher-weight cluster. The span covers the whole `pub fn` plus the marker, not the body alone: an occurrence that starts at the function *name* is not extractable, and pinning one was how a 7-byte truncation stayed invisible.
- `merge_labels` — 44-node declaration (38-node body plus the shared `REGION_FLAG`), two copies (`delta.rs`, `epsilon.rs`), the lower-weight cluster.

Each file also carries one tiny top-level item — the same `pub struct RegionMarker;` in every `settle_invoice` file and the same `pub static REGION_FLAG: bool = true;` in both `merge_labels` files — so the files stay byte-distinct (the banner comments differ) while the wider same-file view the overlap collapse elects remains byte-identical across the pair ([PIPELINE-CLUSTER-EXACT-SCOPE]: scope and width decide the view, so the marker is part of every reported occurrence). The marker is too small to fingerprint, so no extra cluster forms. The fixed flag set is `--no-incremental --embeddings off --min-nodes 16 --notext --nohtml`; 16 sits below both clone declarations (72 and 44 nodes). The `settle_invoice` signature subtree never renders as a third cluster — the declaration around it contains it, so [PIPELINE-CLUSTER-SUBSUME] elects it away.

The test never scans this directory in place — it copies `src/` into a throwaway temp root — so no run can drop a `.deslop/` cache here. Editing anything under `src/` invalidates the golden.

## Regenerating

```
DESLOP_BLESS=1 cargo test -p deslop --test suite report_golden::
```

The bless run rewrites `expected-report.json` and then fails on purpose, telling you to re-run without the variable; the plain run must then pass both tests. Regenerating is never the remedy on its own: `committed_golden_satisfies_report_contract` checks the golden against the authored sources themselves (occurrence slices must be byte-identical Type-1 clones inside real corpus files, weights ranked non-increasing, duplicated/analysed LOC and the percentage recomputed independently, cold cache stats zero), so a wrong golden cannot self-certify. Review the diff before committing a re-bless.

## tool_version is embedded

The report deliberately embeds `tool_version` (deslop-core's `CARGO_PKG_VERSION`, `0.0.0-dev` in dev and CI builds). A workspace version bump therefore changes the report bytes and requires re-blessing this golden — with review, like any other re-bless.
