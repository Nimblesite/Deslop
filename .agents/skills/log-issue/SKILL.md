---
name: log-issue
description: Log any GitHub issue (bug, feature, or task) via `gh issue create` using the canonical issue body structure — TL;DR and Details for humans, Steps To Reproduce, Details (for AI), Acceptance Criteria (for AI), and Screenshots. Use when the user says "log an issue", "file an issue", "create an issue", "log a bug", or "report a bug", and when backfilling existing issue bodies to the canonical structure.
argument-hint: "[optional issue number to backfill, or nothing to file a new issue]"
allowed-tools: Bash, Read, Grep, Glob
---

# Log Issue

File a Deslop issue on GitHub — bug, feature, or task — so a human can read it in seconds and an AI can act on it without asking questions. `gh issue create` and `gh issue edit --body-file` (backfill only) are the sole exceptions to the no-git rule in AGENTS.md. Never close issues.

## PRs NEVER close issues — no closing keywords, ever

AGENTS.md: "NEVER CLOSE GH ISSUES, EVEN WITH PR COMMENTS!" GitHub auto-closes an issue the moment a merged PR (or a commit that reaches the default branch) contains a closing keyword: `Fixes #n`, `Closes #n`, `Resolves #n`, `Fixed #n`, `Closed #n` — in the PR body, PR title, or commit message, in any case form. Issues close only through the release-verification process, never by a PR.

- Never write a closing keyword against an issue number in any PR body, title, or commit message — write "works on #n", "changes #n", or reference the issue bare (`#n`) instead.
- Before pushing or opening any PR, scan its body and commit messages for `fixes|closes|resolves` followed by `#` — if present, rewrite the wording and reopen the edit.
- If a PR is found carrying a closing keyword against an open issue, that is a defect: remove the keyword from the PR body immediately (`gh pr edit <pr> --body`) and check the issue is still open (`gh issue reopen <n>` if the keyword already fired).

## Human sections are for humans — MINIMAL JARGON

TL;DR, Details, and Steps To Reproduce are read by humans. Write them in plain language a new user understands without knowing Deslop's internals. This is a hard rule, not a style preference.

- Say what the user **sees**, not what the code does: "the hover card disappears while you type", not "projectVisible elides the dirty file's occurrence from visibleReport".
- No pipeline vocabulary in human sections: fingerprint, LSH, bucket, normalization, token jaccard, structural score, projection, canonical occurrence, transitive closure, dirty-aware. Use the words the product shows the user: cluster, duplicate, report, hover card, underline, warning, menu.
- Numbers are fine; jargon is not. "85% of the top findings are normal test code" is good.
- File paths, line numbers, code symbols, and internal identifiers belong in `Details (for AI)` — that section exists so the human sections don't carry them.
- Steps To Reproduce are numbered, one action per step, followable by a stranger: clone, run this command, open this, look at that. No narrative, no explanation — explanation lives in Details.

Bad: "After deduping the genuinely-extractable cross-file clones, ~85% of remaining clusters above weight=1000 are pytest idioms not extractable without obscuring per-test intent."
Good: "Deslop reports normal test code as duplicates. In this Python repo, most of the top findings are everyday pytest patterns — every test sets up the same environment variables, so Deslop thinks they are copies. They are not."

## Canonical issue body

Every issue body — newly filed or backfilled, every type — uses exactly these sections in exactly this order. The headings are machine-parseable: the issue atlas (`scripts/issues/generate_issue_report.py`, `plain_excerpt`) reads the `## TL;DR` section for the site excerpt, so the heading text and level must not vary.

````markdown
## TL;DR

<One or two plain-language sentences a human can act on without reading anything else. Minimal jargon — see the rule above.>

## Details

<Two to three plain-language paragraphs for humans: for a bug — what did you expect to see, what did you see instead, why is that wrong; for a feature — the capability wanted and why; for a task — what needs doing and why now. Technical detail goes under Details (for AI), not here.>

## Steps To Reproduce

