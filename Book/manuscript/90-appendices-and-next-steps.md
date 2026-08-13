# Appendices and next steps

> **Scaffold status:** Appendix topics are selected. Commands will be checked against the exact Deslop release used for the finished edition.

## Appendix A — Agent instruction recipe

A paste-ready `AGENTS.md` and `CLAUDE.md` rule based on Deslop's official repository instructions. It will state when to check, what each score range means, which tools handle existing duplication, what to do when the live connection fails, and which changes merely hide a finding.

## Appendix B — Tool-to-job quick reference

| Job | Tool or view |
|---|---|
| Ask whether proposed code already exists | `find-similar` |
| Start cleanup at the highest-impact groups | `top-offenders` |
| Inspect every occurrence in one group | `cluster-by-id` |
| Investigate one active file | `report-for-file` |
| Investigate one selected source range | `report-for-range` |
| Refresh after large external changes | `rescan` |
| Read the current report-field definitions | `schema-doc` once per session |
| Run a standalone audit or CI check | Deslop CLI |

## Appendix C — Audit evidence record

A reusable template for the starting results, the decision to merge or retain code, the files changed, tests before and after, the stable group comparison, duplication left in place, and the updated duplication limit.

## Appendix D — How screenshots and results were captured

The exact Deslop files and hashes, editor version, operating system, example repository state, commands, and screenshot steps used for the edition.

## Where to go next

- Deslop live documentation
- AI-agent integration guide
- Agent-facing prevention guide
- Configuration and CI references
- Releases and repository issue tracker
- Kevin Moore's attributed practitioner audit protocol
