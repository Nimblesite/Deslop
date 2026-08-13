# The Deslop Book

*The Deslop Book* is a practical field guide to keeping coding agents from duplicating code and repairing repositories where duplication has already accumulated.

Chapters 1 and 2 are complete. The remaining chapters have titles, learning goals, planned sections, exercises, and source lists, but they are not yet full chapters. The EPUB and HTML build already works.

## Reader promise

By the end of the finished book, a developer or agent-workflow owner should be able to:

- connect Deslop to a coding agent before the agent writes code;
- ask whether equivalent code already exists before creating a new code unit;
- interpret Deslop's human-facing clone labels without academic shorthand;
- choose between the editor, agent tools, CLI, and CI for each task;
- establish a read-only duplication baseline;
- work from the worst duplicate group downward;
- decide whether a finding should be consolidated or deliberately retained;
- make a minimal refactor after recording a passing test result;
- prove that behavior survived and duplication fell; and
- lower the allowed duplication limit after cleanup so later changes cannot restore the removed copies.

## What the book teaches

The book has two jobs, in this order:

1. **Stop new copies.** The agent calls `find-similar` before writing code and reads the existing implementation when Deslop finds a strong match.
2. **Remove existing copies.** The team records the current Deslop report and test result, inspects the largest groups, merges only the groups that should share an implementation, and reruns the tests and Deslop.

## Project map

```text
Book/
├── book.json                 # reading order and production targets
├── metadata.yaml             # publication metadata
├── OUTLINE.md                # detailed chapter architecture
├── EDITORIAL-BRIEF.md        # audience, voice, teaching pattern, scope
├── SOURCE-POLICY.md          # authority, evidence, and attribution rules
├── VISUAL-DESIGN-SYSTEM.md   # book adaptation of the Deslop design system
├── GLOSSARY.md               # manuscript vocabulary aligned with the product UX
├── sources.json              # approved source ledger
├── evidence.json             # chapter claim-readiness ledger
├── figures.json              # planned and completed visual ledger
├── manuscript/               # front matter, chapters, and appendices
├── examples/                 # planned running workshop and evidence captures
├── assets/
│   ├── brand/
│   ├── cover/
│   ├── diagrams/
│   ├── illustrations/
│   └── screenshots/
├── styles/                   # EPUB and standalone HTML styling
└── dist/                     # generated output; never hand-edited
```

## Production commands

```sh
make check          # validate manifests, source files, and Markdown parsing
make render-assets  # render deterministic SVG masters to publication PNGs
make epub           # build and validate the structural EPUB
make html           # build a standalone HTML reading copy
make release        # run checks and produce both formats
```

## Drafting rules

1. Treat `book.json` as the source of reading order and production targets.
2. Treat `GLOSSARY.md` as the source of manuscript terminology.
3. Use the five current human-facing Deslop labels exactly as the product uses them.
4. Do not replace the product's plain-language duplicate labels with numbered academic jargon.
5. Cite claims beside the sentence they support and use entries from `sources.json`.
6. Distinguish product evidence from explanation: real captures show what Deslop did; diagrams explain the workflow.
7. Never invent a Deslop report, editor view, command result, benchmark, or score in an image.
8. Run `make release` before publishing an edition artifact.

See [OUTLINE.md](OUTLINE.md) for the chapter plan and [GLOSSARY.md](GLOSSARY.md) for the terms used by Deslop and this book.
