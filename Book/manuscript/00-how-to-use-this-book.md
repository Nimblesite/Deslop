# How to use this book

Coding agents can write a useful helper without noticing that it already exists. Deslop gives the agent a repository-wide check before it adds another copy.

This book covers two jobs: prevent new duplicates, then remove existing ones safely. Use your own repository for the exercises; no separate Workshop repository is supplied.

## Choose a starting point

- **Setting up an agent:** Chapters 2 and 3, then the instruction in Chapter 5.
- **Reading a finding:** Chapters 4 and 6.
- **Cleaning an existing repository:** Chapter 9. Record a baseline, inspect one group, make one change, and verify it.
- **Adding a CI limit:** Chapter 8, after measuring the repository.
- **Seeing an empty panel:** Chapter 13.

![The workflow checks proposed code, reuses existing code, inspects existing groups, and verifies cleanup.](assets/diagrams/00-reading-journey.png)

*Check before writing. After cleanup, run tests and measure again. Lower a duplication limit only when the verified result supports it.*

## Read evidence, then decide

Deslop identifies matching source ranges. You decide whether they should share an implementation. A high rank means a large duplication impact; it does not approve a refactor.

Screenshots come from the running VS Code extension analysing the Deslop repository. They show actual findings, not a cleanup performed for this book. The environment, capture steps, and binary hashes are in `figures.json`. Counts describe that snapshot and will change with the code.

Use the [Glossary](#glossary) for finding labels and the quick reference for tools.
