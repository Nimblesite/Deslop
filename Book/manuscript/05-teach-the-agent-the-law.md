# Chapter 5 — Add the check to agent instructions

“Check for duplication” leaves the timing unclear. Say that the check happens before writing, and specify what the agent must inspect.

Add this rule to the instruction file your agent reads:

```text
Before adding a code unit larger than a few lines, call find-similar with the proposed snippet and language. Inspect clusters and existing. Read matching source and reuse the existing owner where appropriate. After editing, call duplicates with the changed path and run the repository's tests.

For existing duplication, start with a small duplicates summary and inspect the chosen cluster with cluster-by-id. Read every occurrence before changing it.

If MCP fails, check a usable JSON snapshot or run the matching bundled CLI. State that unwritten-code checking was unavailable. Errors and stale snapshots never prove uniqueness.

Do not hide findings, relax thresholds, or rewrite code merely to evade detection.
```

Functions, methods, fixtures, test setup, and handlers all qualify. Repetition in tests still deserves a decision; it is not automatically a reason to extract shared setup.

Keep one authoritative rule. If several agents use different instruction files, point them at the same maintained policy where their hosts support it. See the [official recipe](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md).

**Try it:** start a fresh agent session and ask for a helper you know already exists. Confirm that the agent checks before editing and reads the match.
