---
layout: layouts/docs.njk
title: Output Formats
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
  "report_schema_version": "1.0",
  "generator": { "name": "deslop", "version": "1.0.0" },
  "schema_doc": "…",
  "summary": {
    "clusters_total": 142,
    "above_threshold": 17,
    "files_scanned": 4812,
    "loc_scanned": 1832044,
    "scan_time_ms": 27110
  },
  "clusters": [ { "…": "see AI Integration" } ]
}
```

### Guarantees

- `report_schema_version` is semver. Major bumps are breaking.
- Fields marked `optional` in the schema may be absent. Fields marked `required` are always present.
- Clusters are sorted by `score` descending. `clusters[0]` is always the worst offender.
- UTF-8. No BOM. LF line endings.

## TXT — terminal

`deslop-report.txt` is ASCII, line-oriented, and deliberately boring. No ANSI colors, no unicode box-drawing, no paging escape codes. Pipeable into `head`, `grep`, `awk` without surprises.

```
Deslop 1.0.0  —  142 clusters, 17 above threshold
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
| `0` | Ran successfully. |
| `1` | Ran successfully, but at least one cluster was above `--fail-on` threshold. |
| `2` | Usage error — bad flag, missing path. |
| `3` | Pipeline error — parser crash, I/O failure. Never a panic. |

`deslop` never panics on user input. Errors are surfaced through exit codes and a structured error object in the JSON.

## Logging vs. reports

Diagnostics go to `stderr` via `tracing` with structured fields. Reports go to `stdout` (or `--output`) via the renderer. They are different streams. Piping only the report:

```bash
deslop . --format=json > report.json
```
