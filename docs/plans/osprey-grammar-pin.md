# Osprey grammar pin onto Osprey main

## [LANG-CAND-OSPREY-RECOVERY] Pin provenance

Deslop pins `tree-sitter-osprey` and `tree-sitter-osprey-ml` (one repository, one revision) in `Cargo.toml`.

| Where | Pin | Reachable from Osprey main? |
| --- | --- | --- |
| Current Deslop checkout | `093f04242c7d8b2f56d8d5051dd813b9e9d280f3` | Yes. GitHub compare reports `identical` on 2026-11-10; both grammar directories are identical to `e82980cf1`. |

A pin that is not on main is fragile. If someone force-pushes the branch, the commit becomes orphaned and `cargo fetch` eventually fails. PR #603 merged as `6b66bd489bdafe51a37f76a66c09312ee4c98e18`, adding the upstream fix that turns `osprey_recovery` green; the previous pin still had an ancestry risk, now resolved by pinning the identical grammar trees on upstream main. This is no workaround: Deslop does not vendor, patch or fork the grammar ([AGENTS.md](../../AGENTS.md)).

`e82980cf1` touches only the hand-written `tree-sitter-osprey-ml/src/scanner.c` and `bindings/rust/tests.rs`. A cherry-pick onto Osprey main should not conflict with generated files.

## Steps

- [x] Ask OspreyClaude2 over Too Many Cooks for a grammar-only PR to Osprey main: cherry-pick `e82980cf1`, and if a generated file ever conflicts, regenerate it with `tree-sitter generate` rather than hand-merging (sent 2026-11-08).
- [x] Have the plan reviewed (pi `zai/glm-5.3`, two rounds, 2026-11-08). Accepted: main is already fragile, so merge #603 now; re-pin promptly; set an escalation deadline; widen the doc-sync gate; check the lockfile scope. Rejected: holding #603 to keep a test red. The test is green on the genuine upstream fix.
- [x] Merge Deslop PR #603. GitHub confirms merge commit `6b66bd489bdafe51a37f76a66c09312ee4c98e18`.
- [x] Confirm on GitHub, not from a message: `gh api repos/Nimblesite/osprey/compare/main...<rev>` reports `behind` or `identical`, and #374 is closed.
- [x] Within one session of that Osprey merge, re-pin both `Cargo.toml` lines to the main commit and run `cargo update -p tree-sitter-osprey -p tree-sitter-osprey-ml`. The `Cargo.lock` diff touches only those two packages.
- [x] Gate: `Cargo.toml` and `Cargo.lock` contain neither `865f0469` nor `e82980cf1`. `docs/plans/osprey-support.md`, `docs/plans/osprey-recovery-and-compare.md` and `reports/tests/report-reuse-baseline-blockers.md` name the new revision.
- [x] Run `cargo test --release -p deslop --test suite osprey_recovery`: the unchanged recovery test passes on the new pin.
- [ ] Run `make test` and remaining CI gates; the earlier green PR run does not cover this new pin.
- [x] Commit as `deps: pin Osprey grammars to main <rev> [LANG-CAND-OSPREY-RECOVERY]`.
- [ ] Before any Deslop release or tag, check that the pinned revision is reachable: `gh api repos/Nimblesite/osprey/commits/<rev>`.

## Escalation

If there is no grammar-only merge to Osprey main by **2026-11-10**, comment on Nimblesite/osprey#374 asking for it and message OspreyClaude2 again. Deslop never vendors, patches or forks the grammar.

2026-11-10: OspreyCodexCIPrep supplied main `093f04242c7d8b2f56d8d5051dd813b9e9d280f3`. GitHub independently confirms `identical`; both complete grammar directories have no diff against the previous pin. Cargo updated only the two grammar packages. Test and release gates below remain required.
