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
   - **Details** — new functionality, new files, new dependencies, modified behaviour, deletions. Be specific: name the files, functions, and crates (`codededup-core`, `codededup`). Call out any spec or plan updates in [docs/specs/SPEC.md](docs/specs/SPEC.md) / [docs/plans/PLAN.md](docs/plans/PLAN.md), and reference the relevant hierarchical spec IDs (e.g. `[PARSE-CSHARP-NORMALIZE]`, `[RANK-SCORE]`).
   - **How Do The Automated Tests Prove It Works?** — name specific E2E tests (black-box, against fixture repos) and describe what their assertions demonstrate. Note coverage movement if the threshold in [coverage-thresholds.json](coverage-thresholds.json) was ratcheted up. "Tests pass" is NOT acceptable.
5. **Call out breaking changes explicitly** in the Details section if any public API, CLI flag, report format, or spec ID changed.
6. **Create the PR** with `gh pr create --base main --title "<title>" --body-file <path>` (write the filled template to a temp file and pass it via `--body-file` to preserve formatting).

## Rules

- **Never create a PR if `make ci` fails.** Coverage below threshold counts as failure.
- **No git commands beyond `git diff main...HEAD`.** No `add`, `commit`, `push`, `checkout`, `merge`, `rebase` — CI handles git per [CLAUDE.md](CLAUDE.md).
- PR description must be specific and tight — no vague placeholders, no "various improvements".
- Reference spec IDs (`[GROUP-TOPIC]` / `[GROUP-TOPIC-DETAIL]`) whenever the change maps to a spec section, so reviewers can `grep` spec → code → tests.
- Link any related GitHub issue (`Closes #N`) if one exists.
- Base branch is always `main`.

## Success criteria

- `make ci` passed locally (including coverage threshold).
- Diff against `main` was read and summarised into the template's three sections.
- PR created with `gh pr create` against `main`.
- PR URL returned to the user.
