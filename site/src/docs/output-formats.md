---
layout: layouts/docs.njk
title: Output Formats — JSON, TXT, HTML
description: Deslop emits three reports per run — canonical JSON for agents, line-oriented TXT for terminals, and standalone HTML for humans. Same schema, same ranking.
eleventyNavigation:
  key: Output Formats
  order: 4
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
Deslop 0.0.0-dev  —  142 clusters, 17 above threshold
──────────────────────────────────────────────────────────────────────

  SCORE  KIND                FILE                            SPAN
──────── ─────────────────── ─────────────────────────────── ──────────
▲  2184  Nearly identical    UserRepository.cs               120–180
▲  2184  Nearly identical      ProductRepository.cs          58–118
▲  2184  Nearly identical      OrderRepository.cs            40–102

  Signals: structural=1.00  token_jaccard=0.97  embedding_cos=0.91
  Summary: 3 near-identical copies — safe to extract.
──────────────────────────────────────────────────────────────────────
```

The leading `▲` marks the representative (first) member of a cluster; indented rows are additional members. This format survives every terminal, every SSH session, and every CI log.

## HTML — portable

`deslop-report.html` is a single file. All CSS is inlined. No network requests. Drop it into a CI artifact, email it, open it on an airplane — it renders.

The HTML renderer uses the same ranking and the same cluster summaries as JSON and TXT. It adds:

- collapsible cluster cards
- side-by-side diff panels for each pair of members
- a signals strip per cluster

It does not add: scores not in the JSON, commentary beyond the `summary` field, or links to external services.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Completed. Duplication was below the `--fail-over` threshold, or no threshold was set. |
| `1` | Runtime error — nonexistent scan path, analysis failure, I/O error, or a `required` embedding provider that was unreachable. Never a panic. |
| `2` | Usage error — unknown flag or an invalid argument value, rejected before the run starts. |
| `3` | Duplication percentage exceeded the `--fail-over` gate. |

`deslop` never panics on user input. Failures surface through these exit codes and a structured error on `stderr`.

## Logging vs. reports

Diagnostics go to `stderr` via `tracing` with structured fields; the human-readable preamble and summary are written to `stderr` too. The reports themselves are written to files — `deslop-report.json`/`.txt`/`.html` by default, or the prefix you pass to `--output`. To emit only the JSON report, suppress the other two formats:

```bash
deslop . --notext --nohtml      # writes deslop-report.json
```