<Numbered, one action per step, followable by a stranger. Present for every issue type; content varies — see below.>

## Details (for AI)

```shell
# environment
deslop --version: 0.x.y
os: <macOS / Windows / Linux + version>
surface: cli | vsix | repo
vsix version: 0.x.y            # if applicable

# context                                      # bugs
command: deslop . --output /tmp/deslop --min-nodes 30

# evidence                                     # bugs
cluster id: <id from the report>
occurrence count: <n>
files involved:
  - path/to/first/file.ext
report paths:
  - /tmp/deslop-report.json
key output:
  <exact offending lines from the report or logs>

# scope                                        # features and tasks
type: Feature | Task
affected surfaces: <cli | vsix | mcp | site | ci>
related issues: #<n>
```

## Acceptance Criteria (for AI)

- <Testable assertion: for a bug — exact cluster id, occurrence count, file paths, bucket, ranking order that the pinning test asserts; for a feature — the observable behavior once delivered; for a task — the verifiable end state.>

## Screenshots

<Attach screenshot images if possible — embed hosted URLs with ![...](url). gh cannot upload GitHub user-content assets; for local screenshot files, leave "pending" here and have the user attach them via the web UI.>
````

Drop the bug-only blocks (`# evidence`) from `Details (for AI)` on features and tasks; never drop a section heading.

## Steps To Reproduce — same heading, content varies by type

- **Bug** — exact CLI commands or VSIX interactions that reproduce the wrong behavior from a clean clone, plus the fixture repo URL or the smallest attached sample files.
- **Feature** — the steps a user follows *today* without the capability: the current workflow, the workaround, where it hurts. This is what the feature will replace.
- **Task** — the current state the task cleans up: where the stale code/spec/debt lives and how to see it (command, file, or report that exhibits it).

## Filing

1. Verify the substance before filing: for bugs run the repro command or failing test yourself; for features confirm the capability is absent; never file speculation.
2. Write the body to a file with the skeleton above, every section present. No section is ever omitted — write "None." where a section genuinely has no content.
3. File it:

```
gh issue create --title "<imperative one-liner>" --body-file /tmp/deslop-issue.md [--label bug]
gh issue edit <n> --type <Bug|Feature|Task>   # fallback if the flag is rejected: gh api graphql updateIssueIssueType
```

4. Set the Priority field and the lane — both drive the published atlas:

```
scripts/issues/set_priority.sh <n> <showstopper|critical|normal|low>   # Priority is a field, not a label
gh issue edit <n> --add-label lane/<accuracy|detection|performance|editor|integrations|delivery|reporting|quality>
```

   Add domain labels where they apply: `false-positive`, `false-negative`, `spec-violation`, `ignored-test`. Severity is never a label — the `showstopper` and `critical` labels are deleted.
5. Post screenshot images if possible (see Screenshots section above).

## Backfilling existing issues

When given an issue number to backfill: `gh issue edit <n> --body-file <restructured body>` — reorganize the existing content into the canonical sections, adding `Details (for AI)` / `Acceptance Criteria (for AI)` facts only where the issue or codebase establishes them. Never delete information; content with no natural home goes under `Details`. Never close the issue. One backfill = one `gh issue edit`; verify with `gh issue view <n>` afterwards.

## Agreement contract

Four artifacts must stay in sync — change one, change all:

| Artifact | Role |
| --- | --- |
| `.agents/skills/log-issue/SKILL.md` (this file) | Canonical structure; AI files any issue type with it |
| `.github/ISSUE_TEMPLATE/issue.yml` | Web form for humans filing any issue type; same sections, same order; Steps To Reproduce is optional and marked bugs-only there — agents still keep the heading with type-appropriate content |
| `.agents/skills/triage/SKILL.md` | Reads these sections when enriching tickets |
| `scripts/issues/generate_issue_report.py`, `scripts/issues/rules.py` | Excerpt = TL;DR section; headings are parsed; Priority ladder and lane ids mirror GitHub |
