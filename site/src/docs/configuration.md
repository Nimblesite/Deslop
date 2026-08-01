---
layout: layouts/docs.njk
title: Configuration Reference — .deslop.toml and CLI flags
description: Every Deslop knob in one place — the .deslop.toml sections (exclude, report_hide, threshold, ranking, analysis, report), the built-in rules you can't turn off, every CLI flag, and the precedence between them.
eleventyNavigation:
  key: Configuration
  order: 6
icon: tune
---

# Configuration Reference

Deslop is configured two ways:

- **`.deslop.toml`** — a committed file next to your code. This is where project-wide policy lives: what to skip, what to hide, and when to fail CI.
- **CLI flags** — per-run overrides. A flag always wins over the matching config key.

Everything on this page is a reference. For a guided walkthrough, start with [Getting Started](/docs/).

## The `.deslop.toml` file

### Where it lives

Deslop looks for `.deslop.toml` **next to the scan root** — the directory you point it at. Point it elsewhere with `--config`:

```bash
deslop .                          # uses ./.deslop.toml if present
deslop ./src --config ci.toml     # explicit config, any name
```

A missing config is not an error — Deslop runs with the built-in rules below.

### Format

TOML, with one shared `[defaults]` block plus optional per-language overlays and four behaviour sections. Patterns are **glob patterns with gitignore semantics**, resolved **relative to the scan root** — `subdir/**` matches `<scan-root>/subdir/...` regardless of where the repo sits on disk.

### `exclude` vs `report_hide` — the core idea

Two tiers do different jobs, and the difference matters:

| | What happens | Use it for |
| --- | --- | --- |
| `exclude` | The file is **dropped before analysis** — never parsed, never fingerprinted. | Vendored code, build output, anything you never want compared. |
| `report_hide` | The file **is analysed**, but every occurrence is marked hidden. A cluster is dropped from the report **only when all of its members are hidden**. | Generated code. You still want to know when *hand-written* code duplicates a generated file. |

The asymmetry is deliberate. `report_hide` keeps "generated code duplicates itself" out of your headline while still surfacing "your code duplicates generated code" — because that cluster has at least one non-hidden member, so it survives.

## `[defaults]`

Applied to every file regardless of language.

```toml
[defaults]
exclude = [
  "**/node_modules/**",
  "**/dist/**",
  "vendor/**",
]
report_hide = [
  "**/*.snapshot.cs",
]

[defaults.boilerplate]
imports = "report"
```

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `exclude` | list of globs | `[]` | Files dropped from discovery. Additive with the built-in excludes. |
| `report_hide` | list of globs | `[]` | Files analysed but hidden from the rendered report. Additive with the built-in hide rules. |
| `boilerplate.imports` | `"suppress"` \| `"report"` | `"suppress"` | How import / prologue-only clones are handled. `suppress` drops them silently; `report` still suppresses the clone warning but emits a low-severity hygiene hint. |

## `[language.<name>]`

Per-language overlays. The name is the parser's language id: `csharp`, `rust`, `python`, `dart`, `javascript`, `typescript`, `tsx`, `php`, `fsharp`, or `go`. Patterns here **extend** the defaults — they never replace them — so a `.cs` file is tested against `defaults.exclude` **plus** `language.csharp.exclude`.

```toml
[language.csharp]
report_hide = ["**/Migrations/**"]

[language.python]
exclude = ["**/conftest.py"]

[language.dart.boilerplate]
imports = "report"
```

Each overlay accepts the same three keys as `[defaults]`: `exclude`, `report_hide`, and `boilerplate.imports`.

## `[threshold]` — the CI gate

```toml
[threshold]
max_duplication_percent = 20
```

| Key | Type | Range | Meaning |
| --- | --- | --- | --- |
| `max_duplication_percent` | float | `0.0`–`100.0` | When repo-wide `duplication_percent` exceeds this, `deslop` exits **`3`** and fails CI. |

