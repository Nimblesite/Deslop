# The Deslop Book

A short practical guide to checking code before an agent writes another copy, reading Deslop findings, and removing verified duplicates.

The manuscript is a concise draft. Exercises use the reader's own repository. Actual VS Code screenshots show the Deslop repository; no completed cleanup or separate Workshop fixture is claimed.

## Read and build

```sh
make -C Book check
make -C Book release
```

The build writes a standalone reading copy to `Book/dist/index.html` and a validated EPUB to `Book/dist/the-deslop-book-outline.epub`. The historical EPUB filename remains for existing build consumers.

## Maintain

- `book.json`: reading order and installed-bundle provenance.
- `manuscript/` and `GLOSSARY.md`: reader text.
- `figures.json`: image sources, crops, hashes, alt text, and capture environment reference.
- `evidence.json`: verification scope; screenshots do not prove a cleanup.
- `sources.json`: reference ledger.
- `assets/screenshots/masters/`: untouched actual UI captures.

Correct commands against the installed tool's help and `tools/list`. Keep source ranges current, inspect both `clusters` and `existing`, and separate pair evidence from cluster mass and rank. Never invent UI output or before-and-after results.
