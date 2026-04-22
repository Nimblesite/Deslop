# Configuration

### [EXCLUSION-CONFIG] Exclusion configuration
A single opt-in configuration file — `.deslop.toml` in the scan root, or `--config <path>` — controls two orthogonal exclusion tiers. Motivating case: generated code. We want to know when hand-written code duplicates a generated file, but we do not want the generated file itself to dominate the top of the report.

**Tiers.**

- `exclude` — matching files are dropped in [PIPELINE-DISCOVER-FILES] before parsing. They are not counted in `files_analysed`, never fingerprinted, never embedded, and cannot appear in any cluster. Use for third-party vendored code you do not want analysed at all.
- `report_hide` — matching files **are analysed** and can contribute to clustering, but each occurrence is flagged `hidden = true` at render time. A cluster where **every** occurrence is hidden is dropped from the rendered `clusters` list and counted under `clusters_hidden`. A cluster with at least one non-hidden occurrence is kept intact so the user sees "regular code duplicates generated code." This is the default tier for generated output like `*.g.cs`, `*.generated.cs`, OpenAPI clients, protobuf output.

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

**No config ⇒ no exclusions.** Current behaviour is preserved. Absence of `.deslop.toml` is not an error and is not warned on.

**`report_hide` membership is a rendering decision, not an analysis one.** Hidden files still participate in fingerprinting, LSH, and (later) embedding. The `hidden: bool` per occurrence is the only surface-level signal of the policy, so downstream consumers that want the unfiltered view can ignore `clusters_hidden` and inspect `occurrences[].hidden` directly.

### [CONFIG-CROSS-LANGUAGE] Cross-language comparison
The same `.deslop.toml` file controls whether clone candidates may span different parser language ids.

```toml
[analysis]
allow_cross_language_comparison = false
```

Default: `false`. Candidate pairs whose two fingerprints belong to different languages are dropped before fusion and transitive-closure clustering. This keeps normal reports focused on code that developers can realistically refactor together and prevents mixed-language scaffolding from dominating the top offenders list.

Opt-in: set `allow_cross_language_comparison = true` to preserve the full language-agnostic candidate union. This is useful for audits that intentionally compare ports, generated client libraries, or semantic equivalents across ecosystems. The option is global for the run; per-language overlays still apply only to exclusion and reporting policy.
