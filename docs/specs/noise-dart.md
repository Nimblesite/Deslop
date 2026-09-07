# Dart noise filters

## [CLONE-NOISE-DART-WIDGET-SCAFFOLD] Flutter widget declaration scaffolding

Whole declarations of `StatelessWidget`, `StatefulWidget`, and `State<T>` classes are framework scaffolding. A candidate covering only complete widget declarations, optionally with the Flutter launcher, is suppressed by this filter. A range inside a widget body or a range also covering other top-level logic does not qualify. Copied body subtrees remain independently eligible for reporting.

The filter uses parsed nodes and range containment. `crates/deslop-core/src/cluster_filters/dart.rs` implements it; `crates/deslop/tests/issue_331_336_shape_only_saturation.rs` exercises the widget shells alongside copied logic. The general filter and verbatim-family contracts remain in [noise.md](noise.md).
