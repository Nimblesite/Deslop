# Competitive landscape + where CodeDedup wins

Clone detection is a 20-year-old problem; a dozen tools exist. None of them ship the four-way combination CodeDedup targets: **deterministic hybrid core + long-running LSP daemon + MCP for AI agents + first-class VSIX with live Ollama model selection**. This doc inventories the field and locks in the feature bar we must clear to be the default choice.

### [COMPETE-LANDSCAPE] The field (2024–2026)

#### Established detectors

| Tool | Approach | Delivery | Live? | Clone types | Maintenance | Weakness |
|---|---|---|---|---|---|---|
| **PMD CPD** | Token (Rabin-Karp) | CLI, Maven/Gradle | ❌ | Type-1/2, weak 3 | Active | Noisy on generated code, no ranking beyond count, no Type-4, no IDE-live. |
| **Simian** | Line+normalisation | CLI | ❌ | Type-1/2 only | Abandoned ~2019 | Effectively dead. |
| **jscpd** | Token over ~150 langs (regex-ish, not real parsers) | CLI, Node API, GH Action | ❌ | Type-1/2 | Active | Regex-based tokenisation → false positives on imports/boilerplate; slow on monorepos; no AST. |
| **SonarQube / SonarLint** | Token with per-language tokenisers | SaaS + self-host + IDE plugin | Near-live on save | Type-1/2 | Commercial | File-level metric, no cross-file refactor targeting, no MCP, expensive enterprise tier. |
| **JetBrains "Duplicated Code" inspection** | AST (PSI) with anonymisation | Bundled in IntelliJ/Rider/PyCharm/RustRover | In-IDE live on open file | Good Type-2/3 | Active | Ultimate-only for project-wide; no headless CLI; no MCP; no export; locked to JetBrains IDEs. |

#### Research-grade (not shippable as-is)

| Tool | Status |
|---|---|
| **NiCad** | Academic TXL tool; no package, painful build. Strong Type-3 recall on BigCloneBench (~90%) but unusable as a product. |
| **ConQAT** | Abandoned 2014; superseded by **Teamscale** (commercial, enterprise SaaS, no local, no MCP). |
| **SourcererCC** | Apache-2.0 Java tool, scales to 250 MLOC, unmaintained since ~2020. |
| **SSCD / HyClone / SCOTT / Rator** | Research prototypes (2023–2025 papers). GitHub research dumps with no releases. CodeDedup already cites their findings in [fusion.md](fusion.md) and [landscape.md](landscape.md) — we adopt the algorithms, not the artifacts. |

#### Adjacent / AI-native

| Tool | Status for clone detection |
|---|---|
| **CodeScene** | Commercial, token/heuristic "duplication" view. SaaS + on-prem. No MCP, no local embeddings. |
| **Snyk Code (DeepCode)** | Bug-pattern-first; duplication is secondary. Cloud-only, no local models. |
| **Amazon CodeGuru Reviewer** | ML-based recommendations; discontinued for new customers Nov 2024, folding into Q Developer. |
| **Semgrep** | Pattern matching, not clone discovery — user must write the pattern. |
| **GitHub Advanced Security / CodeQL** | No first-party clone detection. |
| **Copilot / Cursor / Claude Code** | Conversational "find duplicates" only — no index, no ranking, no watcher, no deterministic output. |

### [COMPETE-GAP] The unclaimed combination

Breaking the niche into four axes:

- **LSP for clone detection** — nobody. SonarLint and JetBrains do IDE-embedded analysis but neither exposes an LSP server specifically for clones. Every editor but VS Code/JetBrains is uncovered.
- **Long-running watcher daemon** — only Teamscale (server-side, commercial, no local mode). SonarLint is per-file-save, not a full daemon.
- **MCP tool surface for AI agents** — zero clone detectors as of 2026-Q1. General code-search MCP servers exist (Serena, ast-grep wrappers) but none rank clones, and none let an agent ask *"is the block I'm about to write already a clone?"* ([MCP-TOOL-FINDSIMILAR]).
- **Local Ollama embedding selection for Type-4** — no clone detector exposes this. Continue.dev and some RAG tools let you pick embedding models, but they're not clone-specific.

**CodeDedup is the only product that ships all four.** Anyone building the same combination has to rebuild the hybrid core — and we already spent P0–P6 on that.

### [COMPETE-FEATURE-BAR] Features we must clear

Axis-by-axis, the bar we are held to — and the plus-one that wins the category:

#### vs. jscpd (OSS leader on breadth)

