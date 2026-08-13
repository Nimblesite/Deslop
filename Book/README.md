# The Deslop Book

*The Deslop Book* is a practical field guide to keeping coding agents from duplicating code and repairing repositories where duplication has already accumulated.

The current edition is a **structural scaffold with Chapter 1 complete**. It establishes the learning journey, chapter contracts, vocabulary, source policy, production metadata, visual language, representative opening material, and working EPUB/HTML pipeline. Most later chapters are intentionally outlined rather than presented as finished prose.

## Reader promise

By the end of the finished book, a developer or agent-workflow owner should be able to:

- put Deslop in an agent's authoring loop;
- ask whether equivalent code already exists before creating a new code unit;
- interpret Deslop's human-facing clone labels without academic shorthand;
- use live editor, MCP, CLI, and CI surfaces for the job each does best;
- establish a read-only duplication baseline;
- work from the worst duplicate group downward;
- decide whether a finding should be consolidated or deliberately retained;
- make a surgical refactor behind a passing test baseline;
- prove that behavior survived and duplication fell; and
- ratchet a repository ceiling so the mess does not return.

## The central arc

The book has two jobs, in this order:

1. **Prevent the next duplicate.** The agent calls `find-similar` before authoring and reuses the repository's canonical implementation when the evidence is strong.
2. **Clean up the repository you have.** The team takes a read-only baseline, investigates the highest-impact groups, makes only justified consolidations, and verifies every change empirically.

## Project map

```text
Book/
├── book.json                 # canonical reading order and production targets
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
4. Do not replace plain-language clone names with numbered academic taxonomy.
5. Cite claims beside the sentence they support and use entries from `sources.json`.
6. Distinguish product evidence from explanation: real captures show what Deslop did; diagrams explain the workflow.
7. Never invent a Deslop report, editor surface, command result, benchmark, or signal value in an image.
8. Run `make release` before publishing an edition artifact.

See [OUTLINE.md](OUTLINE.md) for the complete journey and [GLOSSARY.md](GLOSSARY.md) for the UX-aligned vocabulary.
