# Isolated performance benchmarks

These benchmarks construct deterministic in-memory workloads. They do not discover files, parse a repository, or scan the Flutter corpus.

Run the baseline before changing the hot path, then pass its label to the after run. Each command writes seven raw timing samples, a median, and a result checksum under `target/perf-artifacts/`. Before/after speedups are calculated by Rust and included in the second JSON artifact.

## Shared-subtree alignment

```sh
node scripts/benchmarks/shared-subtree-alignment.mjs before 100
node scripts/benchmarks/shared-subtree-alignment.mjs after 100 before
```

The optional fourth argument selects `flat`, `chain`, or `balanced`; the default is `all`.

## Ranked cluster signals

```sh
node scripts/benchmarks/cluster-signals.mjs before 5
node scripts/benchmarks/cluster-signals.mjs after 5 before
```

This workload recreates the profiled 877-member exact group: 384,126 logical occurrence pairs and 49,168,128 logical `MinHash` slot comparisons per repetition.
