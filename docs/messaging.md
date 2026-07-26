# Deslop messaging

This is the source of truth for how we describe Deslop across the repository,
website, release notes, and community posts. It is a writing guide for an open
source project, not a sales narrative.

## The idea

Deslop helps keep codebases clean as humans and AI agents change them. It finds
the code that should not be there, stops more of it from landing, and is working
toward helping remove it safely.

Today, Deslop finds duplicate and dead code and prevents new duplication. Those
are the first kinds of slop it tackles, not the limit of the project.

## One-line description

**Deslop finds codebase slop, stops more from landing, and helps you clean it
up.**

When the current capability needs to be explicit:

**Deslop finds duplicate and dead code while you work, and helps stop new
duplication before it lands.**

## Short description

Live duplicate and dead code analysis inside your IDE. Inline warnings as you
type, worst-offender view, and a live channel your AI agent can consult before
it copy-pastes.

This is the canonical project blurb. Use it verbatim in the README and anywhere
that needs a compact description of what Deslop does.

## The three-part story

### Find it

Surface the code that makes a repository harder to understand and maintain.
Rank the worst offenders first so the output leads to action instead of another
backlog.

Today: duplicate and dead code detection, available live and in CI.

### Stop it

Put feedback inside the coding loop. Editors show problems as the repository
changes. Agents can check for an existing implementation before writing a new
one. CI is the final guardrail, not the first time anyone hears about the
problem.

Today: live LSP feedback, MCP tools for coding agents, and CI thresholds.

### Remove it

Turn findings into safe cleanup. Prefer reuse, consolidation, and deletion over
creating another abstraction by reflex. Keep people in control of changes to
their code.

Direction: guided and automated removal. Do not describe code removal as a
shipping capability until it exists.

## Message hierarchy

Use these ideas in this order. Most pages only need the first two or three.

1. **Today, Deslop finds duplicate and dead code.**
2. **Deslop works while code is being written, not only after it lands.**
3. **Humans and coding agents use the same live repository signals.**
4. **The worst offenders come first.**
5. **It is open source, local by default, and built for real repositories.**

## Reusable copy

### Repository subtitle

> Find codebase slop. Stop more from landing. Clean it up.

### README or website lead

> Live duplicate and dead code analysis inside your IDE. Inline warnings as you
> type, worst-offender view, and a live channel your AI agent can consult before
> it copy-pastes.

### Capability lead

> Duplicate code is where Deslop starts. It finds exact, renamed, and similar
> copies; ranks the ones that matter; and puts the result in the editor, the
> agent loop, and CI.

### Vision lead

> The goal is bigger than duplicate detection: find the code that should not be
> there, prevent more of it from appearing, and help remove it safely.

### Very short variants

- Keep slop out of the codebase.
- Catch codebase slop before it spreads.
- Clean code in. Slop out.

## What “slop” means

Slop is code that adds maintenance cost without adding enough value. Duplicate
implementations and dead code are the current focus. Over time, the term can
include redundant abstractions, stale compatibility paths, repeated constants,
and other mechanically detectable waste.

“Slop” describes the code, not the person or agent that wrote it. Avoid using
the term as a judgement about contributors, languages, frameworks, or
AI-generated code in general.

## Voice

Be direct, technical, and a little irreverent. Make strong claims about the
problem and precise claims about the implementation.

- Say **project**, **tool**, or **server**, not **product** or **platform**.
- Say **users**, **developers**, **maintainers**, or **contributors**, not
  **customers**.
- Say **open source** when it is relevant; do not turn it into a slogan.
- Lead with the outcome. Explain LSP, MCP, tree-sitter, and ranking afterward.
- Prefer short, active sentences: “Check before writing.” “Worst offenders
  first.” “The copy never lands.”
- Treat AI agents as participants in the development loop, not as the enemy.
- Avoid inflated claims such as “revolutionary,” “complete,” “effortless,” or
  “the only tool.” Let the architecture and results make the case.

## Accuracy boundary

Keep the mission broad and the present tense narrow.

Use present tense for capabilities that ship now:

- finds and ranks duplicate and dead code;
- updates as the repository changes;
- gives editors and agents live signals;
- checks proposed code for similarity;
- reports and gates duplication in CI.

Use **goal**, **direction**, **working toward**, or **planned** for broader slop
detection and code removal. Never imply that Deslop autonomously edits,
deduplicates, or deletes code until those capabilities ship.

## The test

Good Deslop copy should answer three questions quickly:

1. What is the project trying to do? **Keep slop out of codebases.**
2. What does it do today? **Find duplicate and dead code, and prevent new
   duplication.**
3. Why is it different? **It works inside the live coding loop, before the
   problem lands.**

If the copy answers those questions without sounding like an advert, it is on
message.
