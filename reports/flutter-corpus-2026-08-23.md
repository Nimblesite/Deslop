# Flutter corpus run

Run date: 2026-08-23 (Australia/Sydney)

The Flutter corpus was run against a clean release binary. The scan was stopped at the requested 20-minute limit before the test emitted its final result.

| Metric | Result | Notes |
|---|---:|---|
| Corpus | Flutter | Pinned checkout `67323de285b0` |
| Binary | Clean release build | `cargo clean` completed first |
| Release build time | 1m 29s | `cargo test --release` compilation |
| Scan wall time | 20m 00s limit reached | Process killed at the deadline |
| Deslop CPU time | ~1,200s | Approximately one logical CPU equivalent during the scan |
| Observed normalized CPU sample | ~4% | Point-in-time host-normalized sample; not an average |
| Files analysed | Not emitted | Scan timed out before the result line |
| Cluster count | Not emitted | Scan timed out before the result line |
| Duplicated lines / percentage | Not emitted | Scan timed out before the result line |
| Exit status | Timed out / killed | No corpus pass/fail result was produced |

Command run:

```text
cargo clean
node scripts/corpus/fetch-corpus.mjs flutter
cargo test --release -p deslop --test corpus_repos corpus_flutter_dart -- --ignored --exact --nocapture --test-threads=1
```

The repository checkout was already present at the pinned revision, so fetching did not download new objects.
