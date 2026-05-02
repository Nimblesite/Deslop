# Deslop — Research & Spec

This doc indexes the formal research and design spec for Deslop. **Primary goal:** build a **live duplicate-code analysis server** (LSP + MCP) that stays fast enough to sit in an AI coding agent's inner loop and an editor's keystroke loop — incremental, per-file, byte-range-addressable, cheap to keep live under a file watcher, and accurate across the four clone buckets in [CLONE-BUCKETS] (canonical human-facing labels; academic Type-1 → Type-4 mapping preserved in the same table). The CLI is a secondary surface — same engine, run once, emit a report — used for CI gates and cold-cache audits. Every design decision is judged against whether it still works when the pipeline runs a thousand times per minute, not once per PR.

## Live = Reactive

**This is the load-bearing invariant of the entire product.** *Live* means *reactive*: the moment a change to a watched file produces a new pipeline output, every reader of that output — every editor surface, every webview, every agent — observes the new state **immediately**. Not on the next save. Not when the editor refreshes. Not on a polling timer. Not after a manual command. **Immediately**, in the same microtask that the LSP fires `deslop/reportChanged`. A cluster removed from the source cannot remain visible in the tree, in the bubble, in a hover, in a code lens, in the status bar, in an MCP response, or in any future surface. The CLI is the **only** non-reactive surface in the product — it is the cold-cache fallback for CI gates and one-shot audits, and it exists because reactivity has no meaning for a process that exits before its caller reads the result. Every other surface is reactive by construction; "stale UI" is not a polish defect, it is a correctness bug that fails the brand promise. See [principles.md §[PRINCIPLES-LIVE-IS-REACTIVE]](principles.md#principles-live-is-reactive) for the enforcing rule, [live.md §[LIVE-NOTIFICATIONS]](live.md#live-notifications) for the wire contract, and [vsix.md §[VSIX-REACTIVITY-INVARIANT]](vsix.md#vsix-reactivity-invariant) for the editor-side acceptance test.

The spec is split into topic files for readability and the 500-line file budget. Hierarchical `[GROUP-TOPIC-DETAIL]` IDs (e.g. `[PIPELINE-RANK-WORST-FIRST]`) are stable across the split — `grep -r '\[PIPELINE-' docs/` still finds every reference.

## Canonical clone buckets

Every user-facing surface (HTML report, CLI summary, VS Code extension) labels clusters with the four buckets defined in **[taxonomy.md §[CLONE-BUCKETS]](taxonomy.md)** — `Identical`, `NearlyIdentical`, `LooselySimilar`, `SameBehavior`. That table is the single source of truth. The dual-labelling policy in [CLONE-BUCKETS-DUAL-LABEL] explains when the academic `Type-1 → Type-4` labels appear (JSON + agent-facing surfaces) and when they must not (human UI).

## Architecture at a glance

Every binary is a **thin shell over one shared library** (`deslop-core`). Live analysis is a feature-gated `live` module inside that crate, owned exclusively by `deslop-lsp`. `deslop-mcp` runs no analysis — it reads the state file the LSP writes after every pass. A language is added once, in the core, and every shell inherits it. See [live.md §[LIVE-PACKAGING]](live.md) for the full flow chart.

Every shippable executable and editor package is governed by the Deployment Toolkit manifest contract in [deployment.md](deployment.md).

```mermaid
flowchart LR
    CI(["CI / terminal"])

    subgraph VSCode["VS Code process"]
        VSIX["Deslop VSIX (bubble · tree · webview · status bar)"]
        LspClient2["LSP client"]
        McpHost2["Bundled MCP host"]
        VSIX --> LspClient2
    end

    subgraph JetBrains["JetBrains IDE process (Rider first)"]
        JBPlugin["Deslop IntelliJ Platform plugin"]
    end

    subgraph AgentHost["AI agent host (Claude Code · Cursor · Continue)"]
        Agent["Agent + MCP client"]
    end

    subgraph LspProc["deslop-lsp process"]
        LspInner["AnalysisSession · watcher · scheduler · LiveApi\n(deslop-core live feature linked in)"]
    end

    subgraph McpProc["deslop-mcp process"]
        McpInner["State-file reader + in-memory cache\n(no analysis work)"]
    end

    CliProc(["deslop CLI process\n(one-shot batch)"])

    StateFile[(".deslop-cache/live-report.json")]
    IpcSocket[(".deslop-cache/deslop.sock")]
    DiskCache[(".deslop-cache/\nfingerprints + embeddings")]
    Workspace[(Workspace files)]
    Ollama[(Ollama)]

    LspClient2 == "spawns · LSP stdio" ==> LspProc
    McpHost2 == "spawns · MCP stdio" ==> McpProc
    JBPlugin == "spawns · LSP stdio" ==> LspProc
    Agent == "spawns · MCP stdio" ==> McpProc
    CI == "spawns one-shot" ==> CliProc

    Workspace -- "file events (notify)" --> LspProc
    Workspace -- "walk + read" --> CliProc

    LspProc -- "atomic write after every pass" --> StateFile
    LspProc -- "read/write" --> DiskCache
    LspProc -- "listens" --> IpcSocket
    LspProc <-- "embed batches" --> Ollama

    McpProc -- "reads (cached in-memory)" --> StateFile
    McpProc -- "find-similar · listModels" --> IpcSocket

    CliProc -- "read/write" --> DiskCache
    CliProc <-- "embed batches" --> Ollama
```

The hot loop — **Developer → VSIX → LSP → `live` module → `update_files` → pipeline** — is one process hop. The agent path — **Agent → MCP → state file** — is zero analysis work: the MCP serves the latest report the LSP already produced. The CI path — **CI → CLI → pipeline** — skips `live` entirely.

## Topic files

- [principles.md](principles.md) — `[PRINCIPLES-*]` audience-for-AI-agents, long-running-daemon constraints.
- [taxonomy.md](taxonomy.md) — `[CLONE-BUCKETS]` canonical human-facing buckets (`Identical` / `NearlyIdentical` / `LooselySimilar` / `SameBehavior`), dual-labelling policy, signal routing, and academic `[CLONE-TYPE-TAXONOMY]` reference (Type-1 / Type-2 / Type-3 / Type-4).
- [landscape.md](landscape.md) — `[TECH-*]` survey of token / AST / hashing / neural / LLM techniques (2009 → 2026).
- [fusion.md](fusion.md) — `[FUSION-*]` why Deslop is hybrid (not pure-RAG); embedding + ANN choices; max-sum fusion strategy.
- [pipeline.md](pipeline.md) — `[PIPELINE-*]`, `[STATE-*]`, `[OUTPUT-*]`, `[METRICS-*]`, `[EXIT-CODES]` per-stage design: language plugin trait, discovery, normalization, Merkle fingerprint, clustering, ranking, `[PIPELINE-INCREMENTAL]` on-disk fingerprint cache, JSON / text / HTML output, human-readable HTML mode, repo-wide duplication metric + fail-over threshold.
- [exclusion.md](exclusion.md) — `[EXCLUSION-CONFIG]` `.deslop.toml` `exclude` / `report_hide` tiers and per-language overlays; `[CONFIG-CROSS-LANGUAGE]` candidate-pair language scope.
- [decisions.md](decisions.md) — `[DECISION-*]` defaults with fallback rules (`--min-nodes`, cross-language, two-pass Type-3 recall).
- [reading-list.md](reading-list.md) — `[READ-LIST-DEDUPED]` deduplicated bibliography.
- [live.md](live.md) — `[LIVE-*]` in-process analysis session inside the LSP: lifecycle, watcher, scheduler, state file, IPC socket, delta protocol, `LiveApi` query surface, push notifications.
- [lsp.md](lsp.md) — `[LSP-*]` Language Server Protocol shell: capabilities, diagnostics, code lens, hover, virtual docs, custom methods.
- [mcp.md](mcp.md) — `[MCP-*]` Model Context Protocol shell: tools, resources, notifications. `find-similar` is the keystone tool for AI agents.
- [deployment.md](deployment.md) — `[DEPLOY-*]` Deployment Toolkit manifest, executable version contract, editor-host binary resolvers, VSIX / JetBrains package contents, and release gates.
- [vsix.md](vsix.md) — `[VSIX-*]` VS Code extension: tree view, decorations, webviews, embedding-model picker (Ollama integration), status bar, settings.
- [jetbrains.md](jetbrains.md) — `[JETBRAINS-*]` IntelliJ Platform plugin: Rider-first LSP client, binary resolution, native IDE surfaces, packaging, and testing.
- [competitors.md](competitors.md) — `[COMPETE-*]` landscape of clone-detection tooling (CPD, Simian, jscpd, Sonar CPD, NiCad, ConQAT, SourcererCC) and where Deslop beats them.
- [autofix-extract.md](autofix-extract.md) — `[AUTOFIX-EXTRACT-*]` LSP `refactor.extract` code action that rewrites true Type-1 clusters as a single shared method. v1: pure tree-sitter, no semantic model, blocked on the bucket Type-1 / Type-2 split.

## Sibling docs

- [REPORTING-CONTEXT.md](REPORTING-CONTEXT.md) — embedded `schema_doc` agents see at the top of every JSON report.
- [../plans/PLAN.md](../plans/PLAN.md) — execution plan + live TODO.
