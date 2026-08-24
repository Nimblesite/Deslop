---
name: triage
description: Triage all open GitHub issues — correct the issue type (Bug/Feature/Task), apply severity labels (showstopper/critical), link related issues into attackable clusters, and add codebase-backed detail comments to thin tickets. Use when the user says "triage", "triage issues", or "clean up the backlog".
argument-hint: "[optional issue numbers to limit scope]"
allowed-tools: Bash, Read, Grep, Glob
---

# Issue Triage

Triage every open issue (or only `$ARGUMENTS` if given). Read-only on code; writes go to GitHub only. Never close an issue, never remove a label you cannot justify in the comment trail, never edit issue bodies except through the log-issue backfill, which preserves all information.

Every triaged issue must also land in the **correct published work lane** (Step 4): the site's lanes are derived mechanically from GitHub data by `scripts/issues/generate_issue_report.py`, and triage owns making that derivation come out right — checking it is not optional.

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

## Step 3: Severity labels

- **showstopper** — cannot release: shipped detection reports wrong clusters/figures, main path panics, data loss. Every confirmed false-positive/false-negative in the release path is a showstopper by definition of the accuracy contract.
- **critical** — must fix soon, but a release could ship: defect behind a flag, ignored test, spec drift with no wrong output yet.
- Neither label for everything else.

Note: replace the showstopper label with critical unless the issue would genuinely cause loss of trust. There is no point holding back a release when the issue is already in a released version.

Also verify domain labels are right: `false-positive`, `false-negative`, `spec-violation`, `ignored-test`, `tech-debt`. An accuracy issue missing its `false-*` label is a triage defect — fix it. Apply with `gh issue edit <n> --add-label ...`.

`fixed-on-main` means fixed but **not yet verified in a release** — leave the issue open, keep its severity labels, and note in the cluster comment that it awaits release verification.

**Apply `fixed-on-main` yourself when the evidence is strong.** If the reported defect is gone on `main` *and* a test covers it — the named failing test now passes, or a test asserting the corrected behaviour exists and is not `#[ignore]`d — add the label and comment the evidence: commit/PR that landed it, and the test name and path that pins it. The ticket then closes after release verification. Never apply it on code reading alone; no covering test means no label.

**Severity flows up the dependency tree.** If a blocked issue is showstopper, its root-cause issue is at least showstopper too — a root can never be less severe than anything it blocks. After clustering (Step 5), re-walk each tree from the leaves and raise ancestors as needed.

## Step 4: Work-lane correctness — every triaged issue lands in the right bucket

The published lane is computed by `workstream_for` over the issue's own GitHub data — never by the triager, never by AI. The exact derivation:

1. Labels `false-negative` / `false-positive` force the **accuracy** lane outright (scoring never runs).
2. Otherwise, argmax over the eight lanes' keyword tables in `WORKSTREAMS` (`scripts/issues/generate_issue_report.py`): **4** points per keyword hit in the normalized title, **1** per hit in the first 1600 chars of the normalized body, **3** per hit in the label set (labels match raw with hyphens intact). Substring matching throughout. Ties break to the earliest lane in `WORKSTREAMS` order (accuracy, detection, performance, editor, integrations, delivery, reporting, quality); an all-zero score falls back to **quality**.

The eight lanes: **accuracy** (wrong clusters reported/missed), **detection** (parse/normalize/fingerprint/cluster/LSH/embeddings/fuse/rank), **performance** (incremental, caches, memory, scheduling, throughput), **editor** (VSIX, JetBrains, panels, hovers, diagnostics), **integrations** (CLI, MCP, actions, agent tooling), **delivery** (packaging, signing, releases, deployment), **reporting** (rendered reports, metrics, docs), **quality** (tests, specs, CI, fixtures, repo health).

For every issue touched in this run:

- Judge the correct lane from the defect's substance — the pipeline stage or surface where the fix lands — not from the issue's vocabulary.
- Verify the derivation, from the repo root:

```
gh api repos/Nimblesite/Deslop/issues/<n> > /tmp/issue-<n>.json
python3 -c "import json; from scripts.issues.generate_issue_report import workstream_for; print(workstream_for(json.load(open('/tmp/issue-<n>.json'))))"
```

- Wrong lane? Correct it with the levers triage owns — labels. The current vocabulary scores: `false-positive`/`false-negative` hard-route **accuracy** (apply only on genuine accuracy defects — a false application misroutes with no keyword able to override it); `shipwright` scores 3 for **delivery**; `spec-violation` 3 and `ignored-test` 6 for **quality**; `tech-debt` scores 0 (the keyword is `tech debt`).
- Never edit titles or bodies to steer a lane. A title keyword weighs 4, so most label combinations cannot beat a mis-scoring title (e.g. a performance defect whose title contains "report" derives **reporting**). When the labels cannot produce the correct lane, the rule itself is at fault: flag the issue # and the offending keyword in the final report as a `WORKSTREAMS` gap for the user — do not widen or hand-patch keyword lists during triage.

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

End with a table: issue #, type set, labels added/removed, cluster, blocked-by, lane (derived → correct, and the action taken or `WORKSTREAMS` gap flagged), comment posted (y/n). List the root-cause issues separately as the recommended attack order. Flag anything you could not resolve (e.g. type API rejected, ambiguous severity, dependency API unavailable) for the user to decide.
