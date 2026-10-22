# Chapter 4 — Understand a Deslop result

Start with the label, then read the occurrences. A duplicate group is called a `cluster` in JSON and agent tools.

| Visible label | Meaning |
|---|---|
| Identical code | The same source text, allowing whitespace differences |
| Nearly identical code | Copies with renames, changed inputs or constants, or small edits |
| Similar code | Substantial copied work remains after larger edits |
| Same behavior, different code | Optional semantic evidence suggests equivalent work; inspect both implementations |
| Same shape, different content | Informational structural similarity; **not a clone** |

The editor also shows compact tags: Type I, Type II / III, Type III, Type IV, and Shape. Use the plain-language titles when explaining a finding. The [taxonomy](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/taxonomy.md) defines both forms.

![The actual Deslop cluster panel shows Identical code, a Type I tag, three occurrences, duplicated mass, and comparison controls.](assets/screenshots/04-cluster-panel.png)

*A real finding in the Deslop repository. Open an occurrence to inspect its source, or choose Compare To Canonical to compare it with the reference copy.*

## Read the group correctly

The **canonical occurrence** is the reference for comparison, not an instruction about where shared code must live. **Mass** measures duplicated code for ranking. **Rank** tells you which clone group to inspect first, not how safe extraction would be.

Shape-only findings have no duplicated mass and do not count toward duplication totals. Similar code means an established edited copy, not merely weak textual overlap.

**Try it:** open one group, read every occurrence, and name one reason to merge it and one reason to keep it separate. Do this before proposing a helper.
