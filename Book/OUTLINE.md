# The Deslop Book — chapter plan

## Planned length

The first edition targets about **31,000 words**, **110 print-equivalent pages**, and **36 purposeful visuals**. EPUB pages reflow by device, so the word and visual budgets control scope; the page figure is a design target.

| Material | Words | Print-equivalent pages | Visuals |
|---|---:|---:|---:|
| Front matter | 1,300 | 5 | 1 |
| Part I — Stop duplicates before they are written | 8,000 | 27 | 10 |
| Part II — Make the check part of every change | 8,400 | 29 | 10 |
| Part III — Remove duplication that already exists | 11,900 | 42 | 15 |
| Back matter and glossary | 1,400 | 7 | 0 |
| **Total** | **31,000** | **110** | **36** |

## Example repository used throughout the book

Readers follow an ordinary service repository after several coding agents have worked on it. Each agent reads only part of the repository. As a result, they independently create similar validators, request decoders, command wrappers, fixtures, and repository methods.

The Workshop begins when an agent is about to add one more copy. Part I stops that copy. Part II adds the check to the repository instructions and CI setup. Part III measures the existing duplication, inspects the largest groups, merges the groups that should share code, leaves justified repetition alone, and reruns the repository's tests.

The example is language-neutral at the conceptual level. Edition-specific captures use supported repositories and exact product output. No chapter depends on invented report data.

## What each chapter contains

Every chapter uses the same structure:

1. **The repository state** — a concrete authoring or maintenance decision.
2. **What Deslop reports** — real output from the exact Deslop release used for the book.
3. **The decision** — the choice the developer or agent needs to make.
4. **Before, evidence, action, and after** — steps another developer can repeat.
5. **Workshop exercise** — one small change to the example repository.
6. **Check your understanding** — questions that test the idea and decision, not command memorisation.
7. **Instruction for coding agents** — the exact rule an agent can follow next time.
8. **Main points** — a short summary of the chapter.
9. **Authoritative sources** — adjacent citations plus a short source list.

No chapter introduces more than four new conceptual families. Visuals either provide real product evidence or explain a relationship that product evidence cannot show.

## Front matter — How to use this book

**Target:** 1,300 words · 5 pages · 1 visual

- The prevention-first promise
- Who the book is for: developers, maintainers, and agent-workflow owners
- Prerequisites: ordinary repository, tests, and a supported Deslop editor or agent integration
- How the Workshop checkpoints work
- What Deslop reveals and what it never decides for you
- How to distinguish a report, an interpretation, and a refactoring decision
- How the edition records its Deslop release, platform, and screenshot process
- Visual: the order of the book

## Part I — Stop duplicates before they are written

### Chapter 1 — Why coding agents duplicate code

**Target:** 1,900 words · 6 pages · 2 visuals

**Reader outcome:** Explain why capable agents still create duplication and why prevention belongs inside authoring rather than at the end of CI.

- Files the agent has read versus the rest of the repository
- Why plausible generation reproduces existing behavior under a new name
- DRY as authoritative knowledge; Deslop as evidence of source already repeated
- Why the risk of premature abstraction does not excuse ignoring large identical slabs
- Explicit outcomes: reuse, move ownership, delete, generate, or retain with reason
- Maintenance cost: fixes, tests, policy, and behavior drift multiply per copy
- Prevention, immediate detection, and later cleanup as different costs
- Why the check must happen before a new code unit is written
- Research foundations: syntax-tree comparison, fingerprinting, indexed overlap, and optional semantic search
- Workshop checkpoint: identify the proposed helper and its likely search neighbourhood
- Visuals: DRY versus Deslop; identical code requires a recorded decision

### Chapter 2 — Connect Deslop to agents, editors, and CI

**Target:** 2,000 words · 7 pages · 3 visuals

**Reader outcome:** Explain how the live analysis engine connects the repository, editor, agent, command line, and CI without treating them as separate scanners.

- The live server as the product
- MCP for the coding agent; LSP for the editor; CLI for cold starts and CI
- One analysis implementation and one report format across the editor, agent tools, and CLI
- File changes, live refresh, and focused queries
- How to record the installed version used by the edition
- Workshop checkpoint: verify the agent and editor point at the same workspace
- Visuals: which tool to use; how live updates work; real editor capture

### Chapter 3 — Check for matching code before writing

**Target:** 2,200 words · 7 pages · 3 visuals

**Reader outcome:** Use `find-similar` before writing a function, method, helper, fixture, or other code unit and make a reuse decision from evidence.

- Describe the proposed code before committing it to a file
- Snippet queries versus byte-range queries
- Strong evidence: reuse the canonical occurrence or extract a shared implementation
- Borderline evidence: read the nearest occurrence and bias toward reuse
- Weak or empty evidence: proceed, then recheck the files that changed
- What to do when the live path is unavailable
- Workshop checkpoint: prevent the proposed duplicate and adapt the call site
- Visuals: prevention gate; similarity decision ladder; direct `find-similar` evidence

### Chapter 4 — Understand a Deslop result

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

## Part II — Make the check part of every change

### Chapter 5 — Add the check to agent instructions

**Target:** 2,100 words · 7 pages · 2 visuals

**Reader outcome:** Add clear repository instructions so every compatible agent checks before writing code.

