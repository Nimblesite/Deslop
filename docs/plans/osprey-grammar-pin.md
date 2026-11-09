# Osprey grammar pin onto Osprey main

## [LANG-CAND-OSPREY-RECOVERY] Pin provenance

Deslop pins `tree-sitter-osprey` and `tree-sitter-osprey-ml` (one repository, one revision) in `Cargo.toml`.

| Where | Pin | Reachable from Osprey main? |
| --- | --- | --- |
| Current Deslop checkout | `e82980cf1`, the Nimblesite/osprey#374 fix | No. GitHub compare on 2026-11-10 reports `diverged`: 73 commits ahead, 3 behind Osprey main. |

A pin that is not on main is fragile. If someone force-pushes the branch, the commit becomes orphaned and `cargo fetch` eventually fails. PR #603 merged as `6b66bd489bdafe51a37f76a66c09312ee4c98e18`, adding the upstream fix that turns `osprey_recovery` green; the ancestry risk remains. This is no workaround: Deslop does not vendor, patch or fork the grammar ([AGENTS.md](../../AGENTS.md)).

`e82980cf1` touches only the hand-written `tree-sitter-osprey-ml/src/scanner.c` and `bindings/rust/tests.rs`. A cherry-pick onto Osprey main should not conflict with generated files.

## Steps

- [x] Ask OspreyClaude2 over Too Many Cooks for a grammar-only PR to Osprey main: cherry-pick `e82980cf1`, and if a generated file ever conflicts, regenerate it with `tree-sitter generate` rather than hand-merging (sent 2026-11-08).
- [x] Have the plan reviewed (pi `zai/glm-5.3`, two rounds, 2026-11-08). Accepted: main is already fragile, so merge #603 now; re-pin promptly; set an escalation deadline; widen the doc-sync gate; check the lockfile scope. Rejected: holding #603 to keep a test red. The test is green on the genuine upstream fix.
- [x] Merge Deslop PR #603. GitHub confirms merge commit `6b66bd489bdafe51a37f76a66c09312ee4c98e18`.
- [ ] Confirm on GitHub, not from a message: `gh api repos/Nimblesite/osprey/compare/main...<rev>` reports `behind` or `identical`, and #374 is closed.
- [ ] Within one session of that Osprey merge, re-pin both `Cargo.toml` lines to the main commit and run `cargo update -p tree-sitter-osprey -p tree-sitter-osprey-ml`. The `Cargo.lock` diff touches only those two packages.
- [ ] Gate: `Cargo.toml` and `Cargo.lock` contain neither `865f0469` nor `e82980cf1`. `docs/plans/osprey-support.md`, `docs/plans/osprey-recovery-and-compare.md` and `reports/tests/report-reuse-baseline-blockers.md` name the new revision.
- [ ] Run `cargo test -p deslop --test suite osprey_recovery`, then `make test`. A red gate means the cherry-pick lost something, which goes back to Osprey.
- [ ] Commit as `deps: pin Osprey grammars to main <rev> [LANG-CAND-OSPREY-RECOVERY]`.
- [ ] Before any Deslop release or tag, check that the pinned revision is reachable: `gh api repos/Nimblesite/osprey/commits/<rev>`.

## Escalation

If there is no grammar-only merge to Osprey main by **2026-11-10**, comment on Nimblesite/osprey#374 asking for it and message OspreyClaude2 again. Deslop never vendors, patches or forks the grammar.

2026-11-10: GitHub confirms #374 is closed, but the pinned revision still diverges from main. Requested the replacement main SHA from OspreyClaude2 and OspreyCodexCIPrep over TMC; closure alone does not satisfy the ancestry gate.
