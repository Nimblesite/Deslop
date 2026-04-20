---
layout: layouts/docs.njk
title: Getting Started
eleventyNavigation:
  key: Getting Started
  order: 1
icon: rocket_launch
---

# Getting Started

Deslop is a single Rust binary. Install it, run it against a directory, read the top of the report. If the top row is not the highest-impact duplication in your repo, file a bug.

## Install

```bash
cargo install deslop
```

Precompiled binaries and the VS Code extension land closer to v1. For now, `cargo install` is canonical.

## Run

```bash
deslop .
```

That scans the current directory, writes three reports, and prints the top clusters to your terminal:

```
deslop-report.json   # canonical, agent-consumable
deslop-report.txt    # line-oriented plain text
deslop-report.html   # standalone, human-readable
```

## Tune the threshold

Default minimum AST node count is chosen so trivial getters do not pollute the top of the report. Override per-run:

```bash
deslop . --min-nodes 20
```

Raise it for large codebases where you only want major duplication. Lower it when hunting micro-patterns.

## Enable semantic detection (Type-4)

Structural and token passes are deterministic and run without network. Semantic clones — same behaviour, different syntax — require embeddings:

```bash
deslop . --embeddings required
```

Deslop will use a local Ollama model if one is configured, or degrade gracefully when embeddings are unavailable. See [How It Works](/docs/how-it-works/) for the fusion math.

## Exclude noise

Generated code, vendored dependencies, and migrations should not appear in the report. Configure once per repo in `.deslop.toml`:

```toml
exclude = [
  "**/bin/**",
  "**/obj/**",
  "**/node_modules/**",
  "**/target/**",
  "**/*.Designer.cs",
]

report_hide = [
  "**/*.g.cs",
]
```

`exclude` skips parsing entirely. `report_hide` parses but omits from the final ranking — useful for training-set code you still want in the cache.

## What to do next

1. Read [How It Works](/docs/how-it-works/) to understand the ranking formula.
2. Read [AI Integration](/docs/ai-integration/) if you're wiring Deslop into an agent.
3. Read [Output Formats](/docs/output-formats/) before parsing the JSON yourself.
