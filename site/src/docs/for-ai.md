---
layout: layouts/docs.njk
title: For AI — how coding agents use Deslop to stop duplicate code
description: "Instructions for coding agents: call find-similar before writing code, use the Deslop CLI when MCP is unavailable, and interpret thresholds and byte ranges."
eleventyNavigation:
  key: For AI
  order: 4
icon: terminal
docsGroup: reference
---

# For AI

Use the same engine findings as the editor. This reference covers [0.35.0](/releases/). For connection setup, see [AI Agents](/docs/ai-integration/).

## Check before you write

Call `find-similar` with a proposed snippet, or a `path`, `start_byte` and `end_byte`. It can find an existing implementation even when that implementation appears only once in the repository. For existing duplication, use `duplicates`, then `cluster-by-id` to inspect the finding.

Read the returned `kind` and source ranges:

| `kind` | Meaning and action |
| --- | --- |
| `identical` | Source text matches apart from permitted whitespace differences. Reuse the existing implementation where appropriate. |
| `nearly_identical` | Renamed, parameterized or lightly edited copies. Inspect differences before extracting shared code. |
| `loosely_similar` | Substantial copied work with larger edits. Read both ranges. |
| `same_behavior` | Optional embedding-based match. Similarity does not prove interchangeability. |
| `structural_only` | Informational shape match, not a clone. It contributes nothing to duplication figures. |

A cluster's `mass` ranks impact, not confidence or extraction safety. A cluster carries no pair similarity measurements. Request an explicit comparison with both endpoints when you need evidence about a particular pair.

## If the MCP server is unavailable, use the CLI

Check that the editor's LSP server is running and that MCP uses the same workspace root. A not-ready response or a connection error is not evidence that no similar code exists. A build mismatch must be resolved by using matching bundled binaries.

For a separate scan:

```bash
deslop . --notext --nohtml --no-color
```

Read `.deslop/deslop-report.json`. The CLI cannot query an unwritten snippet: inspect the baseline report before a change, scan again afterwards, and check the changed paths in `clusters[].occurrences[]`. Reuse or consolidate confirmed copies, then rescan. Unchanged files can reuse cached parse work, but analysis still considers repository-wide relationships.

The CLI can also [compare two existing occurrences](/docs/configuration/#compare-two-occurrences). If neither surface is available, report that limitation rather than guessing.

## Read the JSON

Use the report's embedded `schema_doc`; over MCP, request `schema-doc` once per session. Parse JSON rather than text or HTML.

| Field | Meaning |
| --- | --- |
| `metrics.duplication_percent` | Covered clone lines divided by analysed lines. Not detector accuracy. |
| `metrics.threshold.breached` | Whether the configured duplication ceiling was exceeded. The CLI writes reports before exiting `3`. |
| `clusters` | Clones in descending mass order, then informational findings. |
| `cluster.kind` | The classification above; current reports use `kind`, not the older `bucket` field. |
| `cluster.mass` / `cluster.rank` | Clone impact and position. Informational findings have zero mass and no clone rank. |
| `cluster.severity` | Diagnostic level, independent of mass and duplication percentage. |
| `occurrences[].hidden` | An occurrence hidden by report policy; it does not contribute duplicated lines. |

### Byte ranges, not line numbers

Ranges are `[start_byte, end_byte)`, with the end excluded. They refer to the scanned source version. Refresh the analysis after edits before reusing offsets.

### Cluster IDs are stable

Use the cluster ID to identify a finding across surfaces. Rank changes with the report order; IDs can change when the underlying clone content changes.

## Configuring a repository

[`exclude`](/docs/configuration/) removes a file before analysis. `report_hide` retains analysis but hides its occurrences. Built-in rules already cover dependency directories, build artifacts and common generated files. Thresholds opt into failing CI; they do not change what constitutes a clone.

## Operating rules

Read the actual occurrences before editing. Do not treat a label as proof that a refactor preserves behaviour, or weaken thresholds to conceal a finding. If duplication is deliberate, record why. See [Accuracy Transparency](/docs/accuracy-transparency/) for measurement limits and the [agent recipe](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md) for a reusable project rule.
