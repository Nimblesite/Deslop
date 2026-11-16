---
name: release-review
description: Aggressive adversarial review of the current branch against the last Deslop release, hunting accuracy regressions first. Use when the user says "adversarial review", "regression review", "review against the release", or before cutting a release.
---

# Release Review

Find what this branch breaks that the last release got right. One measure outranks the rest: every reported cluster is a real duplicate, every real duplicate is reported, every figure is an accurate calculation. You own every verdict.

## Scope

- Last release: `gh release view --json tagName -q .tagName` (fall back to `git describe --tags --abbrev=0`)
- Diff: `git fetch --tags && git diff <tag>...HEAD --stat`, then read every changed file in full, not just hunks
- Rank areas by blast radius, in pipeline order: discover → parse → normalize → fingerprint → cluster → LSH → embeddings → fuse → rank → render. Then config, the live loop, LSP/MCP, the VSIX
- A code review never replaces the corpus run. Also run `scripts/compare-versions.sh <tag> HEAD` ([CORPUS-SCORE-COMPARE]) and read `.corpus/version-compare/reports/compare-<UTC>.md`: every judged pair that moved is a lead

## Hunt

- Read old and new behaviour side by side (`git show <tag>:<path>`); every removed branch, guard, fallback or default is a suspect
- For each changed gate, filter or threshold, state one input it now reports that it should not (false positive) and one it now drops (false negative). Over-normalizing manufactures the first, under-normalizing the second
- Missing data must never decide: an unreadable source, an unresolved tree or an empty frontier that is read as "different" or "identical" is a finding
- Figures: trace every changed percentage, count and bucket label back to its inputs; a label a credit or fold can reach without proof (`Identical` above all) is a finding
- Config: a key that is parsed and read by nothing is a silent no-op. `grep` each new or changed key for a consumer
- Spec, code and tests must agree. `grep` the spec ID in `docs/specs/` before calling something a bug: a rule the spec mandates is not a finding, a disagreement is
- Check the change is wired end to end: Rust core, generated wire models (`docs/models/live-ipc.td`), LSP/MCP capability, extension registration, panel rendering
- Live loop: dropped events, debounce races, stale generations, leaked tasks, blocking in async
- Hard rules in the diff: regex on source, duplication maths outside Rust, legacy or back-compat paths, `unwrap`/`panic!`/suppressions, files ≥ 500 LOC, functions ≥ 20 LOC
- Tests that re-implement the code they test, assert less than their name claims, pass when the detector goes blind, or were deleted or softened are findings
- Run deslop on the changed crates: new duplication in a duplicate detector is a finding

## Tests

- This is a review, not [ci-prep](../ci-prep/SKILL.md): never run `make ci`, `make test`, `make lint` or a whole test suite
- Run a test only to settle a suspicion: the one named test or the one new failing test that proves or refutes a finding (`cargo test <name> -- --nocapture`)

## Legwork

- Up to two `pi` sessions may each take one bounded slice (a file set plus concrete questions); give them the diff range and these rules. Models, registration on Too Many Cooks and PiNotes records follow [AGENTS.md](../../../AGENTS.md)
- Kill a pi that stalls; never wait on it
- Treat pi output as leads, not findings: open the code and confirm every claim yourself before it counts
- Never use sub-agents

## Verdict

- Confirmed = you traced the mechanism in code and can state input → wrong output. Prove it with a failing black-box test on a fixture repo, asserting cluster, occurrence count, file paths, bucket and rank
- Drop anything you could not confirm; say so instead of padding the list
- Label each: regression against the release, or a new bug on the branch

## Report

- Accuracy finding (possible false positive, false negative or wrong figure) → the accuracy quarantine in [AGENTS.md](../../../AGENTS.md): stop, replace the code with a `panic!`, write the failing test, report, wait
- Bug present in the released tag → one issue per bug with the [log-issue skill](../log-issue/SKILL.md); search `gh issue list --search` first
- Branch-only bug → hand it to the branch owner as a failing test or a message; no issue
- Reply with confirmed findings, severity first, each as `file:line` plus input → wrong output, then exactly what you did and did not review, and the path of the corpus comparison report
