# Source policy

Use local specifications, implementation, tests, and the installed tool to check product claims. Verify version-sensitive flags through `--help` and tool parameters through `tools/list`.

## Evidence boundaries

- A screenshot shows what the running UI displayed at capture time.
- A diagram explains a workflow; it never substitutes for measured output.
- A snapshot does not prove that analysis is current after an edit.
- A similarity result does not prove that a merge preserves behavior.
- A cleanup claim needs actual before-and-after reports and test results.

Record installed artifact hashes in `book.json` and capture details in `figures.json`. The present screenshots use an existing working checkout. They do not claim an immutable release fixture or release-tag verification.

## Attribution and terminology

Keep product references beside the claims they support. Use the visible titles from `clients/vscode/src/types/report.ts` and definitions from `docs/specs/taxonomy.md`. `GLOSSARY.md` explains them for readers.

Credit practitioner guidance where used; it never overrides the target repository's rules. Approved research sources remain in `sources.json` for further investigation, not as evidence for claims the manuscript does not make.

If a specification and observed behavior disagree, omit the claim until resolved. Never make up a capture to fill a gap.
