# Adopting the lspkit memory-leak fixes

## [LSPKIT-ADOPT-SCOPE] What this plan covers

The sibling `lspkit` repository proved five ownership leaks on 2026-11-16 and is repairing them (its `docs/plans/memory-leak-audit.md` and `docs/plans/memory-leak-fixes.md`). The fixes shipped in lspkit v0.5.0 (crates.io, 2026-11-16).

`deslop-lsp` now runs on `lspkit-server`, at the exact release pinned in `Cargo.toml`, in place of `tower-lsp` ([LSP-TRANSPORT-DISPATCH]); the MCP, watcher and scheduler scaffolding is still Deslop's own; [the repository index](../repo-index.md) maps each piece to the lspkit crate meant to replace it. So the fixes reach Deslop in two ways, and this plan does both:

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

- [x] lspkit agent confirms the release version that carries all five repairs, where it is published (crates.io or a git tag), and the date: v0.5.0, crates.io, 2026-11-16.
- [ ] For each row above, a Deslop test that proves the allocation is released when the work ends (weak-count or live-entry assertions, no RSS, no sleeps). A red test is a confirmed Deslop leak: fix it under the fix-bug skill, failing test committed first.
- [x] `FileRegistry` growth: failing test, then removal on delete and rename ([LIVE-SCHEDULER-REGISTRY-BOUNDED], `tests/live/file_handles.rs`).
- [x] The dependency change is its own pull request, opened after the current regression fixes are released (v0.43.0).
- [x] After the lspkit release exists: add `lspkit-server` at that exact version to `Cargo.toml` (never a branch, path or unreleased revision): `lspkit-server = "=0.5.0"`, which brings `lspkit` 0.5.0 with it. Neither CI nor the devcontainer lists crate versions.
- [ ] Replace Deslop's scaffolding piece by piece in the order of the migration table in the repository index, starting with the pieces that had a leak class above. Each replacement deletes the Deslop code it replaces in the same change; no parallel path.
- [ ] Re-run every leak test from step 2 against the lspkit-backed code; they must stay green.
- [ ] `scripts/reports/edit_cost.py` and `scripts/reports/idle_churn.py` before and after each replacement; reports under `reports/`. Resident memory and CPU per edit must not rise.
- [ ] `scripts/compare-versions.sh <last release> HEAD` after the last replacement: the scaffolding swap must move no judged pair.
- [x] Any defect found in lspkit during adoption is logged on the lspkit repository and raised with its agent over Too Many Cooks; Deslop does not work around it. Logged: lspkit has no handle for requests the server sends to the editor, its sidecar correlator cannot be resolved from an id read off the wire, and its JSON-RPC error has no `Display`. Until that ships, `deslop-lsp/src/client/pending.rs` holds the waiting-request table, with the leak tests below.

## [LSPKIT-ADOPT-GATES] What blocks adoption

- No lspkit release containing the repairs: step 2 can proceed, the dependency change cannot.
- A lspkit leak test still red in the released version.
- A Deslop leak test red against the lspkit-backed code.

## [LSPKIT-ADOPT-STATUS] Status

2026-11-16: plan written; lspkit v0.5.0 published the same day.

2026-11-16: first replacement landed. `deslop-lsp` dropped `tower-lsp` and `tower` for `lspkit-server` 0.5.0: framing, request dispatch and cancellation are lspkit's, so its repaired dispatcher (a rejected request releases its registration) is the code Deslop runs. `lsp-types` moved from 0.94 to 0.97 with it, so one version is compiled.

2026-11-19: pin moved to `=0.10.1`. Its reader discards an oversized body instead of reading it as header text, so the session skips that frame and keeps serving ([JSONRPC-FRAME-LIMIT]); `server::tests::a_refused_oversized_frame_leaves_the_session_serving_the_next_request` is red on 0.10.0 and green on 0.10.1.

| lspkit finding | Deslop state after this change | Proof |
|---|---|---|
| Abandoned requests keep their channel | A request sent to the editor is forgotten when its wait is dropped | `client::tests::an_abandoned_request_leaves_nothing_behind` |
| Detached task owns state after its owner is gone | The serve loop aborts its writer, its notification worker and every request in flight when it ends | `server::tests::a_closed_stream_ends_the_session_while_work_is_still_running` |
| Rejected request keeps its registration | lspkit 0.5.0 dispatcher; an unknown method answers `-32601` | `server::tests::an_unknown_request_is_method_not_found` |
| `didClose` keeps the buffer | Unchanged: `close_buffer` releases it | `tests/open_buffers.rs` |
| Handlers capture their own dispatcher | Routes hold the backend, the session holds the routes, nothing holds the session | `server::tests::dropping_the_session_releases_the_backend` |

Still Deslop's own, in migration-table order: the live watcher and scheduler, the MCP server and tool registry, the config loader, tracing setup.
