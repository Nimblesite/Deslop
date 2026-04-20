# CodeDedup — Research & Spec

This doc indexes the formal research and design spec for CodeDedup. **Primary goal:** pick techniques that are (a) fast enough for a CLI to run on a whole repo, (b) accurate across Type-1 → Type-3 clones (and Type-4 where feasible), and (c) compatible with a future **long-running MCP/LSP** mode — incremental, per-file, byte-range-addressable, and cheap to keep live under a file watcher.

The spec is split into topic files for readability and the 500-line file budget. Hierarchical `[GROUP-TOPIC-DETAIL]` IDs (e.g. `[PIPELINE-RANK-WORST-FIRST]`) are stable across the split — `grep -r '\[PIPELINE-' docs/` still finds every reference.

## Architecture at a glance

Every binary in the product — CLI, LSP server, MCP server, VS Code extension — is a **thin shell over one shared library** (`codededup-core`) and, for anything live, one shared **daemon service** (`codededup-daemon`). No pipeline code is duplicated. A language is added once, in the core, and every shell inherits it.

```mermaid
flowchart TB
    subgraph Clients["Consumers"]
        direction LR
        Developer([Developer typing in editor])
        Agent([AI coding agent — Claude Code / Cursor / Continue])
        CI([CI pipeline])
    end

    subgraph VSIX["VSIX — clients/vscode/"]
        direction TB
        LiveBubble["Live duplication bubble\n(&nbsp;DUPLICATE · NEAR-MISS · SEMANTIC MATCH&nbsp;)"]
        TreeView["Activity bar tree view\n(worst-first clusters)"]
        Webview["Cluster + report webviews"]
        EmbedPicker["Ollama model picker"]
        StatusBar["Status bar"]
    end

    subgraph Binaries["Binaries"]
        direction LR
        LSP["codededup-lsp<br/>(tower-lsp, stdio JSON-RPC)"]
        MCP["codededup-mcp<br/>(MCP over stdio)"]
        CLI["codededup<br/>(batch CLI)"]
    end

    subgraph Daemon["codededup-daemon (long-running service)"]
        direction TB
        Watcher["notify watcher<br/>(250 ms debounce / 2 s cap)"]
        Scheduler["Single-flight scheduler<br/>(&lt;&nbsp;500 ms per 10-file changeset)"]
        Session["AnalysisSession<br/>(Arc&lt;Report&gt; · generation&nbsp;N)"]
        Delta["ReportDelta publisher"]
        Subscribers["Push subscribers<br/>(report/changed · analysis/state)"]
    end

    subgraph Core["codededup-core (the library)"]
        direction TB
        Discover["Discover + exclude\n(ignore + .codededup.toml)"]
        Parse["tree-sitter parse + normalise\n(C# · Rust · Python)"]
        Fingerprint["Merkle fingerprint\n(blake3 subtree hash)"]
        LSH["MinHash / LSH (Type-3)"]
        Embed["Embedding pass (Type-4)"]
        Fuse["Fusion + cluster + rank"]
        Render["JSON · text · HTML renderers"]
        Cache["on-disk caches\n(.codededup-cache/: fingerprints + embeddings)"]
    end

    subgraph External["External processes"]
        Ollama["Ollama HTTP<br/>(/api/tags · /api/embeddings)"]
        FS[(Workspace files)]
    end

    Developer -- "types code" --> VSIX
    Developer -- "edits files" --> FS
    Agent -- "find-similar · report-for-range" --> MCP
    CI -- "codededup path/ --output report" --> CLI

    VSIX <-- "LSP JSON-RPC (stdio)" --> LSP
    VSIX -- "bundles + spawns" --> MCP

    LSP --> Daemon
    MCP --> Daemon
    CLI -- "one-shot run()" --> Core

    Watcher --> Scheduler
    Scheduler --> Session
    Session -- "update_files(changed)" --> Core
    Session --> Delta
    Delta --> Subscribers
    Subscribers -. "report/changed" .-> LSP
    Subscribers -. "report/changed" .-> MCP

    FS --> Watcher
    FS --> Discover

    Discover --> Parse
    Parse --> Fingerprint
    Fingerprint --> LSH
    Fingerprint --> Embed
    LSH --> Fuse
    Embed --> Fuse
    Fuse --> Render
    Parse <--> Cache
    Embed <--> Cache
    Embed <-- "HTTP" --> Ollama

    VSIX -- "embedding/listModels" --> LSP
    LSP -- "list_ollama_models" --> Ollama
    EmbedPicker -. "calls" .-> LSP
```

The arrow from **Developer → VSIX → LSP → Daemon → `update_files` → Core** is the hot loop that delivers the [VSIX-LIVE-BUBBLE] UX: every keystroke the user makes, debounced by 250 ms, ends with the bubble over their cursor if the code they just wrote is already a duplicate. The **Agent → MCP → Daemon → `find-similar`** path is the same live index re-framed for programmatic consumers. **CI → CLI** skips the daemon entirely — batch runs never need a watcher. All three paths share the exact same analysis code in `codededup-core`.

## Topic files

- [principles.md](principles.md) — `[PRINCIPLES-*]` audience-for-AI-agents, long-running-daemon constraints.
- [taxonomy.md](taxonomy.md) — `[CLONE-TYPE-TAXONOMY]` Type-1 / Type-2 / Type-3 / Type-4 ground rules.
- [landscape.md](landscape.md) — `[TECH-*]` survey of token / AST / hashing / neural / LLM techniques (2009 → 2026).
- [fusion.md](fusion.md) — `[FUSION-*]` why CodeDedup is hybrid (not pure-RAG); embedding + ANN choices; max-sum fusion strategy.
- [pipeline.md](pipeline.md) — `[PIPELINE-*]`, `[STATE-*]`, `[OUTPUT-*]` per-stage design: language plugin trait, discovery, normalization, Merkle fingerprint, clustering, ranking, `[PIPELINE-INCREMENTAL]` on-disk fingerprint cache, JSON / text / HTML output, human-readable HTML mode.
- [exclusion.md](exclusion.md) — `[EXCLUSION-CONFIG]` `.codededup.toml` `exclude` / `report_hide` tiers and per-language overlays.
- [decisions.md](decisions.md) — `[DECISION-*]` defaults with fallback rules (`--min-nodes`, cross-language, two-pass Type-3 recall).
- [reading-list.md](reading-list.md) — `[READ-LIST-DEDUPED]` deduplicated bibliography.
- [daemon.md](daemon.md) — `[LIVE-*]` shared long-running service that powers the LSP and MCP shells (lifecycle, watcher, scheduler, delta protocol, query API).
- [lsp.md](lsp.md) — `[LSP-*]` Language Server Protocol shell: capabilities, diagnostics, code lens, hover, virtual docs, custom methods.
- [mcp.md](mcp.md) — `[MCP-*]` Model Context Protocol shell: tools, resources, notifications. `find-similar` is the keystone tool for AI agents.
- [vsix.md](vsix.md) — `[VSIX-*]` VS Code extension: tree view, decorations, webviews, embedding-model picker (Ollama integration), status bar, settings.
- [competitors.md](competitors.md) — `[COMPETE-*]` landscape of clone-detection tooling (CPD, Simian, jscpd, Sonar CPD, NiCad, ConQAT, SourcererCC) and where CodeDedup beats them.

## Sibling docs

- [REPORTING-CONTEXT.md](REPORTING-CONTEXT.md) — embedded `schema_doc` agents see at the top of every JSON report.
- [../plans/PLAN.md](../plans/PLAN.md) — execution plan + live TODO.
