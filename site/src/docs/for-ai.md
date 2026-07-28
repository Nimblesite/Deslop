---
layout: layouts/docs.njk
title: For AI — configure, gate CI, and read Deslop reports (agent guide)
description: The operating manual for AI coding agents using Deslop — call find-similar before writing code, configure .deslop.toml, gate CI on a duplication threshold with exit code 3, and parse the canonical JSON report.
eleventyNavigation:
  key: For AI
  order: 4
icon: terminal
---

# For AI

This page is the operating manual for **AI coding agents** — Claude Code, Cursor, GitHub Copilot, Continue, Codex — and the humans configuring them. It is deliberately directive: how to configure Deslop, what to exclude, how to gate CI, and how to parse the report. For wiring `deslop-mcp` into your client, read [AI Integration](/docs/ai-integration/) first.

## The one law: call `find-similar` before you write code

Deslop earns its keep through **prevention**, not cleanup. Before you author any new function, method, class, helper, fixture, or test setup, call the `find-similar` MCP tool with the proposed snippet (or its byte range) and read the response:

- `signals.fused ≥ 0.85`, or bucket `identical` / `nearly_identical` → **do not write the copy.** Reuse the canonical occurrence the tool returns; extract a shared helper if needed.
- `signals.fused < 0.6`, or an empty response → proceed with authoring.
- `0.6 ≤ fused < 0.85` → read the canonical occurrence and bias toward reuse.

`find-similar` is for **authoring**. When you are *cleaning up* existing duplicates, start with `top-offenders`, then `cluster-by-id` for the cluster you will merge. The paste-ready rule block for your project's `AGENTS.md` / `CLAUDE.md` is in the [agent recipe](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md).

| Tool | When to call it |
| --- | --- |
| `find-similar` | **Before** writing new code — does an equivalent already exist? |
| `top-offenders` | Worst clusters in the workspace, worst first. Start cleanup here. |
| `cluster-by-id` | Full member list + signals for one cluster you are about to merge. |
| `report-for-file` / `report-for-range` | Clusters touching a specific file or selection. |
| `schema-doc` | Authoritative JSON schema. Call **once** per session, not per response. |

## Configure with `.deslop.toml`

Deslop reads `.deslop.toml` from the scan root (or the path passed to `--config`). No file is required — Deslop ships conservative built-in defaults. Every section and key is optional; omit what you do not need.

```toml
# Shared rules, applied to every language.
[defaults]
exclude     = ["vendor/**", "third_party/**"]    # dropped before parsing — never analysed
report_hide = ["**/*.generated.cs", "**/*.g.cs"] # analysed, but hidden from the ranked report

# Per-language overlays, keyed by the parser language id:
# csharp, rust, python, dart, javascript, typescript, tsx, php, fsharp, go.
# Overlays EXTEND [defaults]; they never replace it.
[language.csharp]
report_hide = ["**/Migrations/**/*.cs"]

[language.rust]
exclude = ["**/target/**"]

# Opt-in CI gate. Exceeding this exits 3. See "Run in CI" below.
[threshold]
max_duplication_percent = 5.0

# Analysis behaviour.
[analysis]
allow_cross_language_comparison = false  # true → compare clones across languages (ports, generated clients)

# Report rendering.
[report]
split_by_language = false  # true → one HTML section per language
```

**Keys must live under a section.** A bare top-level `exclude = [...]` with no `[defaults]` header is silently ignored. Per-language sections are additive: a `.rs` file is matched against `defaults.exclude ∪ language.rust.exclude`.

## What to exclude

Two tiers, with different semantics. Choose by intent:

| Key | Effect | Use for |
| --- | --- | --- |
| `exclude` | File is dropped during discovery — never parsed, never counted in `analysed_loc`, never in any cluster. | Vendored / third-party code you do not own and do not want analysed at all. |
| `report_hide` | File **is** analysed and can anchor a cluster, but each occurrence is flagged `hidden: true`. A cluster whose members are *all* hidden drops out of the ranking; a cluster with one visible member stays, so you still see "hand-written code duplicates generated code." | Generated output you still want to detect hand-written copies of. |

