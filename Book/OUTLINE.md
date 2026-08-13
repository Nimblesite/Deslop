# The Deslop Book — structural outline

## Shape of the first edition

The first edition targets about **31,000 words**, **110 print-equivalent pages**, and **36 purposeful visuals**. EPUB pages reflow by device, so the word and visual budgets control scope; the page figure is a design target.

| Material | Words | Print-equivalent pages | Visuals |
|---|---:|---:|---:|
| Front matter | 1,300 | 5 | 1 |
| Part I — Prevent the next duplicate | 8,000 | 27 | 10 |
| Part II — Make prevention routine | 8,400 | 29 | 10 |
| Part III — Clean up the repository you have | 11,900 | 42 | 15 |
| Back matter and glossary | 1,400 | 7 | 0 |
| **Total** | **31,000** | **110** | **36** |

## Through-line: the Workshop repository

Readers follow an ordinary service repository after several coding agents have worked on it. The agents are competent, but each enters with limited working context. They independently create similar validators, request decoders, command wrappers, fixtures, and repository methods.

The Workshop begins at the moment an agent is about to add one more copy. Part I prevents that change. Part II turns the successful check into a team-wide authoring contract. Part III returns to the accumulated mess: readers take a baseline, investigate the highest-impact duplicate groups, reject unsafe abstractions, consolidate justified findings, and prove the repository still behaves correctly.

The example is language-neutral at the conceptual level. Edition-specific captures use supported repositories and exact product output. No chapter depends on invented report data.

## Recurring chapter contract

Every chapter follows the same rhythm:

1. **The repository state** — a concrete authoring or maintenance decision.
2. **Deslop in view** — direct evidence from the pinned product surface when behavior is claimed.
3. **The decision** — the smallest useful idea, expressed in the glossary vocabulary.
4. **Before → evidence → action → after** — a short, reproducible workflow.
5. **Workshop checkpoint** — one bounded change to the running repository.
6. **Agent instruction** — the exact rule an agent can follow next time.
7. **What changed** — a compact result and bridge to the next chapter.
8. **Authoritative sources** — adjacent citations plus a short source list.

No chapter introduces more than four new conceptual families. Visuals either provide real product evidence or explain a relationship that product evidence cannot show.

## Front matter — How to use this book

**Target:** 1,300 words · 5 pages · 1 visual

- The prevention-first promise
- Who the book is for: developers, maintainers, and agent-workflow owners
- Prerequisites: ordinary repository, tests, and a supported Deslop surface
- How the Workshop checkpoints work
- What Deslop reveals and what it never decides for you
- How to distinguish a report, an interpretation, and a refactoring decision
- Edition, release, platform, and capture provenance
- Visual: prevention-to-cleanup reading journey

## Part I — Prevent the next duplicate

### Chapter 1 — The duplication tax of agent speed

**Target:** 1,900 words · 6 pages · 2 visuals

**Reader outcome:** Explain why capable agents still create duplication and why prevention belongs inside authoring rather than at the end of CI.

- Local context versus repository memory
- Why plausible generation reproduces existing behavior under a new name
- DRY as authoritative knowledge; Deslop as evidence of source already repeated
- Why the risk of premature abstraction does not excuse ignoring large identical slabs
- Explicit outcomes: reuse, move ownership, delete, generate, or retain with reason
- Maintenance cost: fixes, tests, policy, and behavior drift multiply per copy
- Prevention, immediate detection, and later cleanup as different costs
- The moment of leverage: before a new code unit is written
- Scholarly lineage: syntax-tree comparison, fingerprinting, indexed overlap, and optional semantic retrieval
- Workshop checkpoint: identify the proposed helper and its likely search neighbourhood
- Visuals: DRY versus Deslop; identical code needs a verdict

### Chapter 2 — Put Deslop in the loop

**Target:** 2,000 words · 7 pages · 3 visuals

**Reader outcome:** Explain how the live analysis engine connects the repository, editor, agent, command line, and CI without treating them as separate scanners.

