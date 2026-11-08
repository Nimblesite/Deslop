# Osprey grammar pin onto Osprey main

## [LANG-CAND-OSPREY-RECOVERY] Pin provenance

Deslop's `tree-sitter-osprey` and `tree-sitter-osprey-ml` pins in `Cargo.toml` point at `e82980cf1`, the fix for Nimblesite/osprey#374. That commit is only on Osprey's `qwen-prefill` branch, 74 commits ahead of main. Its route to main is Osprey PR #373, an unrelated, draft Qwen-prefill PR with red CI (`Rust workspace (rest)`, `Osprey Agent UI`).

**Why this blocks:** a pin to a commit on an unmerged branch is not durable. A rebase or force-push of `qwen-prefill` orphans the revision, and once GitHub collects it, `cargo fetch` fails for every Deslop build and CI run. The grammar fix itself has no dependency on the Qwen work, so it should not wait for it.

**Rule:** per `AGENTS.md`, Deslop adds no workaround. The fix lands in Osprey, and Deslop only bumps the pin.

## Steps

- [x] **Ask Osprey for a grammar-only PR** (sent 2026-11-08). Message OspreyClaude2 over Too Many Cooks: cherry-pick `e82980cf1` onto a fresh branch from Osprey main, open its own PR, and merge it once that PR's CI is green. #373 then rebases onto it unchanged.
- [ ] **Confirm on GitHub, not on a message.** Check that the cherry-picked commit is reachable from main (`gh api repos/Nimblesite/osprey/compare/main...<rev>` reports `behind` or `identical`) and that #374 is closed.
- [ ] **Bump the pin** to the commit on main, in both `Cargo.toml` lines, then run `cargo update -p tree-sitter-osprey -p tree-sitter-osprey-ml`. Keep `docs/plans/osprey-support.md` and `reports/tests/report-reuse-baseline-blockers.md` in sync with the new revision.
- [ ] **Run gates:** `cargo test -p deslop --test suite osprey_recovery`, then `make test`. A cherry-pick changes the commit hash but not the grammar. If either gate goes red, the cherry-pick lost something, and the problem goes back to Osprey.
- [ ] **Commit** with the message `deps: pin Osprey grammars to main <rev> [LANG-CAND-OSPREY-RECOVERY]`, then push to the PR #603 branch.
- [ ] **Merge Deslop PR #603 only after the pin is on Osprey main.** Until then, #603 stays open.

## Fallback

If Osprey declines a separate PR, #603 waits for #373 to merge. Deslop does not vendor, patch or fork the grammar.
