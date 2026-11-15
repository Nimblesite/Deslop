---
layout: layouts/docs.njk
title: Reports — JSON, text and HTML output
description: What Deslop writes after a scan, where the files land, and how the JSON, text and HTML reports relate to each other.
eleventyNavigation:
  key: Reports
  order: 6.5
icon: code
docsGroup: reference
---

# Reports

A scan writes three reports to `.deslop/` at the scan root, or to the prefix passed to `--output`. All three render the same data: nothing appears in the text or HTML that is not in the JSON. Turn a format off with `--nojson`, `--notext` or `--nohtml`; settings that change what the reports contain are on [Configuration](/docs/configuration/).

```bash
deslop .                    # .deslop/deslop-report.{json,txt,html}
deslop . --notext --nohtml  # JSON only
```

## JSON

`deslop-report.json` is the canonical report, the one agents and tools parse.

- Clones are sorted by `mass` descending, with stable IDs breaking ties. Shape-only findings follow, unranked and not counted.
- Fields marked `required` in the schema are always present; `optional` fields may be absent.
- UTF-8, LF line endings, no BOM.

Field-by-field shape: [For AI → Read the JSON](/docs/for-ai/#read-the-json).

## Text

`deslop-report.txt` is for terminals and CI logs. Each clone lists its kind, mass, occurrence count and source locations, in rank order. Pair similarity is not in this report; ask for it with [`--compare`](/docs/configuration/#compare-two-occurrences).

## HTML

`deslop-report.html` is one self-contained file: inline CSS, no network requests, so it opens anywhere, including as a CI artifact. On top of the shared data it shows:

- syntax-highlighted snippets, with long snippets and extra locations folded away
- one collapsible group per clone kind, carrying the group's mass
- a "Run details" footer with tool version, group counts and embedding provenance

`--split-by-language` (or `[report] split_by_language`) gives each language its own section.

## Logs

Logs are not reports. They go to a timestamped file under `.deslop/logs/`, or to stderr with `--log-to-console`. The short human summary always goes to stderr.
