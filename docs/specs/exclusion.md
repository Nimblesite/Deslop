# Configuration

### [EXCLUSION-CONFIG] Exclusion configuration
Deslop ships with conservative built-in defaults, and a `.deslop.toml` in the scan root, or `--config <path>`, extends those defaults with project-specific rules. Motivating case: generated code. We want to know when hand-written code duplicates a generated file, but we do not want generated files or build outputs to dominate the top of the report.

**Tiers.**

- `exclude` — matching files are dropped in [PIPELINE-DISCOVER-FILES] before parsing. They are not counted in `files_analysed`, never fingerprinted, never embedded, and cannot appear in any cluster. Use for third-party vendored code you do not want analysed at all.
- `report_hide` — matching files **are analysed** and can contribute to clustering, but each occurrence is flagged `hidden = true` at render time. A cluster where **every** occurrence is hidden is dropped from the rendered `clusters` list and counted under `clusters_hidden`. A cluster with at least one non-hidden occurrence is kept intact so the user sees "regular code duplicates generated code." This is the default tier for generated output like `*.g.cs`, `*.generated.cs`, OpenAPI clients, protobuf output.

**Built-in defaults.** Without a config file, Deslop excludes common dependency/build cache directories (`node_modules`, `target`, `dist`, `build`, `.venv`, `__pycache__`, `.cargo`) and report-hides generated output (`generated` path components, Alembic migration files under `alembic/versions`, plus suffixes such as `.g.cs`, `.generated.cs`, `.designer.cs`, `.pb.cs`, `.openapi.cs`, `.generated.py`, `_generated.py`, `_pb2.py`, `_pb2_grpc.py`). Project config adds to these defaults. `.cargo` blocks Cargo's vendored registry and git-checkout caches from entering discovery even when a misconfigured scan root reaches into the user's home directory.

**File format.** TOML. Parsed via the `toml` crate. Minimal, familiar, diffable:

```toml
[defaults]
exclude = ["vendor/**", "third_party/**"]
report_hide = ["**/*.generated.cs", "**/*.g.cs"]

[language.csharp]
report_hide = ["**/Migrations/**/*.cs"]

[language.rust]
report_hide = ["**/target/**"]
```

**Pattern semantics.** `ignore::gitignore` syntax. Same engine as [PIPELINE-DISCOVER-FILES] so patterns behave identically to `.gitignore`. Paths are matched relative to the scan root.

**Merge rule.** Per-language sections **extend** `[defaults]`, they do not replace it. A `.rs` file is checked against `defaults.report_hide ∪ language.rust.report_hide`. Keeps the config declarative — you never have to repeat shared patterns in every language block.

**No config is valid.** Absence of `.deslop.toml` is not an error and is not warned on; Deslop still applies the built-in generated/build filters above.

**`report_hide` membership is a rendering decision, not an analysis one.** Hidden files still participate in fingerprinting, LSH, and (later) embedding. The `hidden: bool` per occurrence is the only surface-level signal of the policy, so downstream consumers that want the unfiltered view can ignore `clusters_hidden` and inspect `occurrences[].hidden` directly.

### [CONFIG-CROSS-LANGUAGE] Cross-language comparison
The same `.deslop.toml` file controls whether clone candidates may span different parser language ids.

```toml
[analysis]
allow_cross_language_comparison = false
```

Default: `false`. Candidate pairs whose two fingerprints belong to different languages are dropped before fusion and transitive-closure clustering. This keeps normal reports focused on code that developers can realistically refactor together and prevents mixed-language scaffolding from dominating the top offenders list.

Opt-in: set `allow_cross_language_comparison = true` to preserve the full language-agnostic candidate union. This is useful for audits that intentionally compare ports, generated client libraries, or semantic equivalents across ecosystems. The option is global for the run; per-language overlays still apply only to exclusion and reporting policy.
