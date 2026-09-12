# Dart noise filters

## [CLONE-NOISE-DART-WIDGET-SCAFFOLD] Flutter widget declaration scaffolding

Complete `StatelessWidget`, `StatefulWidget`, and `State<T>` declarations qualify as framework scaffolding only when their bodies contain construction expressions and returns. Local bindings, control flow, arithmetic, mutation, closures, and unknown syntax prevent suppression. Exact declaration copies and copied `build` bodies remain clones even when widget names change. Superclass names must match exactly. A `main` function containing only a `runApp` call may accompany these shells. A range inside a widget body or covering other top-level logic does not qualify.

When copied layouts share a candidate group with unrelated widgets, keep only members with the same complete, ordered `build` bodies. The existing family split recomputes category, weight and counts from that subgroup. The later noise check uses the same copy proof.

The filter uses parsed nodes and range containment. `crates/deslop-core/src/cluster_filters/dart.rs` implements it; `crates/deslop/tests/issue_331_336_shape_only_saturation.rs` exercises the widget shells alongside copied logic. The general filter and verbatim-family contracts remain in [noise.md](noise.md).