- The live server as the product
- MCP for the coding agent; LSP for the editor; CLI for cold starts and CI
- One analysis engine and one report shape across surfaces
- File changes, live refresh, and focused queries
- Installation and version provenance for the edition
- Workshop checkpoint: verify the agent and editor point at the same workspace
- Visuals: system map; live update loop; real surface capture

### Chapter 3 — Ask before you author

**Target:** 2,200 words · 7 pages · 3 visuals

**Reader outcome:** Use `find-similar` before writing a function, method, helper, fixture, or other code unit and make a reuse decision from evidence.

- Describe the proposed code before committing it to a file
- Snippet queries versus byte-range queries
- Strong evidence: reuse the canonical occurrence or extract a shared implementation
- Borderline evidence: read the nearest occurrence and bias toward reuse
- Weak or empty evidence: proceed, then recheck the working set
- What to do when the live path is unavailable
- Workshop checkpoint: prevent the proposed duplicate and adapt the call site
- Visuals: prevention gate; similarity decision ladder; direct `find-similar` evidence

### Chapter 4 — Read what Deslop found

**Target:** 1,900 words · 7 pages · 2 visuals

**Reader outcome:** Read Deslop's five human-facing clone labels, duplicate groups, occurrences, ranking, and recommendations without confusing a finding with an instruction to refactor.

- Identical code
- Nearly identical code
- Same shape, different content
- Loosely similar code
- Same behavior, different code
- Group, occurrence, canonical occurrence, score, and worst offender
- Why the label describes evidence while the maintainer owns the decision
- Workshop checkpoint: explain one group in plain language before touching code
- Visuals: vocabulary map; annotated real group

## Part II — Make prevention routine

### Chapter 5 — Teach the agent the law

**Target:** 2,100 words · 7 pages · 2 visuals

**Reader outcome:** Encode the prevention loop in repository instructions so every compatible agent checks before authoring.

- Put the rule in `AGENTS.md` and `CLAUDE.md`
- Name the code units covered by the rule
- Require `find-similar` before authoring, not after
- Route cleanup work to `top-offenders` and `cluster-by-id`
- Define the fallback when MCP is unavailable
- Prohibit threshold gaming and cosmetic evasion
- Workshop checkpoint: add and exercise the repository rule
- Visuals: instruction-to-tool flow; authoring decision card

### Chapter 6 — Use evidence, not hunches

**Target:** 2,100 words · 7 pages · 3 visuals

**Reader outcome:** Interpret similarity evidence, report recommendations, source ranges, and stable group identities with the restraint appropriate to each UX label.

- The fused similarity score as a decision aid
- Structural, textual, and semantic support in plain language
- Byte ranges as the edit authority; line numbers as navigation
- Stable group IDs versus moving rank positions
- Why same-shape and same-behavior findings demand inspection
- Workshop checkpoint: build an evidence note for one proposed change
- Visuals: evidence stack; score bands; byte-range anatomy

### Chapter 7 — Keep the working set live

**Target:** 2,000 words · 7 pages · 2 visuals

**Reader outcome:** Use the editor bubble, file and range reports, and post-edit rescans to keep prevention active while code changes.

- Human and agent feedback from the same changing repository
- Focused file and selection queries
- Recheck after a large external change
- Confirm that a prevented or consolidated group is gone or smaller
- Avoid stale roots, stale servers, and mismatched binaries
- Workshop checkpoint: edit, observe, query, and verify one live change
- Visuals: reactive loop; focused-query map

### Chapter 8 — Turn the ceiling into a quality gate

**Target:** 2,200 words · 8 pages · 3 visuals

**Reader outcome:** Establish a reviewable duplication ceiling in local development and CI, then ratchet it downward without hiding legitimate evidence.

- Baseline before threshold
- Repository configuration as a shared contract
- Exclusion versus report hiding
- Built-in generated and vendor rules
- Exit status and CI behavior
- Ratchet downward after real cleanup; never widen to pass
- Workshop checkpoint: set a ceiling that current evidence can meet
- Visuals: local-to-CI contract; threshold ratchet; generated-code decision

