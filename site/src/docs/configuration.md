---
layout: layouts/docs.njk
docFilm: configuration
title: Configuration — .deslop.toml, dials, flags, exit codes
description: Every Deslop dial, .deslop.toml section and CLI flag in one reference — similarity thresholds, CI gates, exclusions and exit codes.
eleventyNavigation:
  key: Configuration
  order: 6
icon: tune
docsGroup: reference
---

# Configuration

Deslop reads `.deslop.toml` from the scan root, or the file named by `--config`. Without one it runs on the defaults below. A CLI flag beats the matching key. An unknown section or key fails the run and names the offender, so a typo can never switch a gate off silently.

What the reports contain is described in [Reports](/docs/reports/).

{% include "partials/doc-film.njk" %}

## Dials

These are all the numbers you can change. Each one alters what Deslop finds, how it labels a finding, or when it fails CI.

<div class="key-table">

| Dial | Default | Effect |
| --- | --- | --- |
| `--min-nodes` | `30` | Smallest syntax subtree compared, in AST nodes. Raise it for fewer, larger clones; lower it to catch small ones. |
| `nearly_identical_min_shape` | `0.90` | Structure similarity a pair needs to be labelled **Nearly identical**. |
| `nearly_identical_min_content` | `0.70` | Content support a pair needs to be labelled **Nearly identical**. |
| `similar_min_content` | `0.50` | Content support a pair needs to be labelled **Loosely similar**. |
| `shape_only_max_content` | `0.05` | Content support at or below this counts as none. The pair is **Shape-only** and never counts as duplication. |
| `max_duplication_percent` | off | Exit `3` when repo-wide duplication exceeds this percentage. `--fail-over` sets it per run. |
| `[threshold.kinds]` | off | One percentage limit per clone kind, checked against that kind's own figure. |
| `max_average_line_bytes` | `200` | Mean bytes per line above which a file counts as minified build output and is skipped. |
| `--jobs` | all cores | Worker threads per parallel stage. Changes speed only; the report is identical for every value. |

</div>

### Similarity dials

The four `[tuning.routing]` dials decide which **kind** a pair of code ranges gets. They read two measurements Deslop takes on every pair:

- **Shape**: how alike the two syntax trees are once names and literal values are set aside. `1.0` means the same structure.
- **Content support**: how much of what fills that structure matches, such as names, literals, operators and calls, or is renamed consistently (every `a` became `b`). `1.0` means the same content.

