# Pre-merge regression and test-weakening review: `worktree-fused-score-followups`

## Verdict

Do not merge yet. The current branch has three branch-specific regressions:

1. the composite action interpolates a GitHub context directly into a shell body;
2. the polymorphic-signature filter aliases same-shaped, unrelated implementations;
3. literal-echo scoring treats arbitrary identifier substrings inside data as rename proof.

Each defect also has a focused test blind spot that currently permits a green result.
This report is deliberately limited to defects introduced or materially changed by
this branch. It supersedes the previous review rather than carrying stale findings
forward.

No corpus suite or full CI run was used for this reaudit.

## Merge blockers

### P0 — The new action guard violates its own shell-injection boundary

Evidence:

- `action.yml:143-145` says inputs and contexts must reach scripts through `env`,
  never through `${{ }}` inside a `run` body.
- The new error path at `action.yml:164-169` directly embeds
  `${{ github.base_ref }}` in a Bash double-quoted string.
- GitHub's CodeQL review has an open code-injection finding on `action.yml:168`.

The backslash does not protect this expression. GitHub expands `${{ ... }}` before
Bash parses the step. With an ordinary `main` base, Bash preserves the backslash
before `m`, so the suggested command contains the invalid ref `origin/\main`.
With shell-significant text in the context, the backslash protects only the first
ordinary character and does not prevent later shell expansion.

This is branch-introduced: the vulnerable line was added with the new
`diff == '-'` rejection step.

Required patch:

- Use a static safe example such as `origin/main`, or pass `github.base_ref`
  through an environment variable and print it as data with `printf`.
- Keep every `${{ ... }}` expression outside `run` scalar bodies.

Required regression pin:

- Parse every composite-action `run` body and fail if it contains `${{`.
- Assert the exact error output contains a usable command and no stray backslash.

Current test weakness:

`scripts/actions/action-contract-shape-checks.mjs:213-237` checks that the guard
exists, precedes download, and exits with status 2. It never validates the shell
trust boundary or even the emitted guidance. The targeted action-contract run
reported all 41 checks passing while this CodeQL finding remained present.

### P1 — The polymorphic comparator aliases unrelated backend implementations

Evidence:

- `crates/deslop-core/src/cluster_filters/body_shape.rs:28-33` explicitly erases
  identifier and literal text and keeps only framed node kinds.
- `crates/deslop-core/src/cluster_filters/polymorphic.rs:49-64` uses that stream as
  the sole test for whether same-named implementations have different bodies.

These bodies produce the same stream:

```python
async def tool_call(self, job):
    return await self.container.run(job)

async def tool_call(self, job):
    return await self.machine.launch(job)
```

The collaborator and callee names are the entire behavioral distinction, but both
are erased. `subject_bodies_differ` therefore returns false, the polymorphic filter
does not suppress the pair, and mandatory interface implementations can surface as
an actionable clone.

The branch changed this decision from raw body bytes to a normalized kind stream to
stop consistent Type-2 renames being hidden. That fixed the rename false negative
by introducing the opposite false positive.

Required patch:

- Make the comparison role-aware: local/parameter renames may normalize, while
  collaborator, member, and callee substitutions remain substantive.
- Prefer positive contract evidence, such as a shared base/interface method, over
  treating every same-named cross-file function as polymorphic.

Required regression pin:

- In one fixture and one scan, include two same-named, same-shaped implementations
  that call different backends and a consistently renamed helper clone.
- Assert the backend pair is absent and the renamed helper is present with exact
  files, ranges, bucket, signals, occurrence count, and positive duplicated LOC.

Current test weakness:

`crates/deslop/tests/python_issue_69_abstract_method.rs:16-54` is an absence-only
test. Its Docker and Fly bodies already differ structurally, so it never exercises
same-shaped backend substitutions. It also accepts a completely blind detector:
an empty report makes every loop assertion vacuous and satisfies
`cluster_count == 0`.

### P1 — Literal echoes accept arbitrary substring replacement as rename proof

Evidence:

- `crates/deslop-core/src/content/rename.rs:250-279` lets a changed literal
  corroborate an identifier substitution.
- `crates/deslop-core/src/content/rename.rs:335-372` replaces every raw byte
  occurrence with no identifier-boundary or symbol-role check.
- `crates/deslop-core/src/content/rename.rs:242-247` upgrades contradiction-free
  evidence to full rename consistency once the anchor mass reaches the support
  floor.

For an elected `a -> x` identifier substitution, the implementation accepts
`"banana" -> "bxnxnx"` as a literal echo. The `a` bytes are ordinary data, not a
symbol reference. Nine repeated identifier positions plus this false echo produce
ten anchors, clear the `anchors / (anchors + 4) >= 0.7` certification boundary,
and can render `rename_consistency = 1.0` and an act-now confidence for code whose
literal data changed.

Required patch:

- Recognize echoes only where the substituted bytes occupy an identifier-like
  boundary in the decoded literal payload. Do not count a match inside a longer
  word or arbitrary data token.
- Preserve intended cases such as `"OrderService" -> "UserService"` and symbol
  names embedded at real boundaries in paths or messages.

Required regression pin:

- Add a negative `a -> x`, `"banana" -> "bxnxnx"` fixture and assert the literal
  remains a contradiction, rename consistency stays below certification, and the
  cluster does not enter an act-now bucket.
- Keep the existing full-symbol echo as the positive control in the same focused
  test surface.

Current test weakness:

`crates/deslop/tests/rename_literal_monotonicity.rs:104-133` asserts only
`thorough >= sloppy`. An implementation returning `1.0` for both passes. The suite
contains no negative echo whose matching bytes are merely a substring of ordinary
literal data.

## Findings retired from the previous report

The previous report no longer describes the current branch accurately:

- behavior-bearing operators now participate in normalized/content evidence;
- exact duplicate subgroups are split before cluster-wide noise suppression;
- majority verbatim families no longer force whole-cluster agreement to `1.0`;
- live cluster equality now compares the full generated payload.

Inherited call/cell/cache concerns were excluded from this branch-only review, and
the corpus-test audit was removed from scope as requested.

## Required order before merge

1. Remove the direct GitHub-context interpolation and add the action trust-boundary
   contract test.
2. Fix literal-echo boundary handling and pin both the false echo and intended
   full-symbol echo.
3. Replace the polymorphic kind-only decision with a role/contract-aware one and
   add the same-run positive and negative controls.
4. Re-run only the focused action, rename-evidence, and polymorphic regression
   tests needed to prove these fixes before relying on broader CI.

## Focused verification performed

- `node scripts/actions/test-action-contract.mjs` — passed 41/41, demonstrating
  the action test blind spot.
- Static branch-vs-main inspection of the three production paths and their focused
  regression tests.
- No corpus tests and no full CI run.
