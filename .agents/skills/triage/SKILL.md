---
name: triage
description: Triage all open GitHub issues — correct the issue type (Bug/Feature/Task), apply severity labels (showstopper/critical), link related issues into attackable clusters, and add codebase-backed detail comments to thin tickets. Use when the user says "triage", "triage issues", or "clean up the backlog".
argument-hint: "[optional issue numbers to limit scope]"
allowed-tools: Bash, Read, Grep, Glob
---

# Issue Triage

Triage every open issue (or only `$ARGUMENTS` if given). Read-only on code; writes go to GitHub only. Never close an issue, never remove a label you cannot justify in the comment trail, never edit issue bodies written by others.

## Step 1: Fetch

```
gh issue list --state open --limit 200 --json number,title,body,labels,issueType,comments
```

## Step 2: Type — every issue gets exactly one

- **Bug** — existing behavior is wrong: false positive/negative, crash, quarantine panic, wrong report figures.
- **Feature** — new user-visible capability.
- **Task** — chore, refactor, docs, CI, tech-debt with no user-visible defect.

Set with `gh issue edit <n> --type <Bug|Feature|Task>` (fallback if the flag is unavailable: `gh api graphql` with the `updateIssueIssueType` mutation; list type IDs via the org's `issueTypes`).

## Step 3: Severity labels

- **showstopper** — cannot release: shipped detection reports wrong clusters/figures, main path panics, data loss. Every confirmed false-positive/false-negative in the release path is a showstopper by definition of the accuracy contract.
- **critical** — must fix soon, but a release could ship: defect behind a flag, ignored test, spec drift with no wrong output yet.
- Neither label for everything else.

Also verify domain labels are right: `false-positive`, `false-negative`, `spec-violation`, `ignored-test`, `tech-debt`. An accuracy issue missing its `false-*` label is a triage defect — fix it. Apply with `gh issue edit <n> --add-label ...`.

`fixed-on-main` means fixed but **not yet verified in a release** — leave the issue open, keep its severity labels, and note in the cluster comment that it awaits release verification.

**Severity flows up the dependency tree.** If a blocked issue is showstopper, its root-cause issue is at least showstopper too — a root can never be less severe than anything it blocks. After clustering (Step 4), re-walk each tree from the leaves and raise ancestors as needed.

## Step 4: Cluster related issues

Group issues sharing a root cause, pipeline stage (parse → normalize → fingerprint → cluster → LSH → embeddings → fuse → rank → render), or file/module. For each cluster:

1. Pick the most upstream issue as the anchor.
2. Post **one identical comment on every member**: `**Cluster: <name>** — #a #b #c. Shared root cause: <one sentence>. Fix order: #a → #b → #c.`
3. Where one issue strictly blocks another, record it as a sub-issue or dependency of the anchor (`gh api graphql` `addSubIssue`); otherwise the comment cross-links suffice.

## Step 5: Enrich thin tickets

A ticket is thin if a dev could not start work from it alone. For each, investigate the codebase (Grep/Read the named stage, check spec IDs, run the referenced test if cheap) and comment with facts only — no speculation:

- Affected files with paths and line numbers.
- Suspected root cause and the spec ID (`[GROUP-TOPIC]`) it violates.
- Repro: exact command or failing-test name, expected vs actual.

If investigation uncovers a **new** accuracy defect, the CLAUDE.md quarantine rule applies — stop triage and follow it.

## Step 6: Report

End with a table: issue #, type set, labels added/removed, cluster, comment posted (y/n). Flag anything you could not resolve (e.g. type API rejected, ambiguous severity) for the user to decide.