## Part III — Clean up the repository you have

### Chapter 9 — Establish a read-only baseline

**Target:** 2,600 words · 9 pages · 3 visuals

**Reader outcome:** Run a discovery audit that leaves the target repository untouched and present a high-density baseline before choosing any refactor.

- Verify toolchain and repository test commands
- Send report artifacts to a dedicated scratch location
- Disable incremental writes for a read-only discovery run
- Read canonical JSON and the human renderer for different audiences
- Summarize worst groups, affected files, and potential consolidation
- Align on scope before changing repository architecture
- Workshop checkpoint: produce a baseline evidence sheet
- Practitioner source: Kevin Moore's Deslop duplication audit protocol
- Visuals: read-only audit flow; baseline scorecard; report provenance

### Chapter 10 — Work worst-first, then decide

**Target:** 3,000 words · 10 pages · 4 visuals

**Reader outcome:** Start with `top-offenders`, inspect the full group with `cluster-by-id`, and record a justified actionability verdict.

- Ranking is triage, not truth
- Pull every occurrence before proposing an abstraction
- Actionable duplication: one concept repeated with drift risk
- Necessary or deliberate duplication: similar shape whose unification damages types, performance, clarity, or test intent
- Accepting a finding explicitly instead of hiding it
- Choose the canonical implementation based on ownership and behavior, not first position
- Workshop checkpoint: accept one group and reject another, with reasons
- Practitioner source: Kevin Moore's actionable-versus-necessary gate
- Visuals: worst-first funnel; group inspection; verdict matrix; canonical-choice record

### Chapter 11 — Consolidate surgically

**Target:** 3,200 words · 11 pages · 4 visuals

**Reader outcome:** Make the smallest consolidation that removes duplication while preserving types, behavior, readability, and ownership.

- Prove a passing baseline before modification
- Extract shared helpers and contracts where ownership is clear
- Parameterize meaningful variation without creating flag-driven abstractions
- Keep performance-sensitive specialization when evidence requires it
- Avoid broad formatting and adjacent rearchitecture
- Test fixtures, generated code, and boundary adapters as special cases
- Workshop checkpoint: consolidate one identical or nearly identical group
- Practitioner source: Kevin Moore's surgical modification rule
- Visuals: canonical extraction; parameterization boundary; unsafe abstraction warning; bounded diff

### Chapter 12 — Prove the cleanup and hold the line

**Target:** 3,100 words · 12 pages · 4 visuals

**Reader outcome:** Re-run analysis, static checks, and tests; report the result precisely; then prevent the same duplication from returning.

- Run analyzer, formatter policy, tests, and product-specific gates
- Rescan and compare the original group by stable ID
- Confirm the group disappeared or shrank for the intended reason
- Record retained duplication and its rationale
- Measure code delta without treating line count as the goal
- Ratchet the ceiling downward after measured improvement
- Feed the canonical implementation back into agent instructions
- Workshop checkpoint: close the audit with a before/after evidence record
- Practitioner source: Kevin Moore's baseline and post-refactor verification gate
- Visuals: proof loop; before/after group; ratchet; prevention handoff

## Back matter

### Appendices and next steps

**Target:** 700 words · 3 pages

- Appendix A — Agent instruction recipe
- Appendix B — Tool-to-job quick reference
- Appendix C — Audit evidence record
- Appendix D — Screenshot and release provenance
- Where to go next: live documentation, releases, repository, and corrections

### Glossary

**Target:** 700 words · 4 pages

- Exact human-facing clone labels from the current UX
- Report and workflow terms used throughout the book
- Distinction between product labels and editorial decision terms
- Stable links back to the product vocabulary authorities

## Explicitly out of scope for the first edition

- Numbered academic clone taxonomy in reader-facing prose
- A full implementation guide to Deslop's Rust internals
- Blind automatic refactoring
- Treating every reported group as mandatory cleanup
- Competitor scorecards without pinned, reproducible evidence
- Invented reports, IDE screens, terminal output, benchmark data, or signal values
- Promising planned editor or autofix behavior as shipped functionality