- **Match:** three initial languages (C#, Rust, Python), `.gitignore`-respecting discovery, CI-friendly JSON output. ✅ shipped in P1–P4.
- **Beat:** real tree-sitter parsers (no regex tokenisation), AST-level Type-3 via sibling-extension + MinHash, Type-4 via embeddings, stable cluster ids across runs, weight-ranked worst-first output, agent-readable `interpretation` + `action_hints`. ✅ shipped in P2–P5.
- **Category-winning feature:** **live watcher + MCP surface** — jscpd is CI-only; we're live-in-editor.

#### vs. PMD CPD (ubiquity)

- **Match:** Apache-2.0-equivalent (we're MIT-or-Apache dual), scriptable CLI, exit codes suitable for CI gating.
- **Beat:** ranked worst-first output (PMD CPD dumps unranked), byte-range-addressable occurrences (PMD emits line ranges), deterministic cluster ids, per-cluster signal breakdown, human-readable HTML renderer, exclusion tiers (`exclude` vs `report_hide`). ✅ shipped P2–P4.2.
- **Category-winning feature:** **`--incremental` fingerprint cache** — PMD CPD reruns from scratch every time.

#### vs. SonarLint / SonarQube (commercial IDE integration)

- **Match:** VS Code extension with live in-editor feedback.
- **Beat:** free and local-only by default (no SaaS account, no telemetry), Type-3/4 via embeddings (Sonar is token-only), code-lens with per-cluster signal breakdown, jump-across-occurrences via LSP `textDocument/definition`, stable cluster ids for cross-session diffing.
- **Category-winning feature:** **MCP shell + Ollama model picker** — neither SonarLint nor SonarQube expose an MCP surface or allow local embedding-model selection.

#### vs. JetBrains "Duplicated Code" inspection (strongest IDE incumbent)

- **Match:** AST-based detection with identifier/literal anonymisation, live in-editor surfaces.
- **Beat:** editor-agnostic (works in VS Code, Neovim, Helix, Zed, any LSP client — JetBrains is IDE-locked), headless CLI + CI (JetBrains has none), MCP for agents (JetBrains has none), cross-repo stable cluster ids (JetBrains is per-project), local embedding-model choice, exportable JSON/HTML reports.
- **Category-winning feature:** **VSIX + LSP + MCP + CLI from one core** — JetBrains' inspection is trapped inside the IDE.

#### vs. Teamscale (enterprise commercial daemon)

- **Match:** long-running daemon with watcher, incremental re-analysis, ranked report.
- **Beat:** single-machine / local-first (Teamscale is a server product), free/OSS, MCP for AI agents, Ollama embeddings, LSP surface, no licensing friction.
- **Category-winning feature:** **$0 + local-only + MCP + LSP** — we compete on cost, privacy, and agent-readiness at once.

#### vs. ambient AI coding assistants (Copilot / Cursor / Claude Code conversational "find duplicates")

- **Match:** surfaces clone information to an AI agent.
- **Beat:** deterministic ranked index (not token-guessing from chat context), stable cluster ids, cross-repo scan, watcher-driven re-analysis, exact byte ranges, signal-breakdown explainability — everything required for an agent to act reliably rather than guess.
- **Category-winning feature:** **agent-facing MCP tool schema** designed around when the agent should call each tool (see [MCP-AGENT-PROMPT-GUIDANCE]), not a general-purpose code search.

### [COMPETE-MUST-BEAT] Must-beat feature checklist

Things no competitor does that we commit to ship. If any of these slip, we are no longer the obvious choice on our axis:

- [ ] **In-editor live duplication bubble** ([VSIX-LIVE-BUBBLE]) — tells the developer they are duplicating code **as they type**, inline with their cursor. No competitor surfaces duplication at this latency or this granularity. This is the category-defining feature.
- [ ] **Live daemon** — [LIVE-BINARY] + [LIVE-WATCHER] with < 500 ms incremental re-analysis for ≤ 10-file changesets.
- [ ] **MCP `find-similar` tool** ([MCP-TOOL-FINDSIMILAR]) — the only production-ready clone-aware MCP tool in the market.
- [ ] **Ollama model picker in VSIX** ([VSIX-EMBED-PICKER]) — lists installed local models, swaps live, invalidates only the embedding layer.
- [ ] **Stable cluster ids across runs** — ✅ already in v1; no competitor except JetBrains/Teamscale has this, and theirs are per-project.
- [ ] **Byte-range-addressable occurrences, not line numbers** — LSP / agent consumers need byte offsets.
- [ ] **Ranked worst-first, not alphabetical / by count** — ✅ [PIPELINE-RANK-WORST-FIRST].
- [ ] **Three canonical languages with identical output schema** — C#, Rust, Python; no language-specific report shape.
- [ ] **Local-only by default, never phones home** — privacy story is a selling point against every SaaS competitor.
- [ ] **Free + OSS licence** — undercuts every commercial competitor on friction.
- [ ] **VS Code extension UX that's worth opening for its own sake** — [VSIX-PRINCIPLES]; the reference client should be one Marketplace reviewers call out.

### [COMPETE-NON-FEATURES] What we deliberately don't chase

Feature-superiority is bounded. The following are explicit non-goals, and we decline to build them even if a competitor has them:

- **Language breadth past tree-sitter-accurate grammars.** jscpd claims 150 languages via regex tokenisation; the quality is bad. We ship three languages done correctly, then extend.
- **Server / SaaS mode.** Teamscale and Sonar own that corner; we are local-first by architecture ([PRINCIPLES-LONG-RUNNING-DAEMON]).
- **Auto-fix / extract-to-function.** Belongs in refactor tooling, not clone detection. See [LSP-COMMANDS] — the verb slot is reserved, not filled in v1.
- **Execution-based Type-4 validation (HyClone-style).** Research-interesting, product-risky. [DECISION-CROSS-LANGUAGE] and [fusion.md](fusion.md) explain the scope cut.
- **Cloud-hosted embedding provider by default.** We allow one via [FUSION-EMBED-PROVIDER] but we never ship it as the default. Privacy is a feature.

### [COMPETE-POSITIONING] One-sentence positioning

> "The only clone detector that runs as a live daemon in your editor and as an MCP tool for your AI agent — deterministic AST + token fusion + your choice of local Ollama embedding model — no SaaS, no telemetry, no licence."

Every competitor fails at least two of those clauses.
