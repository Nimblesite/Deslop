---
name: triage
description: Triage all open GitHub issues — correct the issue type (Bug/Feature/Task), set the Priority field and the lane/* label, link related issues into attackable clusters, and add codebase-backed detail comments to thin tickets. Use when the user says "triage", "triage issues", or "clean up the backlog".
argument-hint: "[optional issue numbers to limit scope]"
allowed-tools: Bash, Read, Grep, Glob
---

# Issue Triage

Triage every open issue (or only `$ARGUMENTS` if given). Read-only on code; writes go to GitHub only. Never close an issue, never remove a label you cannot justify in the comment trail, never edit issue bodies except through the log-issue backfill, which preserves all information.

**PRs NEVER close issues.** Beyond never closing issues yourself, triage must prevent GitHub from doing it: any open PR whose body, title, or commits carry a closing keyword (`Fixes/Closes/Resolves/Fixed/Closed #n`) referencing an issue will auto-close it on merge. During every triage run, scan open PR bodies (`gh pr list --state open --json number,body,title`) for those keywords; on a hit, remove the keyword from the PR body (`gh pr edit <pr> --body`) and flag it in the final report. Issues close only through release verification — see the log-issue skill for the full rule.

The published atlas (`scripts/issues/generate_issue_report.py`) reads the Priority field and the `lane/*` label straight off each issue. Getting those two right (Steps 3 and 4) is what makes the site correct — there is no derivation to second-guess.

**Comment voice — applies to every comment posted.** Comments are mechanical triage records attached to the ticket, not replies to the reporter. Never address anyone: no "you", no "thanks for reporting", no "could you confirm", no greetings or sign-offs. State facts about the defect and the code in impersonal declaratives. Where information is missing, record what is missing as a fact ("Repro command not specified"), never as a request.

## Step 1: Fetch

```
gh issue list --state open --limit 200 --json number,title,body,labels,issueType,comments
```

## Step 2: Type — every issue gets exactly one

- **Bug** — existing behavior is wrong: false positive/negative, crash, quarantine panic, wrong report figures.
- **Feature** — new user-visible capability.
- **Task** — chore, refactor, docs, CI, tech-debt with no user-visible defect.

Set with `gh issue edit <n> --type <Bug|Feature|Task>` (fallback if the flag is unavailable: `gh api graphql` with the `updateIssueIssueType` mutation; list type IDs via the org's `issueTypes`).

## Step 3: Priority — the GitHub Priority field, not a label

Priority is the org-level **Priority** issue field. The `showstopper` and `critical` labels are deleted; never re-create them. `gh issue edit` cannot set a field — use `scripts/issues/set_priority.sh <issue> <option>`.

- **showstopper** — a regression on main that has not been released yet, or a recent regression in a release.
- **critical** — seriously impacting accuracy or the usefulness of the tool. A confirmed false positive/negative already sitting in a release is critical, not showstopper: there is nothing left to hold back.
- **normal** — a problem that impacts the usefulness of the tool.
- **low** — don't worry about it for now.

Every open issue carries exactly one value. An issue with none shows on the site as **No priority set** — a triage defect.

Domain labels stay labels: `false-positive`, `false-negative`, `spec-violation`, `ignored-test`, `tech-debt`. An accuracy issue missing its `false-*` label is a triage defect — `gh issue edit <n> --add-label ...`.

`fixed-on-main` means fixed but **not yet verified in a release** — leave the issue open, keep its Priority, and note in the cluster comment that it awaits release verification.

**Apply `fixed-on-main` yourself when the evidence is strong.** If the reported defect is gone on `main` *and* a test covers it — the named failing test now passes, or a test asserting the corrected behaviour exists and is not `#[ignore]`d — add the label and comment the evidence: commit/PR that landed it, and the test name and path that pins it. The ticket then closes after release verification. Never apply it on code reading alone; no covering test means no label.

**Priority flows up the dependency tree.** A root cause is never lower-priority than anything it blocks. After clustering (Step 5), re-walk each tree from the leaves and raise ancestors as needed.

## Step 4: Lane — exactly one `lane/*` label

The published lane is the issue's `lane/<id>` label. Nothing is inferred from title or body.

- **lane/accuracy** — wrong clusters reported or missed.
- **lane/detection** — parse, normalize, fingerprint, cluster, LSH, embeddings, fuse, rank.
- **lane/performance** — incremental analysis, caches, memory, scheduling, throughput.
- **lane/editor** — VSIX, JetBrains, panels, hovers, diagnostics.
- **lane/integrations** — CLI, MCP, actions, agent tooling.
- **lane/delivery** — packaging, signing, releases, deployment.
- **lane/reporting** — rendered reports, metrics, docs.
- **lane/quality** — tests, specs, CI, fixtures, repo health.

Judge the lane by where the fix lands, not by the issue's vocabulary. Apply with `gh issue edit <n> --add-label lane/<id> --remove-label lane/<wrong>`; two lane labels or none is a triage defect, and none publishes the issue as **Unassigned**.

## Step 5: Cluster related issues

Group issues sharing a root cause, pipeline stage (parse → normalize → fingerprint → cluster → LSH → embeddings → fuse → rank → render), or file/module. For each cluster:

1. Pick the most upstream issue — the root cause — as the anchor.
2. **Mark every dependency.** Any issue that depends on another MUST be recorded as blocked by it (sub-issue via `gh api graphql` `addSubIssue`, or the blocked-by relationship where available) — this directionality is how the backlog gets attacked in order. Loose thematic relations with no blocking edge get comment cross-links only.
3. Post **one identical comment on every member**: `**Cluster: <name>** — #a #b #c. Root cause: #a — fix it first; the rest may collapse or shrink once it lands. Fix order: #a → #b → #c.` Always steer work toward the root, not the symptoms.

4. **Retire edges the root outgrew.** Before recording anything, walk the *existing* blocked-by edges: any edge whose blocking issue is `fixed-on-main` (or closed) is stale — the work it gated has landed. Remove it (`gh api graphql` `removeBlockedBy`) and comment on the unblocked issue saying which edge went, that the blocker is fixed on `main`, and what remains. Where the dependent still needs the fix to *ship*, say so and keep a cross-link in place of the edge — a release-verification wait is not a blocked backlog item.

## Step 6: Enrich thin tickets

A ticket is thin if a dev could not start work from it alone. For each, investigate the codebase (Grep/Read the named stage, check spec IDs, run the referenced test if cheap) and comment with facts only — no speculation:

- Affected files with paths and line numbers.
- Suspected root cause and the spec ID (`[GROUP-TOPIC]`) it violates.
- Repro: exact command or failing-test name, expected vs actual.

Structure enrichment comments with the canonical section names defined in the log-issue skill (`.agents/skills/log-issue/SKILL.md` — "Canonical issue body") so humans and agents read issues and comments the same way; do not restate the section list here. That skill also owns the plain-language rule for human sections — apply it to enrichment comments too. The issue atlas excerpt (`plain_excerpt` in `scripts/issues/generate_issue_report.py`) is drawn from the `## TL;DR` section — a backfill changes the site card text, so a missing or empty TL;DR on an open issue is a triage finding.

If investigation uncovers a **new** accuracy defect, the CLAUDE.md quarantine rule applies — stop triage and follow it.

## Step 7: Report

End with a table: issue #, type set, Priority set, lane label, other labels added/removed, cluster, blocked-by, comment posted (y/n). List the root-cause issues separately as the recommended attack order. Flag anything you could not resolve (e.g. type API rejected, ambiguous severity, dependency API unavailable) for the user to decide.
