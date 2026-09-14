---
layout: layouts/blog.njk
title: "What an Agent Skill Adds to Duplicate Code Detection"
date: 2026-08-21
author: Christian Findlay
tags:
  - posts
  - ai-coding-agents
  - agent-skills
  - duplicate-code-detection
  - mcp
  - code-quality
category: engineering
description: "Kevin Moore wrapped Deslop in an agent skill. What it adds that a detector cannot supply alone, and a checklist for writing one around your own tool."
excerpt: "A detector produces evidence. A skill supplies procedure — a read-only first pass, a stop, a verdict, a test gate, and a reproducible command in the pull request. Here is why that layer matters and how to build one."
heroImage: "/assets/img/blog/agent-skill-duplicate-code-detection-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "A tactile decision gate routes duplicate code slips through inspection, judgment, testing, and either consolidation or deliberate separation."
ogImage: "/assets/img/blog/agent-skill-duplicate-code-detection-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "A paper-and-brass decision gate turns duplicate-code evidence into a deliberate workflow."
---

[Kevin Moore](https://github.com/kevmoo), product manager for Dart and Flutter at Google, published an agent skill called [`deslop-duplication-audit`](https://github.com/kevmoo/kevmoo_skills/tree/main/skills/deslop-duplication-audit) that uses Deslop to perform deduplication on Dart and Flutter code. This post wirth reading out if you're into developer tools, because it does something we could not have done from inside the detector.

Christian Findlay also did a write-up on this in [Dart, Flutter, Duplicate Code, and the Deslop Skill](https://www.christianfindlay.com/blog/dart-flutter-duplicate-code-deslop-skill).

## A detector produces evidence, not decisions

Deslop parses with tree-sitter, normalizes the AST, fingerprints subtrees, clusters them, and ranks what it finds. Our accuracy bar is absolute: every reported cluster is a real duplicate, and every real duplicate is reported. We hold that line hard because a false positive teaches people to ignore the report and a false negative is never discovered at all.

But "this is genuinely duplicated structure" and "you should merge these" are different claims, and only the first one is ours to make. The second depends on type systems, hot loops, module boundaries, release risk, and who owns the file — context a static analyser does not have and should not pretend to have.

So the division of labour is: the tool supplies the evidence, and the agent navigates that evidence and verifies it before relying on it to decide anything. Neither half works alone. Evidence nobody interrogates becomes a work order; an agent with no evidence is guessing.

That gap is exactly where an agent goes wrong. Hand a coding agent a ranked list of duplicate clusters with no procedure attached and it will treat the list as a work order, refactor everything, and hand back a diff nobody asked for. The report was accurate. The outcome was still bad.

The skill closes that gap with procedure.

## The verdict gate is the important part

The heart of the skill is a section called **"Actionable vs. Necessary"**, and it opens by telling the agent to disagree with the tool:

> Do not treat every duplicate finding as a bug or mandatory refactoring target.

It then does the thing that makes the instruction usable — it enumerates the rejection categories concretely rather than asking for judgement in the abstract:

- **Type-unsafe polymorphic targets.** Similar-shaped classes with no shared interface for the property being touched. Unifying them through `dynamic` or casts trades compile-time safety for a handful of lines.
- **Performance-critical specialized loops.** Symmetric traversals in a solver, where the shared abstraction means allocating a closure or dispatching virtually inside the tight path.
- **Speculative wrapping of standalone entry points.** Four to six lines of `try`/`catch` repeated across unrelated `bin/` scripts. Collapsing it hurts scannability and buys nothing.

When a cluster lands in one of those categories, the skill requires the agent to record a verdict of **Rejected**, state the technical rationale in the report, and leave the code untouched.

"Use your judgement" is not an instruction an agent can follow. "Here are three named shapes that look like duplication and should stay duplicated, and you must write down which one applies" is. The difference between those two sentences is most of the value in the skill.

## Read-only first, then a hard stop

Phase 1 scans. It does not write. The bundled helper runs the CLI, parses the JSON report, and emits a compact Markdown summary with source links — no cache directories in the working tree, no dirty git status, no edits.

Phase 2 is a full stop. The agent presents the top clusters and then has to ask two questions before it may touch anything: do you want to remediate, and where should the changes land — current branch, a feature branch, or an isolated worktree?

Ordering discovery before permission sounds obvious and is routinely skipped. It is the difference between a report you can read over coffee and an unrequested forty-file diff.

## Gate on the project's verifier, not the agent's confidence

Phase 4 is empirical, and it runs in the only order that proves anything:

1. Run the tests **before** editing. A red baseline stops the work and gets reported, rather than being blamed on the refactor forty minutes later.
2. Make surgical edits. Touch what the deduplication requires and nothing adjacent.
3. Re-run the analyser and the suite — `dart analyze --fatal-infos`, `dart test` — and require zero errors, zero infos, and a full pass.
4. `git add`, report the line delta and the per-cluster verdicts, and stop. No commit, no push, no pull request until a human says so.

The agent's opinion that the refactor is safe carries no weight anywhere in that sequence. The project's own tooling is the only thing that can clear a change.

## Scoping to the diff — and the gap it exposed

Running a duplicate detector across an existing repository surfaces every legacy cluster in it. During code review that is noise, because the only thing under review is what the change touched.

The skill handles this by intersecting the report with a diff:

```bash
dart run skills/deslop-duplication-audit/bin/deslop_report.dart \
  --dir {repo_dir} \
  --diff-cmd "git diff main...HEAD" \
  --only-changed
```

It supports Jujutsu too — `--diff-cmd "jj diff"` — which matters inside Google.

This is where the skill paid us back directly. Doing that filtering from outside meant parsing unified diffs, computing line-range intersections, and post-processing our JSON. Kevin filed [issue #364](https://github.com/Nimblesite/Deslop/issues/364) asking for `--diff` and `--only-changed` natively in the CLI, separating the two questions review actually asks: did this change introduce a duplicate, and does the changed code clone a helper that already exists elsewhere?

We had not built that. A wrapper written by someone using the tool in anger is a usability specification with a working reference implementation attached, and it is better evidence than any roadmap conversation we could have had internally.

## Provenance in the pull request

The last phase appends a reproduction block to the commit or PR body — the Deslop version, resolved at runtime from `deslop --version`, and the exact command line that produced the findings.

Small feature, disproportionate effect. A reviewer looking at a deduplication diff usually has no way to check the claim behind it. A pinned version and a runnable command turns "the tool said so" into something a reviewer can re-execute. Tool output that cannot be reproduced is an assertion; tool output with the command attached is evidence.

## Tell the agent what the tool gets wrong

Every tool has open defects, and an agent that treats tool output as ground truth will eventually act on a finding the maintainers already know is wrong.

So we publish ours. The [issue graph](/issues/) shows every open issue and how they relate to each other. The [planner](/issues/planner/) shows the runway and the ordered queue, using transparent default effort rather than promised dates. The whole set is machine-readable at [`/assets/data/issues.json`](/assets/data/issues.json), which means a skill can point an agent at the live defect list instead of a snapshot that went stale the week the skill was written. #364 is in there. So is anything currently affecting accuracy, alongside the measured numbers on the [accuracy and transparency page](/docs/accuracy-transparency/).

That generalizes past us. If your tool has a public tracker, name it in the skill and say what the agent should do with it: when a result looks wrong or surprising, check it against the known defects before acting on it, and file a new issue when it does not match one. Knowing which evidence is already disputed is part of verifying the evidence.

## Writing a skill like this for your own tool

An agent skill is a Markdown file with front matter describing when to use it. That is the whole format. The value is entirely in what the body demands. Here is what we would take from this one:

**Start read-only.** The first phase should be incapable of changing the working tree. Discovery and remediation are separate acts and should be separately authorized.

**Name the stop explicitly.** Write "hard stop gate" and specify what has to be answered before work resumes. An agent will not infer a pause from a polite suggestion.

**Give the agent permission to say no, in specifics.** List the named shapes where your tool's output should be rejected, and require a written rationale for each rejection. This is the single highest-value section you can write, and it will only be right if you have used your own tool on real code long enough to be annoyed by it.

**Delegate the verdict to the project's verifier.** Baseline first, then edit, then analyse and test. No step should depend on the agent believing the change is fine.

**Budget tokens as a design constraint.** The skill ships a helper that turns a JSON report into a short Markdown summary with links. Raw analyser JSON will drown a context window and degrade every decision made after it. Compress at the boundary.

**Stage, do not commit.** Leave version control to the human. `git add` plus `git diff --cached --stat` gives them everything they need to decide.

**Emit provenance.** Version and command, in the PR body.

**Point at the tool's known defects.** Link the tracker, and tell the agent to check a surprising finding against it before acting, and to file an issue when nothing matches. A skill that treats its tool as infallible will confidently automate that tool's bugs.

**Say when not to use it.** Kevin's front matter ends with "Don't use for non-Dart projects, non-Git checkouts, or simple single-file syntax lints." A skill that claims to apply everywhere gets invoked when it shouldn't and burns trust the first time it misfires.

**Point the install step at a channel you actually support.** The skill probes `$PATH` first, then falls back. If you write one for your own tool, aim that fallback at your real distribution — for Deslop that is `brew install nimblesite/tap/deslop`, `scoop install deslop`, or the [VS Code extension](/docs/), which bundles the CLI, the LSP, and the MCP server together and version-locked.

## Audit cleans up; prevention stops the next copy

An audit is retrospective. It finds what already got written twice.

The other half is [our MCP and LSP servers](/blog/live-mcp-lsp-duplicate-code-prevention/), which expose `find-similar` so an agent can check the repository *before* it writes a function. On a strong match the agent reuses the existing implementation instead of producing the near-copy that the audit would have flagged next week. Setup for Claude Code, Cursor, and other clients is in the [AI integration docs](/docs/ai-integration/).

Both halves need the same thing from us, and it is not more features. It is that every cluster we report is real and every real cluster gets reported — the standard we hold ourselves to in [Towards 100% Accuracy](/blog/towards-100-percent-accuracy/) and measure in public on the [accuracy and transparency page](/docs/accuracy-transparency/). A skill built on an inaccurate detector automates bad refactors at speed. The procedure only helps if the evidence underneath it holds.

If you find a wrong, stale, or missing result from the CLI, the MCP server, or the LSP, [open an issue](https://github.com/Nimblesite/Deslop/issues). That is how #364 got written, and it is the fastest path we have to a detector worth wrapping.
