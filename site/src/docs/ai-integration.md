---
layout: layouts/docs.njk
title: AI Integration
eleventyNavigation:
  key: AI Integration
  order: 3
icon: smart_toy
---

# AI Integration

Deslop was designed from the first commit with coding agents as a first-class audience. The MCP and LSP shells ship today — both consume the same `deslop-core` pipeline, the same JSON schema, and the same on-disk caches as the CLI.

## Prevention beats cure — `find-similar` is the keystone

The fastest deduplication is the one that never lands. Deslop's MCP server exposes `find-similar` as the **prevention** tool: an agent calls it *before* writing a new function, helper, or test setup. If the proposed pattern already exists with high similarity, the agent reuses the canonical implementation instead of authoring a fresh copy.

Paste-ready `AGENTS.md` / `CLAUDE.md` snippet that teaches this to your agents lives at [`docs/snippets/agents-md-recipe.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md). It works with Claude Code, Cursor, Copilot, Continue, and Codex.

## The agent loop (live MCP)

The headline workflow is reactive, not batch:

1. Agent proposes a change. Before it writes the new code, it calls `find-similar` over the proposed snippet via MCP.
2. If `find-similar` returns a cluster above the configured similarity floor, the agent reuses the canonical or rewrites the call site.
3. As the agent edits files, the LSP file watcher fires `deslop/reportChanged`. The MCP server reads the freshly written `.deslop-cache/live-report.json` and serves the new state on the next tool call.
4. The agent re-queries `top-offenders` or `report-for-file` to confirm the cluster is gone. No re-run, no flag, no batch CLI invocation.

For a CLI-only loop (CI gates, cold-cache audits, or agents without MCP), the workflow degrades to:

1. Agent proposes code changes.
2. Agent (or harness) runs `deslop . --output report.json`.
3. Agent reads the top `N` clusters from `report.json`.
4. For every cluster above threshold, the agent has three choices: extract to a shared function, reuse the existing implementation, or accept the duplication and annotate why.
5. Agent re-runs Deslop. The top cluster should be different or smaller.

The incremental cache (`--incremental`) means step 5 only pays the cost of parsing files the agent actually touched. On a 1M-LOC monorepo, the warm-cache run returns in single-digit seconds.

## Reading the JSON

Every report begins with an embedded `schema_doc` explaining the shape to the agent consuming it — the model does not need a separate reference to understand the payload:

```json
{
  "schema_doc": "…inline description of every field…",
  "summary": { "clusters_total": 142, "above_threshold": 17, "scan_time_ms": 27110 },
  "clusters": [
    {
      "id": "cl_01HZABC…",
      "score": 2184,
      "bucket": "identical",
      "signals": { "structural": 1.0, "token_jaccard": 0.97, "embedding_cos": 0.91 },
      "summary": "3 near-identical copies of a 42-node method across UserRepository.cs:120-180, ProductRepository.cs:58-118, OrderRepository.cs:40-102 — safe to extract.",
      "suggestion": "extract_shared_function",
      "members": [
        { "path": "UserRepository.cs", "byte_range": [3104, 4820], "lines": [120, 180] }
      ]
    }
  ]
}
```

`summary` is pre-written for an agent reader. It states what was found, where, and — when the signals agree — whether the duplication is safe to extract. The `suggestion` field is filled when the inference is reliable, never guessed.

## Byte ranges, not line numbers

Deslop's source of truth is `[byte_start, byte_end)`. Line numbers are derived at render time only. Agents editing files should slice by byte range — line-based edits drift when surrounding code moves.

## Stable IDs

Cluster IDs are ULIDs generated from the cluster's content fingerprint plus the report timestamp. Feeding the same repo to the same binary twice produces the same IDs. An agent can reference a cluster across runs.

## MCP and LSP — shipping

The `deslop-core` crate owns the entire pipeline. Three shells consume it:

- **MCP server (`deslop-mcp`)** — the agent surface. Tools: `find-similar` (the keystone prevention tool), `top-offenders`, `report-get`, `report-for-file`, `report-for-range`, `cluster-by-id`, `rescan`, `list-embedding-models`, `set-embedding-model`, `session-config`, `schema-doc`. The server reads `.deslop-cache/live-report.json` (atomically rewritten by the LSP after every pass) and delegates `find-similar` to the LSP over a Unix-domain socket so snippet matching runs against the live corpus, not a stale cache.
- **LSP server (`deslop-lsp`)** — the editor surface. Diagnostics, hover, code lens, `textDocument/definition`, virtual `deslop://` documents, and custom `deslop/*` methods (`reportGet`, `reportChanged`, `analysisState`, `pickEmbeddingModel`, `toggleIncremental`). Owns the file watcher, the debouncer (250 ms quiet, 2 s cap), and the analysis scheduler.
- **CLI (`deslop`)** — the cold-cache fallback for CI gates and one-shot audits.

All three reuse the same cache layout (`.deslop-cache/fingerprints/`, `.deslop-cache/embeddings/`) and the same JSON schema. Agents wired to the CLI today get the live channel by adding `deslop-mcp` to their MCP config — no schema change, no parser rewrite.

### Push notifications

The LSP fires `deslop/reportChanged` over the LSP wire and `resources/updated` + `deslop/reportChanged` over the MCP wire as soon as a watcher pass completes. Editor surfaces, agent caches, and webviews all observe the new report in the same microtask. Stale UI is a correctness bug per the [LIVE-IS-REACTIVE](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/principles.md#principles-live-is-reactive) invariant.

## What Deslop deliberately does not do

- It does not rewrite your code. Extraction is your call.
- It does not fail CI unless you wire `--fail-on score>N` yourself.
- It does not assume "near-miss = bug." Some duplication is intentional (test fixtures, bootstrapping). Deslop reports; you decide.
- It does not talk to the network unless you explicitly pick a remote embedding model.
