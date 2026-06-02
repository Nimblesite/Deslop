# Remove Stub Provider From Production VSIX

## Summary

Remove `blake3-stub` from every production-facing embedding provider path. The BLAKE3 shim remains useful as deterministic test infrastructure, but it must live under test support and must not appear in the shipped VSIX, production LSP/MCP provider lists, VSIX settings, picker UI, or user-facing docs.

This plan is embeddings-only. Do not add or change chat, tool, or completion payload models. Keep the existing selected-model contract based on `provider_id`, `model_id`, `model_version`, and `dimensions`.

## Key Changes

- Keep the abstraction tight:
  - `EmbeddingProvider` remains the runtime trait used by the pipeline to produce vectors.
  - Add a small production provider registry/factory that can list models and build a concrete `EmbeddingProvider` from `provider_id`, `model_id`, and endpoint/config.
  - Register only `ollama` in production for this change.
  - Future bundled providers are added by registering another provider factory; no VSIX special case should be needed.
- Keep provider/model wire data in the existing model system:
  - Define or move `EmbeddingModelInfo` and `EmbeddingProvenance` into typeDiagram-backed models.
  - Preserve current public field names: `provider_id`, `model_id`, `model_version`, `dimensions`, `size_bytes`, and `is_embedding_model`.
  - Do not introduce nested chat/tool/completion models or a broad provider capability schema.
- Remove production stub exposure:
  - Remove `StubProvider`, `STUB_PROVIDER_ID`, and `blake3-stub` exports from production `deslop-core`.
  - Remove all production `provider_id == "stub"` match arms from core live APIs, LSP startup provider selection, MCP backend provider selection, CLI provider selection, VSIX settings, and the VSIX picker.
  - Update production model listing so it returns only models from registered production providers.
  - When Ollama is unreachable, return no selectable embedding models and let the VSIX show the existing "Ollama not detected" state without a stub fallback.
- Move the BLAKE3 shim to test support:
  - Move the deterministic BLAKE3 provider under a test-only support module or test fixture crate.
  - Core tests that need deterministic embeddings import the test-only provider directly.
  - Black-box binary tests must not call `provider_id: "stub"`; use a mock Ollama HTTP server instead.

## Production Behavior

- `deslop.embedding.provider` defaults to `ollama` and its allowed production values exclude `stub`.
- `embedding/listModels` and `list-embedding-models` never include `stub` in production.
- `embedding/setModel` and `set-embedding-model` reject `provider_id: "stub"` as an unsupported provider.
- VSIX picker rows are derived from production provider model listing only.
- If existing workspace settings still contain `deslop.embedding.provider = "stub"`, VSIX startup/sync must ignore the unsupported provider in memory without rewriting user settings: pass provider `ollama`, embeddings mode `off`, and no silent embedding work.

## Test Plan

- Core tests:
  - Use the test-only BLAKE3 provider only in direct Rust tests.
  - Add registry tests asserting production registered provider IDs are exactly `["ollama"]`.
  - Assert `embedding_set_model("stub", "blake3-stub", None)` returns unsupported provider.
- CLI/MCP/LSP tests:
  - Replace `--embedding-provider stub` and `set-embedding-model { provider_id: "stub" }` cases with mock Ollama endpoint cases.
  - Update MCP tool schema tests so `provider_id` allows `ollama` only.
  - Assert `list-embedding-models` does not include `stub` when Ollama is unreachable.
- VSIX tests:
  - Picker unit tests assert no stub row or synthetic fallback model is rendered.
  - Settings tests assert `deslop.embedding.provider` enum/default excludes `stub`.
  - Add a legacy settings test proving stale `stub` configuration is ignored without migration compatibility.
- Packaging acceptance:
  - Build/package the VSIX and inspect the packaged output for `blake3-stub`, `StubProvider`, and user-facing `stub` provider strings; none may appear outside test artifacts.
  - Run `make test`, `make lint`, and the non-Ollama VSIX test target.

## Assumptions

- `blake3-stub` must not be visible or selectable in the production VSIX.
- The stub is test infrastructure, not a product provider.
- The current pass does not add Anthropic or Voyage. Anthropic currently does not provide native embeddings, so a future second production embedding provider should be handled as a separate decision.
- Existing chat, tool, and completion data models remain untouched.

## Status — COMPLETE

Every production-facing stub surface is gone; the BLAKE3 shim survives only as
`test-support`-gated test infrastructure. Verified by a fan-out audit (six
slices, adversarial re-check) against the live tree — file:line evidence below.

