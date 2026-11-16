# Adopting the lspkit memory-leak fixes

## [LSPKIT-ADOPT-SCOPE] What this plan covers

The sibling `lspkit` repository proved five ownership leaks on 2026-11-16 and is repairing them (its `docs/plans/memory-leak-audit.md` and `docs/plans/memory-leak-fixes.md`). The fixes are not released: the latest lspkit release is v0.4.0 and the repairs sit on a work-in-progress branch.

Deslop does not depend on lspkit today. `deslop-lsp` is built on `tower-lsp` 0.20.0 and the MCP, watcher and scheduler scaffolding is Deslop's own; [the repository index](../repo-index.md) maps each piece to the lspkit crate meant to replace it. So the fixes reach Deslop in two ways, and this plan does both:

1. Each leak class lspkit proved is checked against Deslop's own scaffolding now, test-first, because the same shape of code exists here.
2. Once lspkit publishes a release that contains the repairs, Deslop adopts the lspkit crates for those pieces, pinned to that release.

Analysis code (parse, fingerprint, cluster, rank, embeddings) is not part of this plan and stays in Deslop.

## [LSPKIT-ADOPT-LEAKS] The five leaks and their counterpart in Deslop

| lspkit finding | Deslop counterpart to test |
|---|---|
| Abandoned sidecar requests keep their channel allocations (`sidecar/correlator.rs`) | MCP-to-LSP IPC requests that time out or are cancelled (`deslop-mcp/src/server.rs`, `deslop-core/src/live/transport.rs`) |
| A dropped inbound reader leaves a detached task owning decoder state (`sidecar/inbound.rs`) | Detached `tokio::spawn` tasks: `live/scheduler.rs`, `live/api.rs`, `deslop-lsp/src/backend.rs`, `diagnostic_session.rs`, `file_watch.rs`, `cache_seed.rs` |
| A terminal preparation error keeps its ingress registration (`lspkit-server/src/dispatch.rs`) | Request cancellation and in-flight bookkeeping in `deslop-lsp` (tower-lsp owns dispatch; Deslop's own maps are the target) |
| The reference server ignores `didClose` and keeps closed buffers | `deslop-lsp/src/backend.rs` `did_close` and the open-buffer model in `live/session_buffers.rs` |
| Reference handlers capture their own dispatcher (an `Arc` cycle) | Closures registered on `LspBackend` and the MCP tool registry that capture the service they belong to |

A first read of Deslop's code (2026-11-16, no test yet) found that `did_close` releases its buffer and that Deslop has no sidecar correlator or inbound reader of its own; the tests in the checklist turn that reading into proof.

Related growth already found in Deslop and tracked with this work: `FileRegistry` never forgets a path, so every path ever admitted stays registered for the life of the process.

## [LSPKIT-ADOPT-CHECKLIST] Steps

- [ ] lspkit agent confirms the release version that carries all five repairs, where it is published (crates.io or a git tag), and the date.
- [ ] For each row above, a Deslop test that proves the allocation is released when the work ends (weak-count or live-entry assertions, no RSS, no sleeps). A red test is a confirmed Deslop leak: fix it under the fix-bug skill, failing test committed first.
- [x] `FileRegistry` growth: failing test, then removal on delete and rename ([LIVE-SCHEDULER-REGISTRY-BOUNDED], `tests/live/file_handles.rs`).
- [ ] The dependency change is its own pull request, opened after the current regression fixes are released.
- [ ] After the lspkit release exists: add `lspkit` and `lspkit-server` at that exact version to `Cargo.toml` (never a branch, path or unreleased revision), with the same pin in `.github/workflows/ci.yml` and `.devcontainer/` where versions are listed.
- [ ] Replace Deslop's scaffolding piece by piece in the order of the migration table in the repository index, starting with the pieces that had a leak class above. Each replacement deletes the Deslop code it replaces in the same change; no parallel path.
- [ ] Re-run every leak test from step 2 against the lspkit-backed code; they must stay green.
- [ ] `scripts/reports/edit_cost.py` and `scripts/reports/idle_churn.py` before and after each replacement; reports under `reports/`. Resident memory and CPU per edit must not rise.
- [ ] `scripts/compare-versions.sh <last release> HEAD` after the last replacement: the scaffolding swap must move no judged pair.
- [ ] Any defect found in lspkit during adoption is logged on the lspkit repository and raised with its agent over Too Many Cooks; Deslop does not work around it.

## [LSPKIT-ADOPT-GATES] What blocks adoption

- No lspkit release containing the repairs: step 2 can proceed, the dependency change cannot.
- A lspkit leak test still red in the released version.
- A Deslop leak test red against the lspkit-backed code.

## [LSPKIT-ADOPT-STATUS] Status

2026-11-16: plan written; release version and date requested from the lspkit agent over Too Many Cooks. Nothing adopted yet.
