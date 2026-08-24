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

Deslop finds duplicate functions and repeated code across C#, Rust, Python,
Dart, JavaScript, TypeScript/TSX, PHP, F#, and Go. It ranks what to remove first
and tells coding agents when similar code already exists.

### Homepage lead

**Find duplicate code across nine languages.**

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
2. **Works across nine programming languages.**
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

## What people actually search

Everything in this section comes from Google Trends, pulled **4 August 2026**,
worldwide. Trends reports interest relative to the peak *inside a single
comparison set*, so only ratios drawn from the same set are meaningful. Never
compare a number here against a number from a different set.

### The demand sits in the agent ecosystem, not the analysis vocabulary

In one 12-month set, **`claude code` outranks `sonarqube` by roughly 29×**
(40.9 vs 1.4 mean interest) and `mcp server` outranks it by roughly 6× (8.9).
`claude code` more than tripled inside those twelve months (18.0 → 63.0 across
the two halves).

The people who need Deslop are not searching the language of static analysis.
They are searching the names of the agents writing their code. Copy that reaches
them names those agents and that protocol plainly.

### “Duplicate code” is the right head term and a contested one

Within one five-year set, `duplicate code` leads its whole family: about **7×**
`code duplication` (16.2 vs 2.2) and roughly **50×** `code clone detection`
(0.3). It also roughly **doubled** over those five years (10.4 → 22.0). It is
the only phrase in the family with an audience.

It is also ambiguous in a way that matters. The top related query for
`duplicate code` — even filtered to the Programming category — is **“duplicate
line in visual studio code.”** Those people want the keyboard shortcut for
duplicating a line. Outside that category the related queries are “how to remove
duplicate photos” and “how to remove duplicate contacts.”

So: use `duplicate code`, and never leave it bare. Pair it with a detection verb
and a scope noun — *find duplicate code **across a codebase***, *duplicate code
**in a repository***, *repeated **functions** in **source code***. A headline
that reads as an editor shortcut has failed even when it ranks.

### The technical vocabulary has no audience

Across those sets, `duplicate code detector` and `duplicate code checker` both
sit at **0.0** mean interest. `copy paste detector` (0.6), `copy paste
programming` (0.3), `find duplicate code` (1.0), and `remove duplicate code`
(1.2) are all close enough to the floor to be worthless as headline terms. In
the set they share, `sonarqube` alone runs about **53×** `remove duplicate
code`.

Nobody types the category name. They type a product name, or they type the
symptom. Do not spend a title on “duplicate code detection tool.”

### The growing vocabulary is adjacent, not exact

Five-year growth, each term measured against its own earlier self:
`code review tool` **15×** (1.0 → 15.0), `code smell` **2.6×** (6.1 → 15.9),
`static analysis` **2.5×** (11.8 → 29.0), `refactoring` **+56%** (19.6 → 30.5).
In one 12-month set, `code quality` (63.1) and `AI code review` (52.7) lead
their set outright, and `AI code review` grew 36% inside the year.

Duplicate code is a *symptom* term with a small, growing audience. Code quality
and AI code review are the *category* terms with the large one. Use the category
words in supporting prose and section headings; keep the symptom words in
titles, where intent is sharpest.

Caution on `code review tool`: its top related queries are “binance review” and
“b2b prospecting tool.” It is a phrase for body copy, never a page title.

### “Slop” is a meme, not a search term

`AI slop` tripled in twelve months (5.3 → 16.1) — but every one of its top and
rising related queries is culture, not tooling: “your ai slop bores me,” “what
is ai slop,” “ai slop youtube,” “slop meaning.” Zero developer-tooling intent.

Two consequences. The brand name cannot carry the search load; “Deslop” must
always sit next to a plain description of what it does. And the existing rule
against calling anyone's code “slop” now has evidence behind it: the word
belongs to a cultural argument we are not having.

Same caution for `vibe coding`, which runs about **4×** `duplicate code` in a
shared 12-month set (35.8 vs 8.7). It is a large, current audience whose search
intent is commentary, not tool acquisition. Write to it in blog posts and
release notes. Do not build a product page on it.

## Search language

Given the above, use these and nothing else:

**Head terms — earn the title tag.**

- **duplicate code** — always qualified by scope (*across a codebase*, *in a
  repository*, *in source code*)
- **duplicate functions** — pair with *source code* so it is not read as
  spreadsheet formulas
- **repeated code**

**Entity anchors — earn the subheadings and body copy.**

- **VS Code**, **Claude Code**, **Cursor**, **MCP server**
- **code quality**, **static analysis**, **refactoring**, **code smell**
- the nine language names, used exactly

**Technical synonyms — at most once per page, for readers who know the
literature.**

- code clone detection, Type-2 clones, code deduplication

Pair “code deduplication” with programming-language or source-code context so it
is not confused with storage deduplication.

**Retired.** These appeared in earlier versions of this guide and are no longer
supported by the data: *duplicate code detector*, *duplicate code finder*,
*duplicate code checker*, *duplicate code detection tool*, *copy paste
detector*.

Do not paste keyword lists into prose. Google's
[SEO Starter Guide](https://developers.google.com/search/docs/fundamentals/seo-starter-guide)
calls “excessively repeating the same words over and over” a spam-policy
violation, and says content people find compelling and useful “will likely
influence your website's presence in search results more than any of the other
suggestions.” One clear sentence that answers the query beats ten awkward
variants.

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

When space allows, use the full list exactly once:

> C#, Rust, Python, Dart, JavaScript, TypeScript/TSX, PHP, F#, and Go.

When space is tight, say **nine programming languages** and link to the full
list. Do not imply support for every language. Java and C/C++ are roadmap
languages, not shipping languages.

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
- supports the nine listed languages;
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
2. Where does it work? **Across nine programming languages, live in the
   editor.**
3. Why is it different? **It also tells coding agents when the code already
   exists.**

If the reader has to understand the architecture before answering those
questions, the copy is off message.

## Re-pulling the search data

The agent-tooling vocabulary is moving fast enough that this section ages in
months, not years. Re-pull it when a major surface is rewritten, or every six
months, whichever comes first. Compare terms inside one set, record the date,
and replace the numbers here rather than adding a second set beside them.

The sets behind the numbers above, all worldwide, all categories:

| Window | Terms compared |
| --- | --- |
| 5 years | duplicate code · code duplication · duplicate code detector · code clone detection · copy paste detector |
| 5 years | sonarqube · refactoring · remove duplicate code · find duplicate code · duplicate code checker |
| 5 years | static analysis · code smell · code review tool · dry principle · copy paste programming |
| 12 months | code quality · AI code review · vibe coding · AI slop · technical debt |
| 12 months | claude code · mcp server · sonarqube · cursor rules · ai code quality |
| 12 months | duplicate code · vibe coding · AI slop |

Related queries were pulled per term, both all-categories and inside the
Programming category. The `/website-audit` skill covers the procedure.