This is the **only** opt-in failure path. With no `[threshold]` block (and no `--fail-over`), `deslop` always exits `0` no matter how much duplication it finds. The CLI `--fail-over` flag overrides this key; `--no-fail-over` clears it for a single run. See [Output Formats → Exit codes](/docs/output-formats/#exit-codes).

## `[analysis]`

```toml
[analysis]
allow_cross_language_comparison = false
```

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `allow_cross_language_comparison` | bool | `false` | When `true`, candidate clone pairs may span different languages. Off by default, so reports stay focused on same-language refactoring. |

## `[report]`

```toml
[report]
split_by_language = false
```

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `split_by_language` | bool | `false` | Divide the HTML report into one section per language. Also enabled by the `--split-by-language` flag — either source turns it on. |

## `[ranking]`

Controls how two demotable clone classes are scored. **Data clones** are near-verbatim data blobs (long literal tables, fixtures). **Structural-only** clusters match on shape alone, with no token or semantic support. Both default to `demote`, so they sink below real, full-evidence clones without vanishing.

```toml
[ranking]
data_clones = "demote"
data_clone_weight = 0.15
structural_only = "demote"
structural_only_weight = 0.15
```

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `data_clones` | `"demote"` \| `"ignore"` \| `"keep"` | `"demote"` | Policy for data-category clusters. `demote` down-weights, `ignore` drops them from the report, `keep` ranks at full weight. |
| `data_clone_weight` | float | `0.15` | Multiplier applied when `data_clones = "demote"`. Must be in `(0.0, 1.0]`. |
| `structural_only` | `"demote"` \| `"ignore"` \| `"keep"` | `"demote"` | Same three-way policy for shape-only clusters. |
| `structural_only_weight` | float | `0.15` | Multiplier applied when `structural_only = "demote"`. Must be in `(0.0, 1.0]`. |

A weight of `0.0` is rejected (a demoted cluster must never be silently erased), and so is anything above `1.0` (that would *promote* the demoted class). The VS Code extension can override `structural_only` from its settings; the editor setting wins over the file.

## Built-in rules (always on)

These run regardless of your config and **cannot be disabled** — they keep dependency trees and machine-generated code out of every report.

**Always excluded** (any path containing one of these directory components):

```
node_modules   target   dist   build   .venv   __pycache__
.cargo   .git   .claude   .dart_tool   .pub-cache
```

**Always hidden from the report** (analysed, but kept out of the headline):

- The directory component `generated/`.
- The adjacent component pairs `alembic/versions`, `test/fixtures`, `tests/fixtures` — unless that pair *is* your scan root, so you can still analyse a fixture corpus on purpose.
- File suffixes: `.g.cs`, `.generated.cs`, `.designer.cs`, `.pb.cs`, `.openapi.cs`, `.generated.py`, `_generated.py`, `.g.dart`, `.freezed.dart`, `.gr.dart`, `.config.dart`, `.gen.dart`, `.mocks.dart`, `.pb.dart`, `.pbenum.dart`, `.pbjson.dart`, `.pbserver.dart`, `.pbgrpc.dart`.
- Any file whose first kilobyte carries a generated-code banner: `@generated`, `GENERATED CODE`, `auto-generated`, `autogenerated`, or `automatically generated` (matched case-insensitively).

## A complete example

```toml
# Skip vendored and build output everywhere.
[defaults]
exclude = [
  "**/node_modules/**",
  "**/dist/**",
  "third_party/**",
]
report_hide = [
  "**/*.snapshot.cs",
]

# Surface a hygiene hint instead of silently dropping import-only clones.
[defaults.boilerplate]
imports = "report"

# C# database migrations are generated — analyse but hide them.
[language.csharp]
report_hide = ["**/Migrations/**"]

# Fail CI when repo-wide duplication climbs above 20%.
[threshold]
max_duplication_percent = 20

# Split the HTML report per language.
[report]
split_by_language = true

# Drop pure shape-only matches entirely instead of demoting them.
[ranking]
structural_only = "ignore"
```

## CLI flags

Every run also takes flags. A flag overrides the matching `.deslop.toml` key for that run.

| Flag | Default | Meaning |
| --- | --- | --- |
| `PATH` | `.` | Directory to analyse (positional). |
| `--min-nodes <N>` | `30` | Minimum AST subtree node count for a clone candidate. Higher = fewer, larger clones. |
| `--output <PREFIX>` | `deslop-report` | Base path for reports; `.json` / `.txt` / `.html` are appended. |
| `--config <FILE>` | `<root>/.deslop.toml` | Explicit config file. |
| `--split-by-language` | off | One HTML section per language (same as `[report] split_by_language`). |
| `--nojson` / `--notext` / `--nohtml` | all on | Suppress a single output format. At least one must remain. |
| `--fail-over <PERCENT>` | — | Exit `3` when duplication exceeds `PERCENT`. Overrides `[threshold]`. |
| `--no-fail-over` | — | Clear any threshold for this run; never exit `3`. |
| `--technical` | off | Show the researcher view (taxonomy IDs, signal letters, node counts) on stderr. |

### Embeddings

The hybrid embedding layer is **off by default**; Deslop ships structural and token signals without it. Turn it on when you have a provider running.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--embeddings <MODE>` | `off` | `off` skips embeddings; `auto` probes the provider and falls back with a warning; `required` hard-fails when the provider is unreachable. |
| `--embedding-provider <ID>` | `ollama` | Provider registry key. Only `ollama` is implemented today. |
| `--embedding-model <MODEL>` | `nomic-embed-text` | Model id as the provider understands it (for Ollama, the name from `ollama list`). |
| `--embedding-endpoint <URL>` | `http://127.0.0.1:11434` | Provider endpoint. |

### Logging and colour

| Flag | Default | Meaning |
| --- | --- | --- |
| `--log-to-console` | off | Send logs to stderr instead of a timestamped `deslop-<timestamp>.log` file. |
| `--log-level <LEVEL>` | `info` | `error` \| `warn` \| `info` \| `debug` \| `trace`. Overridden by `RUST_LOG`. |
| `--no-color` | off | Disable colour in the stderr preamble and summary. |
| `--no-incremental` | off | Turn **off** the on-disk fingerprint cache under `<root>/.deslop/cache/`. The cache is on by default, so unchanged files skip parsing on the next run; pass this to leave the scanned tree untouched. |

### Developer and simulation flags

These bypass or replay the pipeline and are not part of normal use: `--from-report <FILE>` re-renders an existing JSON report into `.txt`/`.html`; `--debug-ast <FILE>` prints one file's normalised AST and exits; `--rerun-touch`, `--rerun-remove`, and `--rerun-add` replay file changes through an incremental session to exercise the live update path.

## Precedence and environment

When the same setting comes from more than one place, this is who wins:

- **Threshold:** `--no-fail-over` → `--fail-over <n>` → `[threshold] max_duplication_percent` → off.
- **Split by language:** the `--split-by-language` flag **or** `[report] split_by_language` — either one enables it.
- **Ranking (`structural_only`):** the VS Code extension setting overrides `.deslop.toml`.

Two environment variables also apply:

- **`RUST_LOG`** — overrides `--log-level` using the standard `tracing` filter syntax.
- **`NO_COLOR`** — disables colour even without `--no-color`. Colour is also suppressed automatically when stderr is not a terminal.
