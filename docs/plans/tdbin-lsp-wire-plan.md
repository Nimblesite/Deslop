# tdbin across the LSP ↔ client boundary

Spec: [lsp.md §LSP-CUSTOM-METHODS](../specs/lsp.md), [§LSP-WIRE-BUDGET](../specs/lsp.md#lsp-wire-budget). New spec section: `[LSP-WIRE-TDBIN]`. Format: [tdbin wire format](https://typediagram.dev/docs/tdbin-wire-format.html).

> **Every `deslop/*` payload that crosses between `deslop-lsp` and an editor client is one framed tdbin message generated from `docs/models/live-ipc.td`. No JSON payload body, no hand-written codec, no fallback.**

## Why

- `deslop/reportChanged` and `deslop/reportGet` carry the whole live report on every analysis pass. JSON is the reason `[LSP-WIRE-BUDGET]` exists: occurrences are capped at 100 so the wire stays in kilobytes.
- tdbin is schema-known, word-packed and verified on decode. The schema hash replaces string-matching on the producing version, and an unknown union variant becomes a typed error rather than a misread.
- The VSIX and its binaries ship lock-step ([VSIX-PACKAGING]), so no version negotiation and no dual-format period are needed. The cut is a single change.

## Decisions

| Topic | Decision |
|---|---|
| Envelope | The LSP base protocol stays JSON-RPC: `vscode-languageclient` and `lsp4ij` cannot carry raw bytes. Every `deslop/*` request param, response result and notification param becomes `{ "tdbin": "<base64 framed packed message>" }`. Standard LSP methods (code lens, inlay hints, diagnostics, `workspace/executeCommand`) are untouched. |
| Framing | Framed `TDB1` with `PACKED` + `HASH`. Each root type is decoded against its own generated compatibility-major hash, so a hash mismatch is the wire-mismatch error. |
| Scope | Every `deslop/*` method and notification in [LSP-CUSTOM-METHODS]. Clients: VSIX **and** JetBrains, cut over together. |
| Out of scope | The MCP IPC socket (line-delimited JSON-RPC, [LIVE-IPC-SOCKET]); the extension-host → webview `postMessage` (in-process, structured clone); JSON reports on disk and on stdout (`live-report.json`, `--format json`), which are AI-facing output and not wire. These get their own plans after this one lands. |
| Rust types | One type per model. The serde derives stay because the JSON reports still need them, and the tdbin codec is an extra impl on the **same** generated type. A second tdbin ADT beside the serde one is duplication. |
| `[LSP-WIRE-BUDGET]` cap | Stays at 100 for this change. Lifting it changes what clients render, so it is a separate, measured decision taken from Phase 0 numbers. |
| `deslop/reportSchemaDoc`, `deslop/virtualDocument` | String results also travel as tdbin roots (`SchemaDoc`, `VirtualDocument`) so that one decode path covers every method. |

## What has to be done in typediagram (blocking)

Repo: `../typediagram`. Log each item on its GitHub repo and track the pin. Per [AGENTS.md], Deslop never works around these. Current status is from `docs/specs/tdbin-future-typescript.md` there.

- [ ] **TS codec release-conformant** (`[TDBIN-FUTURE-TS]`): generated codecs for every schema form Deslop uses, including records, tagged unions, enum-unions inlined as scalars, `List<record|union|enum|String|Int|Float>`, scalar and pointer `Option`, and nested types. Rust ↔ TS byte-identity fixtures both ways, with the same behaviour in Node and the browser.
- [ ] **Lossless `Int`** in TS (`bigint` or an equivalent) with i64 range checks. Deslop carries `u64` hashes and sizes (21 `u64` overrides today).
- [ ] **Generated compatibility-major hash per root**, required by the generated framed decode (TS and Rust). The caller-supplied-hash-only API is not enough.
- [ ] **Codec on the existing Rust type**: `--to rust` needs an option that emits the tdbin `encode`/`decode` impls for the same structs it already emits, with serde derives intact. It must not generate a separate ADT.
- [ ] **Width-refined numerics** (`[TDBIN-FUTURE-WIDTH]`: `U16`, `U32`, `U64`, `I32`, `U8`, plus `usize` mapping), **or** a supported per-field Rust type override that the codec respects. Today Deslop gets these through `fieldOverrides` in `scripts/typediagram/type-config-*.mjs` (52 `usize`, 21 `u64`, 4 `u32`, …). That post-processing belongs in typediagram, not in a Deslop script.
- [ ] **Path type** (or a documented `String` ↔ `PathBuf` mapping in Rust codegen) to replace the 14 `PathBuf` overrides.
- [ ] **Serde attribute emission from the `.td`** (`rename_all`, internally tagged `tag = "kind"`, `untagged`) so that `scripts/typediagram/rust-postprocess.mjs` and `ts-postprocess.mjs` can be deleted. The JSON reports keep needing these attributes.
- [ ] **Kotlin/JVM tdbin codegen + runtime** for the JetBrains client. Without it JetBrains cannot decode the wire, and a JSON path kept for JetBrains alone is legacy. This blocks the cut-over.
- [ ] **Base64 helpers** in the TS and Kotlin runtimes (`encodeFramedBase64` / `decodeFramedBase64`), so that no client hand-rolls the envelope.
- [ ] **Published versions**: an npm `typediagram` release with the TS runtime importable from the VSIX bundle (esbuild, tree-shakeable, size reported), a `tdbin` crate on crates.io or a git tag Deslop can pin, and the JVM artifact.

## Deslop checklist

### Phase 0 — Measure (before any cut)
- [ ] `[LSP-WIRE-TDBIN-MEASURE]`: a script under `scripts/` encodes captured `deslop/reportGet` and `deslop/reportChanged` payloads from the corpus register as JSON and as framed packed tdbin + base64. It records bytes and the client-side decode time for each, and the code writes the results mechanically to `docs/reports/tdbin-wire-<UTC>.md`.
- [ ] Gate: if base64 tdbin is not smaller **and** faster to decode than JSON on the largest corpus report, stop and report. The decision then moves to a binary side channel.

### Phase 1 — Model completeness
- [ ] Move every wire shape into `docs/models/live-ipc.td`. `EXTERNAL_TYPES` / `EXTERNAL_TS_TYPES` (`ReportCluster`, `EmbeddingProvenance`, `CacheStats` → `crate::report`) go away: hand-written wire types in `deslop-core::report` become generated.
- [ ] Add roots for every `deslop/*` param and result that is currently an ad-hoc JSON object (for example the `embeddingSetModel` params, `reportDelta` params, `pairCompare` `{ left, right }`, and string results).
- [ ] Delete `type-config-*.mjs` overrides and the post-processors as each typediagram item lands.

### Phase 2 — Server (`crates/deslop-lsp`)
- [ ] One generic `tdbin_params<T>` / `tdbin_result<T>` pair in `custom_methods.rs` that wraps every `deslop/*` handler and push in `file_watch.rs`. The handlers keep their typed signatures.
- [ ] A decode failure (bad base64, bad magic, hash mismatch, `UnknownVariant`) returns a typed JSON-RPC error that names the cause. It is never a panic and never a default.
- [ ] `tracing` at entry and exit: root type, byte count, schema hash.

### Phase 3 — VSIX (`clients/vscode`)
- [ ] One `deslopRequest<Req, Res>(method, value)` and one `onDeslopNotification<T>(method, handler)` built on the generated codecs. Every call site in the following files moves onto them, and no direct `sendRequest("deslop/…")` stays: `notifications.ts`, `extension.ts`, `extensionConfiguration.ts`, `bubble/live.ts`, `compare/title.ts`, `commands/embeddingPicker.ts`, `commands/register.ts`.
- [ ] `wire-generated.ts` exports the codecs. `types/report.ts` keeps only the UI intersection (`displayLocation`).
- [ ] Bundle-size delta recorded in the Phase 0 report.

### Phase 4 — JetBrains (`clients/jetbrains`)
- [ ] `DeslopLanguageClient.kt` and the report panel/renderer decode through generated Kotlin codecs. The Gson/JSON report parsing is deleted.

### Phase 5 — Specs, tests, docs
- [ ] `lsp.md`: add `[LSP-WIRE-TDBIN]` under [LSP-CUSTOM-METHODS] (envelope, framing, hash, error cases). Rewrite the `[LSP-WIRE-BUDGET]` rationale. In `vsix.md` and `site/src/docs/for-ai.md`, third-party LSP clients must now decode tdbin, and the docs say so and link the codecs.
- [ ] E2E (built binaries, real VSIX, no fake LSP) cites `[LSP-WIRE-TDBIN]` and covers:
  - an edit → `reportChanged` round-trip asserting the cluster ids, occurrence counts, paths and percentages the UI renders, for every fixture language;
  - each `deslop/*` method at least once;
  - a corrupted payload, a wrong-hash payload and an unknown variant, each asserting the named error.
- [ ] Rust ↔ TS ↔ Kotlin byte-identity test on a corpus report, so that all three codecs agree.
- [ ] Coverage thresholds unchanged or higher.

## Risks

- **Base64 overhead (+33%)** can cancel out the packing gain on small payloads. Phase 0 decides from measurements, not from expectation.
- **Third-party LSP clients** (Neovim, Helix, Zed) lose plain-JSON access to `deslop/*`. Lock-step versioning covers our clients only. This is accepted: the published codecs are their integration path.
- **TS `bigint`** changes every generated `Int` field in the VSIX and webview code. Count values (`occurrences_total`, mass) need explicit `Number()` at render time, done in one shared helper.