Deslop first decides whether a pair is a clone at all ([how admission works](/docs/how-it-works/#find-candidate-copies)). The dials then label it, checking in this order:

1. **Identical**: the source text is the same. No dial applies.
2. **Nearly identical**: a consistent rename, or shape ≥ `nearly_identical_min_shape` **and** content support ≥ `nearly_identical_min_content`.
3. **Same behavior**: embeddings say the two do the same thing (only with `--embeddings`).
4. **Loosely similar**: a copied core of identical lines, or content support ≥ `similar_min_content`.

A pair that is **not** a clone, but has shape ≥ `nearly_identical_min_shape` and content support ≤ `shape_only_max_content`, is listed as **Shape-only**: the same structure filled with different code. It never counts toward duplication.

<div class="key-table">

| To… | Turn… | Side effect |
| --- | --- | --- |
| Call fewer pairs Nearly identical | `nearly_identical_min_shape` or `nearly_identical_min_content` up | Those pairs become Loosely similar. By default only Identical and Nearly identical count toward the headline percentage, so it drops. |
| Keep weakly matching pairs out of Loosely similar | `similar_min_content` up | Admitted pairs below it and with no copied core get no kind. |
| Treat more same-shaped pairs as Shape-only | `shape_only_max_content` up | Must stay below `similar_min_content`. |
| Find smaller clones | `--min-nodes` down | More, smaller findings, and slower scans. |

</div>

Each dial takes a value from `0` to `1`, and together they must satisfy `shape_only_max_content < similar_min_content <= nearly_identical_min_content`; a config that breaks this is refused. No dial turns a rejected pair into a clone. The JSON report records the values in effect under `routing`. The defaults are provisional, and a corpus sweep that scores alternatives is recorded in [corpus/tuning-sweep.md](https://github.com/Nimblesite/Deslop/blob/main/corpus/tuning-sweep.md).

```toml
[tuning.routing]
nearly_identical_min_shape = 0.90
nearly_identical_min_content = 0.70
similar_min_content = 0.50
shape_only_max_content = 0.05
```

## A complete `.deslop.toml`

Patterns are globs with gitignore semantics, resolved against the scan root.

```toml
[defaults]                       # every language
exclude = ["third_party/**"]     # never analysed
report_hide = ["**/*.snapshot.cs"]  # analysed, hidden from the report

[defaults.boilerplate]
imports = "report"               # "suppress" (default) | "report"

[language.csharp]                # extends [defaults] for one language
report_hide = ["**/Migrations/**"]

[threshold]
max_duplication_percent = 20     # exit 3 above 20%

[threshold.kinds]
identical = 3                    # exit 3 when identical copies alone pass 3%

[total]
include = ["identical", "nearly_identical"]  # kinds counted in the headline figure

[analysis]
allow_cross_language_comparison = false
include_dependencies = false
incremental = true
max_average_line_bytes = 200

[report]
split_by_language = false

[visibility]
data_clones = "keep"             # "keep" | "ignore"
```

## `exclude` vs `report_hide`

<div class="key-table">

| Key | What happens | Use it for |
| --- | --- | --- |
| `exclude` | The file is dropped before analysis. | Vendored code and anything you never want compared. |
| `report_hide` | The file is analysed, but its occurrences are hidden. A cluster leaves the report only when **all** its members are hidden. | Generated code: you still learn when hand-written code duplicates it. |

</div>

Both keys go under `[defaults]` or `[language.<name>]`, along with `boilerplate.imports`. `suppress` drops import-only clones; `report` turns them into a low-severity hint. Language overlays **extend** the defaults. Language ids: `csharp`, `rust`, `python`, `dart`, `javascript`, `typescript`, `tsx`, `php`, `fsharp`, `go`, `c`.

### Built-in rules

These apply before your patterns.

- **Always excluded:** `target`, `dist`, `build`, `out`, `coverage`, `.vscode-test`, `__pycache__`, `.dart_tool`, `.git`, `.claude`. Minified and bundled files are excluded too, by filename suffix and by `max_average_line_bytes`.
- **Excluded unless `include_dependencies = true`:** `node_modules`, `vendor`, `.cargo`, `.pub-cache`, `.venv`.
- **Always hidden from the report:**
  - any `generated/` directory
  - `alembic/versions`, `test/fixtures` and `tests/fixtures`, unless that folder is the scan root
  - generated suffixes: `.g.cs`, `.generated.cs`, `.designer.cs`, `.pb.cs`, `.openapi.cs`, `.generated.py`, `_generated.py`, `.g.dart`, `.freezed.dart`, `.gr.dart`, `.config.dart`, `.gen.dart`, `.mocks.dart`, `.pb.dart`, `.pbenum.dart`, `.pbjson.dart`, `.pbserver.dart`, `.pbgrpc.dart`
  - any file whose first comment line is a generator banner, such as `// GENERATED CODE - DO NOT MODIFY BY HAND` or `// @generated`. Banners are read from parsed comments, so the phrase inside a string or a later comment never counts. See the [full banner list](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/exclusion.md).

## CI gate

Deslop exits `0` unless you opt into a gate. Any breached gate exits `3`, and the full report is still written.

<div class="key-table">

| Key | Default | Effect |
| --- | --- | --- |
| `[threshold] max_duplication_percent` | off | Limit on the repo-wide duplication percentage, `0`–`100`. |
| `[threshold.kinds]` | off | Optional limit per kind: `identical`, `nearly_identical`, `loosely_similar`, `same_behavior`. Each is checked independently of the total. |
| `[total] include` | `["identical", "nearly_identical"]` | Which kinds count toward the headline figure. Excluded kinds keep their clusters and per-kind figures. `structural_only` is refused because shape-only is never duplication. |

</div>

`--fail-over <PERCENT>` overrides `max_duplication_percent` for one run; `--no-fail-over` clears every gate. With `--diff` and `--only-changed`, the gate reads the diff-scoped percentage, so old debt cannot fail a pre-merge check. Walkthrough: [Gate CI on a duplication threshold](/docs/getting-started/#gate-ci-on-a-duplication-threshold) and the [GitHub Action](/docs/github-action/).

## Other settings

<div class="key-table">

| Key | Default | Effect |
| --- | --- | --- |
| `[analysis] allow_cross_language_comparison` | `false` | Lets a clone pair span two languages. |
| `[analysis] include_dependencies` | `false` | Analyses dependency trees. Off, because their duplication would outrank code you can edit. |
| `[analysis] incremental` | `true` | Reuses the on-disk parse cache on every surface. `false` forces a full re-parse. |
| `[report] split_by_language` | `false` | One HTML section per language. `--split-by-language` also enables it. |
| `[visibility] data_clones` | `"keep"` | `"ignore"` drops proven data-table findings before ranking. |

</div>

Clones rank by duplicated mass alone: AST nodes × additional copies. No setting reweights a kind.

## CLI flags

<div class="key-table">

| Flag | Default | Effect |
| --- | --- | --- |
| `PATH` | `.` | Directory to analyse. |
| `--output <PREFIX>` | `.deslop/deslop-report` | Report path prefix; `.json`, `.txt` and `.html` are appended. Logs go to `<dir>/logs/`. |
| `--config <FILE>` | `<root>/.deslop.toml` | Config file to use instead. |
| `--nojson` | off | Skip the JSON report. |
| `--notext` | off | Skip the text report. |
| `--nohtml` | off | Skip the HTML report. At least one format must remain. |
| `--diff <FILE>` | — | Tag clones on a unified diff's added lines; `-` reads stdin. A diff that does not match the tree is refused. |
| `--only-changed` | off | Drop clones outside the diff. Requires `--diff`. |
| `--technical` | off | Researcher view on stderr: taxonomy IDs, signal letters, node counts. |
| `--no-incremental` | off | Skip the on-disk caches for this run. |
| `--embeddings <MODE>` | `off` | `auto` probes the provider and falls back with a warning; `required` fails when it is unreachable. |
| `--embedding-provider <ID>` | `ollama` | Provider. Only `ollama` exists today. |
| `--embedding-model <MODEL>` | `nomic-embed-text` | Model name, as `ollama list` shows it. |
| `--embedding-endpoint <URL>` | `http://127.0.0.1:11434` | Provider endpoint. |
| `--log-to-console` | off | Logs to stderr instead of `logs/deslop-<timestamp>.log`. |
| `--log-level <LEVEL>` | `info` | `error`, `warn`, `info`, `debug` or `trace`. `RUST_LOG` overrides it. |
| `--no-color` | off | Plain stderr. `NO_COLOR`, or a stderr that is not a terminal, does the same. |

</div>

The dials above (`--min-nodes`, `--jobs`, `--fail-over`, `--no-fail-over`) are flags too. Developer flags: `--from-report <FILE>` re-renders a saved JSON report, `--debug-ast <FILE>` prints one file's normalised AST, and `--rerun-touch`, `--rerun-remove` and `--rerun-add` replay file changes through the live update path.

### Compare two occurrences

Pass two endpoints as `<path>:<start_byte>:<end_byte>`, copied from the JSON report:

```text
deslop . --compare "<left-path>:<start-byte>:<end-byte>" --compare "<right-path>:<start-byte>:<end-byte>"
```

Deslop prints a JSON verdict for that pair to stdout. Offsets are bytes and the end is exclusive. Each range must match a fingerprinted occurrence; a cluster ID is not accepted.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Completed, with no gate breached. |
| `1` | Runtime error: missing scan path, invalid `.deslop.toml`, analysis or I/O failure, or a `required` embedding provider that was unreachable. |
| `2` | Usage error: unknown flag or an out-of-range `--fail-over` value. |
| `3` | A [CI gate](#ci-gate) was breached. The report is still written. |
