# Chapter 10 — Inspect the largest duplicate groups first

Call `duplicates {"detail":"summary","limit":5}` for a short list ranked by duplicated mass. Choose one ID and call `cluster-by-id` to inspect its occurrences. In the editor, use Top Offenders.

Filters apply before pagination. Follow `page.next_offset` with the same filters while `has_more` is true. Full results can omit occurrences because of a payload budget; read `truncated` and `next_action`. `cluster-by-id` also pages large groups with `offset`.

## Compare the copies

Read each occurrence's inputs, outputs, errors, side effects, callers, and tests. List the differences. Decide whether they represent parameters of one operation or independent policies.

![VS Code displays two actual Deslop occurrences side by side after Compare To Canonical.](assets/screenshots/10-compare-to-canonical.png)

*The comparison opens the selected clone ranges. These test setup blocks have identical bytes; that establishes repeated source, not whether shared test setup is the right design.*

Use `compare-pair` when you need the engine's evidence for a specific relation. A chain of matches is not proof that every member matches every other member.

## Choose an outcome

- **Merge** when one owner can preserve behavior, types, dependencies, and performance.
- **Retain** when the copies have a concrete reason to evolve separately or sharing would harm the design.
- **Investigate** when a named missing test or fact blocks the choice.

Record the chosen owner or the reason to retain the copies. Do not hide a finding simply because you decided to keep it.

**Try it:** inspect one real group and record the decision before editing. Rank chooses the investigation order; it does not choose the design.
