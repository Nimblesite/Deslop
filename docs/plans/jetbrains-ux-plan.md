# JetBrains Native UX Plan

## Scope

Add native Rider / IntelliJ UX on top of the existing `deslop-lsp` process.

The plugin must stay thin. Kotlin owns editor integration, settings UI, and
tool windows only. Clone detection, ranking, report schema, bucket labels, and
embedding model discovery stay in Rust behind the LSP custom methods.

## Implementation Notes

- Use native JetBrains surfaces instead of porting VSIX webviews.
- The Tool Window must consume `deslop/reportGet`; it must not re-rank or
  re-bucket clusters.
- The model picker must call `deslop/embeddingListModels` and
  `deslop/embeddingSetModel`.
- Do not parse hover markdown to recover structured data. Use custom LSP
  methods for structured report data.

## TODO

- [ ] Add `Duplicate Clusters` Tool Window.
- [ ] Add Top Offenders tab using report order from `deslop/reportGet`.
- [ ] Add Focused File tab filtered by the active editor path.
- [ ] Add Session tab showing active model, cache stats, files analysed, and
      analysis state.
- [ ] Add navigation from Tool Window rows to source occurrences.
- [ ] Add cluster detail view with bucket label, signals, interpretation, and
      occurrence list.
- [ ] Add native embedding model picker backed by `deslop/embeddingListModels`.
- [ ] Persist selected model through the shared settings contract and call
      `deslop/embeddingSetModel`.
- [ ] Surface embedding refresh progress without blocking editor typing.
