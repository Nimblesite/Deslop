# Pre-merge regression and test-weakening reaudit: `worktree-fused-score-followups`

## Verdict

Do not merge yet.

The action injection defect is fixed and pinned. The literal-substring defect is
fixed in production, but its new fixture is not connected to any test. The
polymorphic rewrite fixes the demonstrated Python backend pair, but its new
"declared contract" heuristic is not method-level proof and cannot represent
implicit interface implementations. That leaves one production regression and two
test gaps before this branch is merge-ready.

No corpus suite or full CI run was used for this reaudit.

## Remaining production regression

### P1 — The polymorphic fix mistakes inheritance for method-contract proof

Evidence:

- `crates/deslop-core/src/cluster_filters/polymorphic.rs:17-49` recognizes a
  hard-coded set of containing type/base-list shapes.
- `polymorphic.rs:62-92` suppresses a same-named cross-file cluster when every
  subject is merely somewhere under one of those declarations and the new body
  streams differ.
- `polymorphic.rs:108-136` proves only that an ancestor names some base, interface,
  or trait. It never proves that the subject method is declared by that contract.
- `crates/deslop-core/src/cluster_filters/body_shape.rs:12-34,61-96` now embeds raw
  collaborator/member/callee names in the body stream.

Any ordinary subclass now counts as a contract implementation. A copied helper in
two unrelated subclasses can therefore be hidden solely because the copies renamed
their collaborators:

```python
class InvoiceWorker(CommonBase):
    def synchronise(self, order):
        return self.order_repo.fetch(order.id)

class UserWorker(CommonBase):
    def synchronise(self, user):
        return self.user_store.load(user.key)
```

`CommonBase` need not declare `synchronise`. The two method bodies are a consistent
Type-2 rename, but the ancestor-base check returns true and the raw reach symbols
make the body streams differ, so the polymorphic filter suppresses the clone.

The opposite hole remains for implicit contracts. Go methods are declared outside
their receiver type and interface satisfaction is implicit, so walking lexical
ancestors can never establish `under_contract`. The new filter therefore cannot
suppress Go implementations even though
`crates/deslop-core/src/cluster_filters/mod.rs:478-481` explicitly routes Go
methods through the shared function-kind surface. The reach-symbol list also lacks
Go's `selector_expression`, so same-shaped Go backend calls still erase the
collaborator and callee names.

Required patch:

- Establish that the specific method implements a declared contract; the presence
  of any superclass is insufficient.
- Add a language-specific strategy for implicit contracts, or explicitly fail open
  without claiming the generic filter covers them.
- Keep collaborator-aware comparison scoped to proven contract implementations;
  do not apply it as evidence that an ordinary inherited method is polymorphic.

Required regression pins:

- Add a Python fixture with two ordinary subclasses of one base whose same-named
  methods are consistently renamed copies; assert the clone remains visible.
- Add a Go interface with two same-signature, different-backend implementations;
  assert the contract pair is absent.
- Keep both controls in scans that contain an exact positive clone, with exact
  paths, ranges, buckets, signals, files analysed, and duplicated LOC assertions.

## Test weakening and missing pins

### P1 — The Python backend test does not test the contract proof it relies on

`crates/deslop/tests/python_same_shape_backends.rs:68-153` is now a useful two-sided
test for one explicit Python `ABC` hierarchy: the backend pair must disappear and a
renamed free-function clone must remain. It does not cover the heuristic boundary:

- a subclass whose base does not declare the subject method;
- a consistently renamed method inside ordinary subclasses;
- a language with implicit interface satisfaction.

The positive control is a free function, so `under_contract == false` protects it
before the new collaborator-aware comparison is tested. The suite can pass while
the false-negative subclass case above remains live.

### P1 — The literal-boundary fix has an orphaned fixture, not a regression test

The production replacement is corrected:

- `crates/deslop-core/src/content/rename.rs:335-392` now replaces an identifier only
  at symbol boundaries, so `id -> key` no longer turns `"invalid request"` into
  `"invalkey request"` and counts it as rename proof.

The new fixture exists under
`crates/deslop/tests/fixtures/ts-rename-literal-substring/`, but no test currently
scans it. `crates/deslop/tests/rename_literal_monotonicity.rs:104-133` is unchanged
and still asserts only `thorough >= sloppy`; returning `1.0` for both remains green.
Deleting the boundary check would therefore leave the focused rename suite passing.

Required regression pin:

- Scan `ts-rename-literal-substring` and assert the changed data literal remains a
  contradiction, rename consistency stays below certification, and the cluster
  does not enter an act-now bucket.
- In the same test surface, retain the intended full-symbol
  `"OrderService" -> "UserService"` echo as a positive control.
- Assert a strict separation between the intended echo and substring collision;
  `>=` alone is not a regression oracle.

## Fixes verified and retired

### GitHub Action shell injection — fixed

- `action.yml:168-171` passes `github.base_ref` through `env` and prints it with a
  `%s` `printf` argument.
- `scripts/actions/action-contract-shape-checks.mjs:247-269` extracts every `run`
  body, asserts the expected body count, and rejects `${{` interpolation inside
  any body.
- `node scripts/actions/test-action-contract.mjs` passed all 42 checks.

### Literal substring replacement — production fix present

The symbol-boundary implementation at `content/rename.rs:335-392` addresses the
reported byte-substring defect. It remains listed above only because the regression
pin is missing.

### Same-shaped Python backends — production fix present

The new body stream carries Python collaborator/callee identity and the new test
models the exact Docker/Fly shape. The remaining blocker is the broader contract
heuristic introduced to apply that decision.

## Focused verification performed

- `node scripts/actions/test-action-contract.mjs` — 42/42 passed.
- Before the final polymorphic production edit landed, the new
  `python_same_shape_backends` test correctly failed and reported the exact
  `docker_host.py L1-15` / `fly_host.py L1-15` `nearly_identical` cluster.
- The worktree changed during the audit. The post-edit Rust rerun did not complete
  because other cargo processes held the shared build lock; the current findings
  above are from the final source inspection.
- No corpus tests and no full CI run.
