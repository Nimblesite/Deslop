# Deslop messaging

This is the source of truth for how we describe Deslop across the repository,
website, release notes, extension listings, and community posts. It is a writing
guide for an open source project, not a sales narrative.

Where the **Voice & Positioning** section of
[the design system](designs/messaging.md) disagrees with this file, this file
wins. That document governs type, colour, spacing, and component copy; it does
not govern positioning.

## The one thing

**Deslop finds duplicate source code and stops coding agents from creating
more.**

Lead with that outcome. Do not make people decode “slop,” LSP, MCP,
tree-sitter, embeddings, or clone taxonomy before they know what the tool does.
Those details prove the promise; they are not the promise.

Dead-code analysis and the broader cleanup vision are secondary capabilities.
They do not belong in the primary website message.

## Canonical descriptions

### One line

**Find duplicate code. Stop coding agents from creating more.**

### Short description

Deslop finds the duplicate code in your repo, ranks what to remove first, and stops coding agents from writing more.

### Homepage lead

**Find duplicate code across C, C#, Rust, Python, TypeScript, Go, Osprey and more.**

Deslop ranks what to remove first and tells your coding agent when similar code
already exists.

### Agent-directory blurb

For MCP server lists, extension marketplaces, and awesome-lists — surfaces where
the reader arrived looking for agent tooling, not for a linter:

> An MCP server that checks whether similar code already exists in the
> repository before your coding agent writes more. Also runs as an LSP server
> for live editor warnings, and as a CLI for CI.

## Message hierarchy

Use these ideas in this order. Most surfaces need only the first three.

1. **Find duplicate code.**
2. **Works across the languages your repo uses** (linked, never listed inline).
3. **Ranks what is worth removing first.**
4. **Tells coding agents when similar code already exists.**
5. **Updates live in the editor.**
6. **Runs locally and is open source.**

The protocol follows the outcome: editor warnings come from the LSP server;
agent checks come from the MCP server. Avoid leading with either acronym.

Item 4 moves to the front on agent-native surfaces — MCP directories, Claude
Code and Cursor communities, agent-tooling roundups — where the reader already
has the vocabulary and is searching for exactly that. It never moves to the
front on the homepage.

## Where the audience is

- **Lead with the outcome people want:** clean up duplicate code in a repo, find
  duplicate code, duplicate code detection. Give *duplicate code* a scope
  (*across a codebase*, *in source code*) so it is not read as a single
  duplicated line in an editor.
- **Name the agents and the protocol plainly:** Claude Code, Cursor, GitHub
  Copilot, MCP server. That audience is far larger than the duplicate-code one.
- **Meet the tools people already use:** jscpd, SonarQube, ReSharper, linters.
  Position Deslop as the jscpd alternative that also stops agents adding copies.
- **Adjacent topics carry the readers:** refactoring, code smell, code quality,
  code review, AI code review, static analysis, technical debt.
- **Language pages lead with the language:** *duplicate Python code*, *duplicate
  C# code across a .NET solution*, then work review, refactoring and the
  language's own tooling into the body ([SITE-PYTHON-LANDING]).

## Accuracy claims against other tools

- State **method differences**, which are verifiable: tree-sitter parsers, not
  token streams; edited copies (renamed identifiers, changed literals, inserted
  statements) as well as exact ones; worst-first ranking; live editor and agent
  checks.
- Never write *more accurate than jscpd* (or any tool) until a mechanically
  generated head-to-head report in this repo measures both tools on the same
  judged pairs. Link that report wherever the claim appears.
- Until then, point at the judged clone registers and Accuracy Transparency,
  and say the results cover the judged examples only. No superlatives, no
  "first", no "best".

## Search language

**Searched terms are mandatory, everywhere.** Copy that never uses the words
people type is invisible to them, however good it is. Every page, post, listing
and release note works the searched terms that are true of Deslop into its
titles, headings, opening paragraph, body, link text and image alt text, and
into every description, JSON-LD block and marketplace listing.

- **Titles and headings:** *duplicate code* with a scope, *duplicate functions*
  with *source code*, *repeated code*, the agent names and the language names.
- **Body and metadata:** VS Code, Claude Code, Cursor, GitHub Copilot, MCP
  server; code quality, code review, AI code review, static analysis,
  refactoring, code smell, technical debt; .NET, C# and F#; every supported
  language name, spelled exactly.
- **Technical synonyms, for readers who know the literature:** code clone
  detection, Type-2 clones, code deduplication (with source-code context, so it
  is not read as storage deduplication).
- **Comparisons, never self-description:** linter, SonarQube, ReSharper, jscpd.
  Say what Deslop adds to the tools people already search for; never call it
  one of them.
- **Not worth a title:** duplicate code checker, duplicate code finder, copy
  paste detector. They may still appear in body copy where they read naturally.

Weave the terms in; don't stack them. A searched term belongs wherever a reader
would expect that word: “find duplicate C# and F# code across a .NET solution
before it reaches code review” carries five searched terms and still tells the
reader exactly what happens. Keyword soup is the failure mode, not keywords: a
comma list of synonyms in visible prose, the same phrase repeated with nothing
new to say, or a term that isn't true of Deslop.

## Entity clarity

AI search systems extract entities. Each of these gets one plain definition, in
one place, in visible page text — not in a tooltip, not only in markup:

