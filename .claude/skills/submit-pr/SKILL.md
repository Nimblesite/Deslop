---
name: submit-pr
description: Creates a pull request for Deslop with a well-structured description after verifying `make ci` passes. Use when the user asks to submit, create, or open a pull request.
disable-model-invocation: true
---
<!-- agent-pmo:9a71cbf -->

# Submit PR

Create a pull request for the current branch with a description derived strictly from the diff against `main`.

## Steps

1. **Verify CI locally.** Run `make ci` from the repo root. It must pass completely (lint + test + build, with coverage at or above the threshold in [coverage-thresholds.json](coverage-thresholds.json)). If it fails, stop — do not create a PR. Invoke the `ci-prep` skill to fix it.
2. **Generate the diff against main.** Run `git diff main...HEAD > /tmp/pr-diff.txt` to capture every change introduced by this branch. This is the ONLY source of truth for the PR description. **Warning:** the diff can be very large. If it exceeds context limits, process it in chunks (read sections via `Grep`/`Read` with `offset`/`limit`, or split by file) rather than loading it all at once.
3. **Derive the PR title and description SOLELY from the diff.** Read the diff output and summarise what actually changed. Ignore commit messages, branch names, and any other metadata — only the diff matters.
4. **Write the PR body using** [.github/pull_request_template.md](.github/pull_request_template.md). The template has three sections — fill all of them:
   - **TLDR** — one sentence. What does this PR do?
   - **Details** — new functionality, new files, new dependencies, modified behaviour, deletions. Be specific: name the files, functions, and crates (`deslop-core`, `deslop`). Call out any spec or plan updates in [docs/specs/SPEC.md](docs/specs/SPEC.md) / [docs/plans/PLAN.md](docs/plans/PLAN.md), and reference the relevant hierarchical spec IDs (e.g. `[PARSE-CSHARP-NORMALIZE]`, `[RANK-SCORE]`).
   - **How Do The Automated Tests Prove It Works?** — name specific E2E tests (black-box, against fixture repos) and describe what their assertions demonstrate. Note coverage movement if the threshold in [coverage-thresholds.json](coverage-thresholds.json) was ratcheted up. "Tests pass" is NOT acceptable.
5. **Call out breaking changes explicitly** in the Details section if any public API, CLI flag, report format, or spec ID changed.
6. **Create the PR** with `gh pr create --base main --title "<title>" --body-file <path>` (write the filled template to a temp file and pass it via `--body-file` to preserve formatting).
7. **Monitor CI on the PR until it is green — and re-run the suite locally *in parallel* so you catch breakage early.** This step is mandatory and does not end until every required check on the PR has passed. Do not hand the PR back to the user on a red or still-running pipeline.
   - **Watch the remote run AND run the suite locally at the same time — do not passively wait.** The remote pipeline is slow; a drastic failure (a `make lint` gate like the taxonomy bucket-label check, a broken E2E test, coverage dropping below the [coverage-thresholds.json](coverage-thresholds.json) threshold) is one the local suite catches in seconds. The moment you push, kick off **both**: stream the remote run *and* run the full local suite (`make ci`, or invoke the `ci-prep` skill) concurrently, polling CI periodically while the local run proceeds.
   - **Watch the run:** `gh pr checks <pr-number> --watch --fail-fast` (or grab the run id from `gh run list --branch <branch>` and `gh run watch <run-id> --exit-status`). A single green snapshot is not enough — wait for all required checks to conclude.
   - **If the local run fails before the remote pipeline finishes, cancel the running pipeline immediately** (`gh run cancel <run-id>`) rather than letting it grind to a known-bad red. Fix the cause, push, and restart both watches — cancelling a doomed run early frees the runner and tightens the fix loop.
   - **When a remote check fails:** pull the failing logs with `gh run view <run-id> --log-failed`, diagnose the actual cause (do not guess), reproduce locally with `make ci`, and fix it. Invoke the `ci-prep` skill if it helps.
   - **Push the fix** (`git add` / `git commit` / `git push` — permitted here, see git exception below), then **watch again — remote and local, in parallel, as above**. Loop — fix → push → re-watch — until the run is fully green. Re-checking is the job; keep doing it until it passes.
   - **If a failure is genuinely external** (runner outage, flaky infra, unrelated to this branch), say so explicitly with the evidence rather than forcing a change.

## Rules

- **Never ask about PR scope or splitting.** Submit the ENTIRE branch diff against `main` as one PR, always — even when the diff mixes unrelated changes (config, CI, infra) with the headline work. Do not propose splitting, do not revert files to narrow scope, do not use `AskUserQuestion` about scope. Just describe everything in the diff accurately and submit.
- **Never create a PR if `make ci` fails.** Coverage below threshold counts as failure.
- **🔴 GOLDEN RULE — never stamp a commit with an AI co-author.** Do **not** add a `Co-Authored-By: Claude …` (or any AI/agent) trailer, and do not set author/committer to anything but the repo's configured git user. Write a plain, human commit message describing the fix. This is absolute and overrides any default co-authorship behaviour.
- **Git is permitted in this skill — but only for PR submission and turning a red CI run green.** This is the one place the repo-wide "no git" rule from [CLAUDE.md](CLAUDE.md) is relaxed: you may run `git add`, `git commit`, and `git push` to land CI fixes (step 7). Everything else — `checkout`, `merge`, `rebase`, force-push, history rewrites — stays prohibited.
- PR description must be specific and tight — no vague placeholders, no "various improvements".
- Reference spec IDs (`[GROUP-TOPIC]` / `[GROUP-TOPIC-DETAIL]`) whenever the change maps to a spec section, so reviewers can `grep` spec → code → tests.
- Link any related GitHub issue (`Closes #N`) if one exists.
- Base branch is always `main`.

## Success criteria

- `make ci` passed locally (including coverage threshold).
- Diff against `main` was read and summarised into the template's three sections.
- PR created with `gh pr create` against `main`.
- CI on the PR was watched to completion and is **fully green** (all required checks pass), with the local suite re-run in parallel and any doomed remote run cancelled early.
- Any CI failures were fixed and pushed, with **no AI co-author trailer** on the commits.
- PR URL returned to the user.