- Put the rule in `AGENTS.md` and `CLAUDE.md`
- Name the code units covered by the rule
- Require `find-similar` before authoring, not after
- Route cleanup work to `top-offenders` and `cluster-by-id`
- Define the fallback when MCP is unavailable
- Prohibit threshold gaming and cosmetic evasion
- Workshop checkpoint: add and exercise the repository rule
- Visuals: instruction-to-tool flow; authoring decision card

### Chapter 6 — Use Deslop evidence to decide what to do

**Target:** 2,100 words · 7 pages · 3 visuals

**Reader outcome:** Interpret similarity evidence, report recommendations, source ranges, and stable group identities with the restraint appropriate to each UX label.

- The fused similarity score as a decision aid
- Structural, textual, and semantic support in plain language
- Byte ranges as the edit authority; line numbers as navigation
- Stable group IDs versus moving rank positions
- Why same-shape and same-behavior findings demand inspection
- Workshop checkpoint: build an evidence note for one proposed change
- Visuals: evidence stack; score bands; byte-range anatomy

### Chapter 7 — Check the result after every code change

**Target:** 2,000 words · 7 pages · 2 visuals

**Reader outcome:** Use the editor bubble, file and range reports, and post-edit rescans to keep prevention active while code changes.

- Human and agent feedback from the same changing repository
- Focused file and selection queries
- Recheck after a large external change
- Confirm that a prevented or consolidated group is gone or smaller
- Avoid stale roots, stale servers, and mismatched binaries
- Workshop checkpoint: edit, observe, query, and verify one live change
- Visuals: reactive loop; focused-query map

### Chapter 8 — Set a duplication limit in CI

**Target:** 2,200 words · 8 pages · 3 visuals

**Reader outcome:** Set a duplication limit for local development and CI, then lower it after verified cleanup without hiding findings.

- Baseline before threshold
- Repository configuration used by developers, agents, and CI
- Exclusion versus report hiding
- Built-in generated and vendor rules
- Exit status and CI behavior
- Lower the limit after real cleanup; never raise it just to pass
- Workshop checkpoint: set a limit that the current repository can meet
- Visuals: matching local and CI configuration; lowering the limit; deciding how to treat generated code

## Part III — Remove duplication that already exists

### Chapter 9 — Measure existing duplication without changing code

**Target:** 2,600 words · 9 pages · 3 visuals

**Reader outcome:** Run a discovery audit that leaves the target repository untouched and present a high-density baseline before choosing any refactor.

- Verify toolchain and repository test commands
- Send report artifacts to a dedicated scratch location
- Disable incremental writes for a read-only discovery run
- Read the main JSON report and the HTML report for different readers
- Summarize worst groups, affected files, and potential consolidation
- Align on scope before changing repository architecture
- Workshop checkpoint: produce a baseline evidence sheet
- Practitioner source: Kevin Moore's Deslop duplication audit protocol
- Visuals: read-only scan; recorded starting state; where the report came from

### Chapter 10 — Inspect the largest duplicate groups first

**Target:** 3,000 words · 10 pages · 4 visuals

**Reader outcome:** Start with `top-offenders`, inspect the full group with `cluster-by-id`, and record whether the group should be merged or left separate.

- Ranking is triage, not truth
- Pull every occurrence before proposing an abstraction
- Duplication worth merging: one concept repeated in several places that can change differently over time
- Duplication that should remain separate: similar code that cannot share an implementation without damaging types, performance, clarity, or test intent
- Accepting a finding explicitly instead of hiding it
- Choose the canonical implementation based on ownership and behavior, not first position
- Workshop checkpoint: accept one group and reject another, with reasons
- Practitioner source: Kevin Moore's decision between duplication worth merging and duplication that should remain separate
- Visuals: largest groups first; group inspection; decision table; retained implementation record

### Chapter 11 — Merge one duplicate group safely

**Target:** 3,200 words · 11 pages · 4 visuals

**Reader outcome:** Make the smallest consolidation that removes duplication while preserving types, behavior, readability, and ownership.

- Prove a passing baseline before modification
- Extract shared helpers and contracts where ownership is clear
- Parameterize meaningful variation without creating flag-driven abstractions
- Keep performance-sensitive specialization when evidence requires it
- Avoid broad formatting and adjacent rearchitecture
- Test fixtures, generated code, and boundary adapters as special cases
- Workshop checkpoint: consolidate one identical or nearly identical group
- Practitioner source: Kevin Moore's rule to change only the code required for the cleanup
- Visuals: choosing the shared implementation; named differences; unsafe abstraction warning; small diff

### Chapter 12 — Verify the cleanup and prevent new copies

**Target:** 3,100 words · 12 pages · 4 visuals

**Reader outcome:** Re-run analysis, static checks, and tests; report the result precisely; then prevent the same duplication from returning.

- Run analyzer, formatter policy, tests, and product-specific gates
- Rescan and compare the original group by stable ID
- Confirm the group disappeared or shrank for the intended reason
- Record retained duplication and its rationale
- Measure code delta without treating line count as the goal
- Lower the duplication limit after measured improvement
- Feed the canonical implementation back into agent instructions
- Workshop checkpoint: close the audit with a before/after evidence record
- Practitioner source: Kevin Moore's baseline and post-refactor verification gate
- Visuals: verification steps; before and after group; lower limit; updated agent instruction

## Back matter

### Appendices and next steps

**Target:** 700 words · 3 pages

- Appendix A — Agent instruction recipe
- Appendix B — Tool-to-job quick reference
- Appendix C — Audit evidence record
- Appendix D — How screenshots and results were captured
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