| Entity | Definition to use |
| --- | --- |
| Deslop | A live duplicate-code analysis server for a codebase and the agents working in it. |
| Duplicate code | The same logic written more than once, even when names and literals differ. |
| Clone cluster | A group of code locations that all match each other. |
| Canonical occurrence | The occurrence in a cluster to keep; the others are the copies. |
| Worst-first ranking | Clusters ordered by duplication impact, biggest payoff at the top. |
| LSP server | The process that shows duplicate-code warnings in the editor. |
| MCP server | The process a coding agent asks before writing new code. |
| Type-2 clone | A copy where identifiers and literals changed but the structure did not. |

Definitions stay identical everywhere they appear. Two different definitions of
one entity are worse than none.

## Language statement

**Never count the languages, and never hardcode the list in copy.** A count or a
fixed list goes stale the moment a parser ships. The one source of truth is
`site/src/_data/languages.js`, rendered on the homepage language panel
(`https://deslop.live/#languages`).

- The README, the VS Code listing, the extension `description` and the agent
  blurbs link to that panel instead of naming languages.
- Generated surfaces (page titles, meta descriptions, `SoftwareApplication`
  JSON-LD, marketplace keywords) may name languages only when they are generated
  from `languages.js`.
- Java and C++ are roadmap languages, not shipping ones.

## The agent message

The agent angle is the differentiator, not a separate story.

Use:

> Deslop tells your coding agent when similar code already exists, before it
> writes another copy.

Then explain `find-similar` and MCP if the reader wants implementation detail.
Treat agents as participants in the development loop, not as the enemy.

The search data makes this the highest-leverage sentence we have: it is the one
claim that connects the small, precise audience searching the symptom to the
very large one searching the agents. Say it on every surface. Lead with it only
where the reader arrived through agent tooling.

## Proof order

Show evidence in the same order as the message:

1. A real ranked duplicate-code report.
2. A real editor warning and canonical comparison.
3. The supported-language list.
4. The MCP check an agent makes before writing.

Use genuine product screenshots and genuine report output. Do not recreate
terminal output, paths, scores, or agent decisions as decorative HTML.

## Voice

Be direct, technical, and a little irreverent. Make strong claims about the
problem and precise claims about the implementation.

- Lead with the outcome.
- Prefer short, active sentences: “Find duplicate code.” “Worst offenders
  first.” “Check before writing.”
- Say **project**, **tool**, or **server**, not **product** or **platform**.
- Say **users**, **developers**, **maintainers**, or **contributors**, not
  **customers**.
- Say **open source** when relevant; do not turn it into a slogan.
- Avoid “revolutionary,” “complete,” “effortless,” “AI-powered,” and “the only
  tool.”
- Avoid calling contributors, languages, frameworks, or AI-generated code
  “slop.”

## Claims and evidence

Every factual claim in public copy is one of three things:

1. **Something the tool does**, stated in present tense and demonstrable in a
   screenshot or a command someone can run.
2. **Something measured**, published with the measurement — what was measured,
   on what, when.
3. **Someone else's finding**, published with a link to the source.

A claim that fits none of these does not ship. “Teams waste X% of their time on
duplicate code” without a linked study is not a claim, it is a liability. Cut it
or source it.

This applies to the search data above: it carries its pull date and its method
so anyone can re-run it and contradict us.

## Writing for AI search

Google's
[AI features guide](https://developers.google.com/search/docs/fundamentals/ai-optimization-guide)
says its AI experiences run on ordinary Search ranking and need no separate
treatment: “you don't need to create new machine readable files, AI text files,
markup, or Markdown,” there is “no requirement to break your content into tiny
pieces for AI to better understand it,” and “you don't need to write in a
specific way just for generative AI search.”

So there is no second, AI-flavoured version of this message. What actually
carries:

- **A unique point of view.** Commodity restatements of what a duplicate-code
  tool is have nothing to extract. What no one else can write: what the ranking
  weighs and why, what an agent asks before it writes, what the report showed on
  a real repository.
- **Answer the question in the first sentence** under each heading, then expand.
- **Structured data that matches the visible text.** The `SoftwareApplication`
  JSON-LD on the homepage carries the same description a human reads on the
  page. If one changes, both change.
- **Crawlable, fast pages.** Nothing load-bearing behind script or interaction.

The AI-targeted JSON report is a different artefact with a different audience
and is out of scope here. This section is about public prose.

## Accuracy boundary

Use present tense for capabilities that ship now:

- finds duplicate code and duplicate functions;
- ranks the worst offenders first;
- supports the listed languages;
- updates as the repository changes;
- warns developers in the editor;
- lets agents check proposed code for similarity;
- reports and gates duplication in CI.

Deslop does not yet remove or rewrite duplicate code autonomously. “Remove
duplicate code” may describe the developer's goal, but the tool's current role
is to find, rank, compare, and prevent it. Use **goal**, **direction**, or
**planned** for automated cleanup.

## The test

Good Deslop copy answers three questions immediately:

1. What does it do? **Finds duplicate code.**
2. Where does it work? **Across C, C#, Rust, Python, TypeScript, Go, Osprey and
   more, live in the editor.**
3. Why is it different? **It also tells coding agents when the code already
   exists.**

If the reader has to understand the architecture before answering those
questions, the copy is off message.
