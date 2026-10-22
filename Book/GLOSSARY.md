# Glossary

Titles and tags follow Deslop's [label registry](https://github.com/Nimblesite/Deslop/blob/main/clients/vscode/src/types/report.ts) and [taxonomy](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/taxonomy.md).

## Finding labels

| Title | Meaning | Machine kind | UI tag |
|---|---|---|---|
| Identical code | Same source text, allowing whitespace differences | `identical` | Type I |
| Nearly identical code | Renamed, parameterized, or lightly edited copies | `nearly_identical` | Type II / III |
| Similar code | Established copies with larger edits | `loosely_similar` | Type III |
| Same behavior, different code | Optional semantic evidence of equivalent work; requires inspection | `same_behavior` | Type IV |
| Same shape, different content | Informational similarity, not a clone | `structural_only` | Shape |

No label proves that extracting shared code preserves behavior.

## Report terms

**Cluster / duplicate group:** related source occurrences. A shape-only group is informational rather than a clone.

**Occurrence:** one source range in a group. Paths and offsets describe the source at analysis time.

**Canonical occurrence:** the reference copy used for comparison. The developer chooses the final shared owner.

**Group ID:** an identifier stable while the cloned code is unchanged. Editing that code may change it. Rank is not an ID.

**Mass:** duplicated-code impact used to rank clone groups. It is not confidence or permission to merge.

**Pair evidence:** measurements and a decision about two explicitly named source ranges. Read it through `compare-pair`; clusters do not carry one selected pair's measurements.

**Duplication percentage:** duplicated source lines divided by analysed source lines for the configured included clone kinds. The default includes Identical and Nearly identical code. Shape-only findings never contribute. This percentage does not measure detector accuracy.

**Hidden occurrence:** still analysed but removed from duplication metrics by `report_hide` policy. Exclusion instead removes a file before analysis.

**Generation:** a live analysis revision. Recheck pagination when it changes during an inventory.

## Workflow terms

**Baseline:** the saved Deslop report, configuration, source revision, and check results before editing.

**Duplication ceiling:** the configured maximum percentage. The CLI writes reports and exits `3` when the result exceeds it.

**Read-only audit:** a scan that writes reports outside the target checkout, disables cache writes there, and changes no source or configuration.

**Deliberate duplication:** copies retained for a recorded technical reason, such as independent policies or a fixture that tests repeated code.

**DRY:** a design principle about one authoritative representation of knowledge or intent. A source match alone does not identify that owner.