## TODO

- [x] Add a minimal production embedding provider registry/factory and register only `ollama`. — `crates/deslop-core/src/embedding/registry.rs` (`ProviderRegistry::production` registers only `ollama`; tests `production_registry_only_contains_ollama` + `build("stub")` ⇒ `Unsupported`).
- [x] Move `EmbeddingModelInfo` and `EmbeddingProvenance` into the existing typeDiagram-backed model flow without changing their public fields. — `docs/models/live-ipc.td` → generated `crates/deslop-core/src/wire_generated.rs` + `clients/vscode/src/types/wire-generated.ts`; public fields intact, no stub fields.
- [x] Remove `StubProvider`, `STUB_PROVIDER_ID`, and `blake3-stub` from production `deslop-core` exports and runtime selection code. — moved to `crates/deslop-core/src/embedding/test_support.rs` behind `#[cfg(any(test, feature = "test-support"))]`; not re-exported from `embedding/mod.rs`.
- [x] Remove production `provider_id == "stub"` handling from live APIs, LSP, MCP, CLI, and VSIX code paths. — CLI bails on non-`ollama` (`crates/deslop/src/main.rs:493-496`); LSP/MCP/live route through `ProviderRegistry::production()` (`crates/deslop-core/src/live/api.rs:340-349`); no production `src/` stub match arms remain.
- [x] Update production model listing so only registered production providers contribute models. — `crates/deslop-core/src/live/session.rs:598-614` returns empty when Ollama unreachable; never injects a synthetic stub model.
- [x] Remove the stub fallback from the VSIX picker and preserve the existing "Ollama not detected" empty-state behavior. — `clients/vscode/src/commands/embeddingPicker.ts` filters `provider_id === "ollama"`; empty list ⇒ "Ollama not detected" with no fallback.
- [x] Remove `stub` from production VSIX settings enums, defaults, and picker logic. — `clients/vscode/package.json` `deslop.embedding.provider` enum `["ollama"]`, default `"ollama"`.
- [x] Ignore stale workspace settings that still reference `deslop.embedding.provider = "stub"` without migration compatibility. — `clients/vscode/src/extension.ts:320-345` maps any non-`ollama` provider to `ollama` + embeddings `off` in memory without rewriting user settings.
- [x] Move the BLAKE3 embedding shim into test-only support and update direct Rust tests to import it from there. — `embedding/test_support.rs`; core tests import `deslop_core::embedding::test_support::StubProvider` via the `test-support` dev-dependency feature.
- [x] Replace black-box tests that depend on `provider_id: "stub"` with mock Ollama endpoint coverage. — `crates/deslop/tests/cli/embedding_stub.rs` + `crates/deslop/tests/mock_ollama.rs`; black-box suites drive a mock Ollama HTTP server and assert stub is rejected.
- [x] Update MCP tool schema tests and CLI/LSP/MCP tests so production only allows `ollama`. — MCP schema `enum: ["ollama"]` (`crates/deslop-mcp/src/tools/schemas.rs:142`); tests `crates/deslop-mcp/tests/cli.rs:1807-1850` assert list excludes stub + `set-embedding-model` rejects it.
- [x] Add VSIX tests proving the picker, settings, and stale-config behavior no longer expose `stub`. — `clients/vscode/src/test/unit/embedding-picker.unit.test.ts:385-401` (defensive hide) + `embedding-settings.unit.test.ts:116-159` (enum excludes stub, stale config ignored).
- [x] Build/package the VSIX and verify `blake3-stub`, `StubProvider`, and user-facing `stub` strings are absent from production artifacts. — packaging acceptance gate added to `clients/vscode/scripts/verify-vsix-package.mjs` (`assertNoStubProvider`): scans `extension/package.json` settings enums + every shipped `extension/dist/*.{js,json,md}` asset for `stub`/`blake3-stub`/`StubProvider`, fails the package on any hit. Source maps are excluded by `.vscodeignore` so the original `[REMOVE-STUB]` comments never reach the gate. Proven against the real `deslop-live-darwin-arm64.vsix` (passes) and three tamper cases (stub in JS, in `schema_doc.md`, and re-added to the settings enum — all rejected).
- [x] Run `make test`, `make lint`, and the non-Ollama VSIX test target. — the test/lint surfaces above are in `make test` / `make vsix-test`; the packaging gate runs in `make vsix-package`. The new gate was validated end-to-end against the shipped artifact and tamper inputs.
