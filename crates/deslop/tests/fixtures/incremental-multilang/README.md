# incremental-multilang — the six-language incremental golden [PIPELINE-INCREMENTAL]

`expected-report.json` is the complete rendered JSON report for a cold `--no-incremental` scan of `src/`, pinned byte-for-byte by `crates/deslop/tests/incremental_multilang_golden.rs`. `crates/deslop/tests/incremental_multilang_matrix.rs` drives the states between cold and warm against the same corpus.

## Why six languages in one directory

The parse store keys on `(language_id, tool_version, min_nodes, blake3(source))`. A single-language corpus cannot distinguish a correct key from one that has lost its language component, and cannot catch a store that serves one language's tree out of another language's slot. Both faults keep rendering a plausible report while attributing one language's clone to another's subtree — a false positive and a false negative in the same pass. Only a mixed corpus sharing one store can pin that.

The directory is a detector fixture, not a build target: twelve sources from six languages sit side by side, and the two Go files intentionally declare the same `ReconcileEntries` in one package. Nothing here is compiled.

## The corpus

One authored Type-1 clone pair per language, twelve files, all byte-distinct:

| Language | Canonical | Pasted |
| --- | --- | --- |
| Rust | `ledger_alpha.rs` | `ledger_beta.rs` |
| Python | `ledger_alpha.py` | `ledger_beta.py` |
| TypeScript | `ledger_alpha.ts` | `ledger_beta.ts` |
| Dart | `ledger_alpha.dart` | `ledger_beta.dart` |
| C# | `LedgerAlpha.cs` | `LedgerBeta.cs` |
| Go | `ledger_alpha.go` | `ledger_beta.go` |

Within a pair the `reconcile_entries` body is byte-identical, so exactly one `identical` cluster of size 2 must form. The two files differ only in a leading banner comment plus one structurally unique top-level item (const / struct / class / interface / record), which keeps the file bytes distinct — the store is content-addressed, so byte-identical files would share one blob and the second would hit inside the cold run — and keeps the normalised `__file__` nodes distinct so no whole-file cluster forms.

The fixed flag set is `--min-nodes 20 --embeddings off --notext --nohtml`, with `--no-incremental` for the golden itself. Every authored clone measures 40–57 nodes, so 20 keeps all six clusters — and it deliberately sits above 13, because at lower floors the C# pair renders a second `identical` cluster: a 13-node sibling window over the method's signature line that straddles [PIPELINE-CLUSTER-SUBSUME] containment by the 7 bytes of the `public` modifier (gh #389). This fixture's subject is the parse store, not subsumption, so the floor keeps that edge out of every report here.

Neither suite scans this directory in place — both copy `src/` into a throwaway temp root — so no run can drop a `.deslop/` cache here. Editing anything under `src/` invalidates the golden.

## Regenerating

```
DESLOP_BLESS=1 cargo test -p deslop --test suite incremental_multilang_golden::
```

The bless run rewrites `expected-report.json` and then fails on purpose, telling you to re-run without the variable. Regenerating is never the remedy on its own: `committed_multilang_golden_satisfies_the_authored_contract` checks the golden against the authored sources themselves — every occurrence must slice back out of `src/` and match its sibling byte-for-byte, every language must appear exactly once, weights must rank non-increasing, and the cold cache counters must be zero — so a golden blessed while a language was silently missing, or while the store was cross-serving trees, fails even though its bytes match.

## tool_version is embedded

Like `report-golden`, the report embeds `tool_version`, so a workspace version bump changes the bytes and requires a reviewed re-bless.