**Built-in defaults already cover the common cases — do not re-add them.** Excluded by default: `node_modules`, `target`, `dist`, `build`, `.venv`, `__pycache__`, `.cargo`. Report-hidden by default: any path with a `generated` component, Alembic migrations under `alembic/versions`, and the suffixes `*.g.cs`, `*.generated.cs`, `*.designer.cs`, `*.pb.cs`, `*.openapi.cs`, `*.generated.py`, `_generated.py`, `*_pb2.py`, `*_pb2_grpc.py`. Add only project-specific patterns on top.

Globs are gitignore-style and matched against paths relative to the config file.

## Run in CI

`deslop` exits `0` regardless of how much duplication it finds — unless you opt into a gate. Then it exits `3` when the repo-wide `duplication_percent` exceeds your ceiling. Two ways; the flag wins over the config key:

```bash
deslop . --fail-over 5.0   # exit 3 when duplication_percent > 5.0
```

```toml
# .deslop.toml — shared by local runs, CI, and agents
[threshold]
max_duplication_percent = 5.0
```

`--fail-over 0` fails on any duplication. `--no-fail-over` clears the gate for a single local run. The value must be a finite number in `[0.0, 100.0]`; anything else exits `2`.

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Succeeded; within threshold, or no threshold set. |
| `1` | Runtime error — bad scan path, parse/I-O failure, or an unreachable `required` embedding provider. Never a panic. |
| `2` | Usage error — unknown flag, or an out-of-range / non-finite threshold. |
| `3` | **Threshold breached.** The full report is still written to disk so the offenders can be surfaced. |

### GitHub Actions

```yaml
name: deslop
on: [push, pull_request]
jobs:
  duplication-gate:
    runs-on: ubuntu-latest
    env:
      DESLOP_VERSION: "0.1.0"   # pin the tool version — see the Releases page
    steps:
      - uses: actions/checkout@v4
      - name: Install the Deslop CLI
        run: |
          curl -sSfL "https://github.com/Nimblesite/Deslop/releases/download/v${DESLOP_VERSION}/deslop-${DESLOP_VERSION}-linux-x64.tar.gz" | tar -xz
          echo "$PWD/deslop-${DESLOP_VERSION}-linux-x64" >> "$GITHUB_PATH"
      - name: Gate on duplication
        run: deslop . --fail-over 5.0   # or omit --fail-over to use [threshold] in .deslop.toml
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: deslop-report
          path: deslop-report.html
```

A non-zero exit fails the step. The `if: always()` upload keeps `deslop-report.html` even on a breach so a human can browse the offenders.

## Read the reports

`deslop-report.json` is canonical and the **only** file you should parse — `.txt` and `.html` are renderers over it. Call `schema-doc` once for the authoritative schema, and see [Output Formats](/docs/output-formats/) for the full shape. The decision-relevant slice:

```json
{
  "metrics": {
    "duplication_percent": 2.63,
    "threshold": { "percent": 5.0, "breached": false, "source": "config" }
  },
  "clusters": [
    {
      "id": "0362505641efe3c7",
      "weight": 1252.8,
      "size": 3,
      "bucket": "identical",
      "signals": { "structural": 1.0, "token_jaccard": 0.98, "embedding_cos": 0.0, "fused": 0.99 },
      "occurrences": [ { "path": "src/UserRepository.cs", "start_line": 12, "end_line": 41, "hidden": false } ],
      "summary": "3 near-identical copies — safe to extract."
    }
  ]
}
```

| Field | How to act on it |
| --- | --- |
| `metrics.duplication_percent` | The repo-wide headline number the CI gate compares against. |
| `metrics.threshold.breached` | `true` → the run exited `3` and the gate failed. `source` is `cli`, `config`, or `none`. |
| `clusters` | Sorted by `weight` **descending** — `clusters[0]` is always the worst offender. Work top-down. |
| `bucket` | `identical` / `nearly_identical` → extract a shared definition. `structural_only` → only the code shape matches (no token or semantic evidence) — verify it is a real duplicate before extracting; demoted in ranking by default. `loosely_similar` → parametrise the difference. `same_behavior` → reconcile two implementations of one behaviour (needs `--embeddings`). |
| `signals.fused` | Unit-bounded confidence. `≥ 0.85` is the act-now line, the same threshold as the `find-similar` law above. |
| `occurrences[].hidden` | `true` marks a `report_hide` match — a hand-written clone of generated code. |

Do not silence findings by widening the threshold, marking code `hidden`, or splitting it into trivially different shapes. If Deslop flags it, treat it as a real signal until you have shown otherwise.
