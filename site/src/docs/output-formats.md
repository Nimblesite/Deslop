---
layout: layouts/docs.njk
title: Output Formats — JSON, TXT, HTML
description: Deslop emits three reports per run — canonical JSON for agents, line-oriented TXT for terminals, and standalone HTML for humans. Same schema, same ranking.
eleventyNavigation:
  key: Output Formats
  order: 5
icon: description
---

# Output Formats

Every Deslop run emits three reports. The JSON is the product; the text and HTML are renderers over the same data. No claim appears in TXT or HTML that is not also present in the JSON.

## JSON — canonical

`deslop-report.json` is what agents read and what schema consumers should parse.

```json
{
  "tool_version": "0.0.0-dev",
  "schema_doc": "…",
  "metrics": {
    "analysed_loc": 1832044,
    "duplicated_loc": 48120,
    "duplication_percent": 2.63,
    "clusters_total": 142,
    "duplicated_files": 318,
    "threshold": { "…": "--fail-over verdict" }
  },
  "clusters": [ { "…": "see AI Integration" } ]
}
```

### Guarantees

- Fields marked `optional` in the schema may be absent. Fields marked `required` are always present.
- Clusters are sorted by `weight` descending. `clusters[0]` is always the worst offender.
- UTF-8. No BOM. LF line endings.

## TXT — terminal

`deslop-report.txt` is ASCII, line-oriented, and deliberately boring. No ANSI colors, no unicode box-drawing, no paging escape codes. Pipeable into `head`, `grep`, `awk` without surprises.

```
deslop 0.0.0-dev -- 840 file(s), 142 cluster(s), 0 hidden
repo: 2.6% duplicated (48120 / 1832044 LOC, 142 clusters across 318 files)
embeddings: off
-- action hints --
  [identical] Extract the shared code into one definition and call it from every duplicate site.
#1 [0362505641efe3c7] weight=1252.80 size=3 nodes=58
  3 near-identical copies — safe to extract.
  :: Nearly identical across UserRepository.cs, ProductRepository.cs, OrderRepository.cs.
```

Each cluster is a numbered block — `#1` is the worst offender — with its weight, size, and node count, followed by a plain-English summary and a one-line interpretation. Clusters are listed worst-first. This format survives every terminal, every SSH session, and every CI log.

## HTML — portable

`deslop-report.html` is a single file. All CSS is inlined. No network requests. Drop it into a CI artifact, email it, open it on an airplane — it renders.

The HTML renderer uses the same ranking and the same cluster summaries as JSON and TXT. It adds:

- syntax-highlighted example snippets, with long snippets and extra locations folded into collapsible toggles
- an "AI match" badge and an impact chip on each duplicate group
- a per-group signals table (structural / token / embedding / fused) in a collapsible "Run details" footer

It does not add: scores not in the JSON, commentary beyond the `summary` field, or links to external services.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Completed. Duplication was below the `--fail-over` threshold, or no threshold was set. |
| `1` | Runtime error — nonexistent scan path, analysis failure, I/O error, or a `required` embedding provider that was unreachable. Never a panic. |
| `2` | Usage error — unknown flag or an invalid argument value, rejected before the run starts. |
| `3` | Duplication percentage exceeded the `--fail-over` gate. |

`deslop` never panics on user input. Failures surface through these exit codes and a structured error on `stderr`.

Exit `3` is the **CI gate**. It only ever fires when you opt in — by passing a `--fail-over` percentage or setting `[threshold] max_duplication_percent` in `.deslop.toml`. See [Gate CI on a duplication threshold](/docs/#gate-ci-on-a-duplication-threshold) for the walkthrough, or [For AI](/docs/for-ai/#run-in-ci) for a ready-to-paste GitHub Actions job.

## Logging vs. reports

By default, diagnostics go to a timestamped log file (`deslop-<timestamp>.log`) via `tracing` with structured fields; pass `--log-to-console` to send them to `stderr` instead. The human-readable preamble and summary always go to `stderr`. The reports themselves are written to files — `deslop-report.json`/`.txt`/`.html` by default, or the prefix you pass to `--output`. To emit only the JSON report, suppress the other two formats:

```bash
deslop . --notext --nohtml      # writes deslop-report.json
```
