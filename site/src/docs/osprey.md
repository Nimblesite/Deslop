---
layout: layouts/docs.njk
title: Osprey — Syntax Flavors and Duplicate Detection
description: Scan all three Osprey syntax flavors with Deslop, including native ML layout and explicit Python-flavor selection.
eleventyNavigation:
  key: Osprey
  order: 12
icon: code
docsGroup: guides
---

# Osprey support

Deslop analyses Osprey with tree-sitter and sends findings through the same Rust engine used by the CLI, live editor and MCP tools. Osprey receives a gold crowned bird in the language groups and report views. Start with [Osprey Gets the Crown](/blog/osprey-first-class/) for an introduction.

## Syntax support

This page describes the integration in the current development tree. It does not promise that an older installed VSIX contains these changes.

| Source flavor | Selection | Integration status |
| --- | --- | --- |
| Default braces | `.osp` | Native grammar, CLI fixtures and real LSP/MCP language-label checks verified. |
| Python | Explicitly opted-in `.py` | CLI routing, ordinary-Python isolation and real LSP/MCP language-label checks verified. |
| ML layout | `.ospml` | Native grammar registered after source-corpus and structure checks; CLI and real LSP/MCP language-label checks verified. |

The default grammar preserves effects, handlers, patterns and expressions, including expressions inside string interpolation. Names and literals normalize for structural comparison; shared evidence checks decide whether a candidate contains copied implementation. Comments and imports do not count as copied implementation.

## Scan a project

Open the project with Deslop Live installed, or use the CLI:

```bash
deslop ./my-osprey-project --output ./target/deslop-report
```

Review the ranked findings and compare the occurrences before choosing an extraction. Detection support does not imply automatic Osprey refactoring or prove that two implementations are interchangeable. See [How It Works](/docs/how-it-works/) for categories and ranking, and [Configuration](/docs/configuration/) for exclusions and thresholds.

## Select Osprey's Python flavor

Ordinary `.py` files retain Python identity. The development integration recognises an Osprey flavor marker only in a leading source comment:

```python
# osprey: flavor=python
```

A string containing that text or a comment after code cannot select the flavor. Alternatively, list individual source files in the existing `osprey.toml`:

```toml
[project]
entry = "src/main.py"
python_sources = ["src/domain.py", "src/storage.py"]
```

The Python entry file and the listed files opt into Osprey. Paths are literal files relative to the manifest, not directory-wide or glob selections. Other `.py` files retain ordinary Python identity.

The project `flavor` setting accepts `default`, `ml` or `python` and overrides the syntax for sources claimed by that project. Without an override, a marker that contradicts the extension produces an error. Malformed manifests and unknown flavor values also produce errors. Source roots default to `src` when present, otherwise the project directory; `source_roots` overrides this default. Nested projects take precedence.

## Language identity in reports

The engine distinguishes `osprey`, `osprey-ml` and `osprey-python` in parser and cache identities. Occurrences and file metrics carry the selected language so clients can distinguish Osprey Python from ordinary Python even though both use `.py`. The interface groups the three identities as Osprey and uses the same crowned icon.

Sharing a language family does not establish cross-flavor clone equivalence. Automatic Python refactoring actions are not inherited by Osprey's Python flavor. Invalid ML syntax produces a visible parse error rather than a report based on a partial syntax tree.
