---
layout: layouts/docs.njk
title: AI Integration
eleventyNavigation:
  key: AI Integration
  order: 3
---

# AI Integration

CodeDedup was designed from the first commit with coding agents as a first-class audience. Everything below is available today in the CLI. The MCP and LSP shells in v2 reuse the same JSON and the same cache.

## The agent loop

A typical agent workflow:

1. Agent proposes code changes.
2. Agent (or harness) runs `codededup . --output report.json`.
3. Agent reads the top `N` clusters from `report.json`.
4. For every cluster above threshold, the agent has three choices:
   - extract to a shared function,
   - reuse the existing implementation,
   - accept the duplication and annotate why.
5. Agent re-runs CodeDedup. The top cluster should be different or smaller.

The incremental cache means step 5 only pays the cost of parsing files the agent actually touched. On a 1M-LOC monorepo, the warm-cache run returns in single-digit seconds.

## Reading the JSON

Every report begins with an embedded `schema_doc` explaining the shape to the agent consuming it — the model does not need a separate reference to understand the payload:

```json
{
  "report_schema_version": "1.0",
  "schema_doc": "…inline description of every field…",
  "summary": { "clusters_total": 142, "above_threshold": 17, "scan_time_ms": 27110 },
  "clusters": [
    {
      "id": "cl_01HZABC…",
      "score": 2184,
      "kind": "Type-2",
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

CodeDedup's source of truth is `[byte_start, byte_end)`. Line numbers are derived at render time only. Agents editing files should slice by byte range — line-based edits drift when surrounding code moves.

## Stable IDs

Cluster IDs are ULIDs generated from the cluster's content fingerprint plus the report timestamp. Feeding the same repo to the same binary twice produces the same IDs. An agent can reference a cluster across runs.

## Schema versioning

`report_schema_version` is semver. Breaking changes bump the major. Additive changes — new fields, new signals — bump the minor. Agents should pin the major version they were written against.

## MCP and LSP (v2)

The `codededup-core` crate is the entire pipeline. The CLI is one shell over it. A second shell will expose:

- an **MCP server** with a `find-similar` tool — given a snippet, return clusters ranked by similarity, updated in real time as the watcher fires;
- an **LSP server** with diagnostics and code lens — surface clusters inline in the editor the same way a linter surfaces warnings.

Both shells reuse the existing cache and the existing JSON schema. Agents wired to the CLI today migrate to the daemon without rewriting their parser.

## What CodeDedup deliberately does not do

- It does not rewrite your code. Extraction is your call.
- It does not fail CI unless you wire `--fail-on score>N` yourself.
- It does not assume "near-miss = bug." Some duplication is intentional (test fixtures, bootstrapping). CodeDedup reports; you decide.
- It does not talk to the network unless you explicitly pick a remote embedding model.
