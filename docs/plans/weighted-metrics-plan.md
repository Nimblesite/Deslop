# Remove evidence-weighted metrics — wholesale deletion plan

[METRICS-REPO-WEIGHTED](../specs/pipeline.md#metrics-repo-weighted) and [EXIT-CODES-WEIGHTED](../specs/pipeline.md#exit-codes-weighted) prohibit evidence-weighted repository figures and gates. Pair evidence cannot be projected onto a closure component or a covered line. This plan deletes the design and every partial implementation in one cutover; it does not preserve a dormant configuration or compatibility wire field.

## Contract

- The repository carries one duplication percentage: `100 × duplicated_loc / analysed_loc`, with the zero-denominator rule in [METRICS-REPO](../specs/pipeline.md#metrics-repo).
- Each duplicated line counts once. Pair evidence, pair classification, finding kind, confidence, and severity do not scale it.
- The repository has one threshold gate: `--fail-over` or `[threshold] max_duplication_percent`. No weighted gate exists.
- Cluster mass is [RANK-MASS-SUM], not a metric percentage and not an input to `duplication_percent`.

## One destructive removal

- [x] Remove the weighted metric, weighted gate, weight tables, and multipliers from the governing specifications.
- [ ] Delete `WeightedMetrics`, weighted file/folder fields, resolved weight-table fields, and weighted threshold fields from `docs/models/live-ipc.td`; regenerate every Rust and TypeScript model.
- [ ] Delete `[metrics.bucket_weights]`, `[metrics.category_weights]`, validation, defaults, CLI overrides, environment plumbing, and serialization.
- [ ] Delete `--fail-over-weighted`, `max_weighted_duplication_percent`, two-gate resolution, weighted verdicts, and weighted exit-code branches.
- [ ] Delete weighted numerator calculation, per-line evidence lookup, bucket/category weight lookup, weighted folder rollup, and every helper used only by them.
- [ ] Delete weighted text, HTML, JSON, LSP, MCP, VSIX, site, and documentation rendering. Do not leave hidden fields, empty placeholders, deprecated aliases, or no-op switches.
- [ ] Replace tests that assert weighted output with strict negative schema tests and positive mechanical-metric tests; never remove the underlying coverage assertions.

## Proof

- [ ] Black-box CLI tests assert exact `analysed_loc`, `duplicated_loc`, `duplication_percent`, per-file values, folder values, threshold source, breach flag, exit code, and rendered text/HTML.
- [ ] Configuration tests reject every retired weighted key and flag with a named invalid-configuration or invalid-argument error.
- [ ] Generated-model tests prove no weighted field or table exists in Rust or TypeScript.
- [ ] Regression tests prove changing pair evidence without changing visible occurrence coverage cannot change repository metrics.
- [ ] Repository-wide searches find no executable weighted-metric type, field, flag, config key, renderer, or compatibility branch.
- [ ] Full CI and installed VSIX verification pass with the one mechanical metric on every surface.

## Completion

This plan is complete when the weighted design is absent from the wire, engine, configuration, CLI, renderers, clients, tests, and installed VSIX, and the one unweighted repository metric is identical on every surface.
