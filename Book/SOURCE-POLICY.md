# Source policy

## Authority order

The book uses the narrowest authoritative source available:

1. Deslop specifications for intended product behavior and vocabulary
2. Deslop implementation and tests for shipped behavior
3. Direct captures from the edition's pinned artifacts
4. Deslop's public documentation for supported workflows
5. Attributed practitioner protocols for field-tested process guidance
6. Primary research for the detection mechanisms and agent-repetition claims that underpin the book

The source ledger in `sources.json` records approved sources and their roles. A source entry is permission to investigate, not permission to copy prose.

## Claims beside evidence

Product claims are cited beside the sentence they support. A chapter-ending list helps readers continue, but it never substitutes for adjacent attribution.

When the book explains Deslop's scholarly lineage, it links directly to the original paper or official DOI record. Short quotations are attributed beside the claim; the surrounding explanation remains original prose.

Commands, UX labels, field names, thresholds, and installation paths are version-sensitive. They must be verified against the edition's pinned Deslop build before a chapter moves from outline to publishable prose.

## UX vocabulary contract

Reader-facing clone names come from two local authorities:

- `clients/vscode/src/types/report.ts` for the exact plain title and action sentence used by the current extension UX
- `docs/specs/taxonomy.md` for the canonical bucket definitions and surface rules

`GLOSSARY.md` translates those authorities into book definitions. Manuscript chapters link to the glossary instead of inventing synonyms. Numbered academic clone taxonomy is outside the manuscript vocabulary.

## Practitioner sources

Kevin Moore's `deslop-duplication-audit` skill is an attributed practitioner source for the cleanup protocol. The book uses its durable sequence—read-only discovery, alignment, actionability, baseline tests, surgical change, and post-refactor proof—while reconciling commands and distribution details with current Deslop documentation.

Practitioner guidance never overrules a repository's own safety, testing, branch, or toolchain rules. When guidance is specific to Dart or Flutter, the book either keeps that scope explicit or restates only the language-neutral principle.

## Product evidence versus diagrams

- A screenshot proves what Deslop, an editor, or a terminal displayed.
- A diagram explains a workflow, decision, or relationship.
- A generated illustration may establish a non-factual part-opening concept.

A diagram or generated image never contains invented report data, source code, interface copy, diagnostic text, benchmarks, or commands. If a real capture is unavailable, the product claim remains unillustrated.

## Release pinning

The scaffold records a development build because the long-form edition version has not yet been selected. Before substantive drafting:

1. set the release and artifact hashes in `book.json`;
2. update `metadata.yaml`;
3. capture all product evidence from those artifacts; and
4. mark each verified chapter in `evidence.json`.

Unverified plans and proposals may shape the future outline, but they are never described as shipped behavior.

## Corrections

When a source, implementation, and UX disagree, the book omits the claim. The discrepancy belongs in the product repository, not in a reader-facing caveat that asks readers to guess which behavior is real.
