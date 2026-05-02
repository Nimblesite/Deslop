# Plan — Migrate the JSON-RPC envelope into typeDiagram

## Context

The user's directive in `CLAUDE.md` is unambiguous:

> ALL MODELS TRANSFERRED ACROSS THE WIRE MUST USE typeDiagram. NO IFS. NO BUTS

In a prior session **I authored an exception in [protocol.rs](crates/deslop-mcp/src/protocol.rs) myself** that exempted the JSON-RPC 2.0 envelope (`RequestId`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcErrorResponse`, `JsonRpcError`, `ErrorCode`, plus the `JSONRPC_VERSION` const) from typeDiagram. The user did not approve this — they correctly challenged it.

The cited blockers (`serde(untagged)`, `skip_serializing_if`, numeric discriminants, `serde_json::Value` payloads) are real impedance points but **all addressable** by small extensions to the existing generator at [scripts/typediagram-gen.mjs](scripts/typediagram-gen.mjs). The intended outcome of this plan is to delete every hand-rolled wire struct in `protocol.rs` and re-export the generated equivalents — leaving only the constant `JSONRPC_VERSION` (a plain `&str`, not a wire model) and any pure constructor helpers.

The user explicitly asked for **small batches, not one big hit**. Every batch below ends with a green `cargo build --workspace --all-targets` + `cargo clippy --workspace --all-targets --no-deps` so that any single batch can land independently.

## Approach

Three generator extensions, then six migration batches. Generator extensions land first because they're prerequisite, and each one is independently testable on existing types (no envelope work yet).

### Generator extensions (Batches G1–G3)

**G1. `serdeAttrs: ['untagged']` — already works.**  Confirmed via inspection: [decorateItem](scripts/typediagram-gen.mjs#L774) joins `serdeAttrs` into a single `#[serde(...)]` line, so adding `'untagged'` to a TYPE_CONFIG entry already emits the right code. **No change needed.** Will validate by writing the entry for `RequestId` in batch M1.

**G2. Enum discriminants.** New TYPE_CONFIG knob `variantDiscriminants: { ParseError: -32_700, ... }`. Extend the variant-line walker (currently in [applyVariantDocs](scripts/typediagram-gen.mjs#L746)) so a unit-only enum variant `Foo,` becomes `Foo = <value>,` when the entry declares a discriminant for `Foo`. Doc comments still go above the line. Idempotent: if no `variantDiscriminants` declared, behaviour is unchanged.

**G3. `skipTs: true` flag.** New TYPE_CONFIG knob. The TS post-processor currently emits everything. The `skipTs` flag drops the matching `export type|interface` block from the TS output. Reason: the JSON-RPC envelope has no TS consumer (the VSIX uses VS Code's built-in LSP machinery, not raw JSON-RPC frames; the only TS importers of `wire-generated.ts` are `clients/vscode/src/types/report.ts` which imports report-domain types only — confirmed by Explore). Emitting envelope types into TS would create dead code and an actively-misleading shape (`params: string` instead of `params: unknown`).

### Migration batches (M1–M6)

Each batch adds one envelope type to [docs/models/live-ipc.td](docs/models/live-ipc.td) + a TYPE_CONFIG entry, regenerates, then deletes the hand-rolled struct in `protocol.rs` and re-exports from `wire_generated`. Order is bottom-up so each batch only depends on types from earlier batches.

- **M1. `RequestId`** (untagged enum). Validates G1.
- **M2. `ErrorCode`** (numeric-discriminant enum). Validates G2. `as i32` cast in `JsonRpcError::new` keeps working unchanged.
- **M3. `JsonRpcError`** (uses `ErrorCode` from M2; `data: Option<serde_json::Value>` via `fieldOverrides` from `String` placeholder; `skip_serializing_if` via existing `fieldSerdeAttrs`). Uses G3 `skipTs`.
- **M4. `JsonRpcRequest`** (uses `RequestId` from M1; `params: Option<serde_json::Value>` via override). Uses G3 `skipTs`. The `jsonrpc: String` field replaces the existing `jsonrpc: String` (already `String`, not `&str` on the request side).
- **M5. `JsonRpcResponse`** (uses `RequestId`; `result: serde_json::Value` via override; `jsonrpc: String` replacing the current `jsonrpc: &'static str`). Constructor `JsonRpcResponse::ok` becomes a free function `jsonrpc_response_ok(id, result) -> JsonRpcResponse` in `protocol.rs` since `impl` blocks on the generated struct can't live in `wire_generated.rs`. Uses G3 `skipTs`.
- **M6. `JsonRpcErrorResponse`** (uses `RequestId`, `JsonRpcError`; `jsonrpc: String`). Constructor moved to free function `jsonrpc_error_response(id, error)` in `protocol.rs`. Uses G3 `skipTs`.

### What stays in `protocol.rs`

After M1–M6, `protocol.rs` is reduced to:
- `pub const JSONRPC_VERSION: &str = "2.0";` — string constant, not a wire model
- `pub fn jsonrpc_response_ok(id, result) -> JsonRpcResponse` — convenience constructor
- `pub fn jsonrpc_error_response(id, error) -> JsonRpcErrorResponse` — convenience constructor
- `pub fn jsonrpc_error(code: ErrorCode, message: impl Into<String>) -> JsonRpcError` — convenience constructor (current `JsonRpcError::new`)
- `pub use deslop_core::wire_generated::{ErrorCode, JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, JsonRpcResponse, RequestId};`
- Module docstring rewritten: *no exception cited*; module is just constructors + re-exports now.

`crates/deslop-mcp/src/server.rs`, `tools/mod.rs`, `tools/handlers.rs`, `resources.rs`, `notify.rs` continue to import from `protocol` — only the names change minimally where constructors moved (`JsonRpcResponse::ok` → `jsonrpc_response_ok`, `JsonRpcErrorResponse::new` → `jsonrpc_error_response`, `JsonRpcError::new` → `jsonrpc_error`).

## Wire-shape contracts to preserve

The Explore agent identified these test-asserted wire contracts. Every batch must preserve all of them:

- `"jsonrpc": "2.0"` literal in every frame ([cli.rs:1856, 1892, 1925](crates/deslop-mcp/tests/cli.rs#L1856))
- Error code numeric values: -32_700, -32_600, -32_601, -32_602, -32_603, -32_001, -32_002, -32_003, -32_004 ([cli.rs:1860, 1873, 1883, 1894, 1848](crates/deslop-mcp/tests/cli.rs#L1860))
- `RequestId` round-trips both as JSON number and JSON string ([cli.rs:1928](crates/deslop-mcp/tests/cli.rs#L1928))
- Error frames have **no** `result` key (distinct struct, not a `Result` variant)
- `JsonRpcError.data` is omitted when `None` (skip_serializing_if behaviour)

## Critical files

- [scripts/typediagram-gen.mjs](scripts/typediagram-gen.mjs) — generator extensions (G2, G3)
- [docs/models/live-ipc.td](docs/models/live-ipc.td) — six new types added incrementally
- [crates/deslop-mcp/src/protocol.rs](crates/deslop-mcp/src/protocol.rs) — replace structs with re-exports + free-function constructors
- [crates/deslop-mcp/src/server.rs](crates/deslop-mcp/src/server.rs) — three constructor renames
- [crates/deslop-mcp/src/tools/mod.rs](crates/deslop-mcp/src/tools/mod.rs) — multiple `JsonRpcError::new` → `jsonrpc_error` renames
- [crates/deslop-mcp/src/tools/handlers.rs](crates/deslop-mcp/src/tools/handlers.rs) — same renames
- [crates/deslop-mcp/src/resources.rs](crates/deslop-mcp/src/resources.rs) — same rename
- [crates/deslop-core/build.rs](crates/deslop-core/build.rs) — already auto-runs `typediagram-gen` on every cargo build; no change needed
- [crates/deslop-core/src/wire_generated.rs](crates/deslop-core/src/wire_generated.rs) — gitignored output; regenerated each batch

## Existing functions / patterns reused

- [decorateItem](scripts/typediagram-gen.mjs#L774) already handles `serdeAttrs` joining — used as-is for `untagged`
- [applyFieldOverrides](scripts/typediagram-gen.mjs#L671) already injects `fieldSerdeAttrs` like `skip_serializing_if` — used as-is for `JsonRpcError.data`
- [overrideType](scripts/typediagram-gen.mjs#L714) already remaps placeholder field types — used to swap `String` placeholders for `Option<serde_json::Value>`
- The existing `fieldOverrides: { dimensions: "Option<usize>" }` pattern (e.g. `EmbeddingModelInfo`) is the model for `params: "Option<serde_json::Value>"`

## Verification (per batch)

After every batch:
1. `node scripts/typediagram-gen.mjs` — generator runs clean, both files written
2. `cargo build --workspace --all-targets` — green
3. `cargo clippy --workspace --all-targets --no-deps` — green
4. `git ls-files | grep -iE "wire_generated|wire-generated"` — empty (still untracked)

After M6 (final batch):
5. `cargo test -p deslop-mcp --test cli` — wire-shape tests still green (especially `tools_call_missing_name_returns_invalid_params`, `invalid_jsonrpc_version_returns_invalid_request`, `string_request_id_round_trips_through_dispatch`)
6. `grep "Serialize\|Deserialize" crates/deslop-mcp/src/protocol.rs` — empty (no hand-rolled wire structs remain)
7. `grep "typeDiagram exception" crates/deslop-mcp/src/protocol.rs` — empty (the self-granted exception text is gone)

---

## Detailed TODO

### G1 — Validate untagged works through existing `serdeAttrs`
- [ ] *No code change.* This batch is folded into M1 — the validation IS adding `serdeAttrs: ['untagged']` to RequestId and confirming the generated Rust contains `#[serde(untagged)]`.

### G2 — Generator: numeric enum discriminants
- [ ] In [scripts/typediagram-gen.mjs](scripts/typediagram-gen.mjs), add new function `applyVariantDiscriminants(item, variantDiscriminants)` that walks lines, matches `^(\s*)(\w+),\s*$` (unit variant), and rewrites to `${indent}${variant} = ${disc},` when the variant name has an entry.
- [ ] Wire it into `postprocess()` between `applyFieldOverrides` and `applyVariantDocs` so doc comments stay above the discriminant-bearing line.
- [ ] Run `node scripts/typediagram-gen.mjs` against an unchanged `.td` — output must be byte-identical to the previous run (no enum currently uses discriminants).
- [ ] `cargo build --workspace --all-targets` — green
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### G3 — Generator: `skipTs: true` flag
- [ ] In [scripts/typediagram-gen.mjs](scripts/typediagram-gen.mjs), extend `postprocessTs(ts)` to walk top-level blocks (`export type` and `export interface`) and drop any block whose name is in `TYPE_CONFIG[name].skipTs === true`.
- [ ] Run `node scripts/typediagram-gen.mjs` — TS output unchanged (no entry uses `skipTs` yet).
- [ ] `cargo build --workspace --all-targets` — green
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### M1 — Migrate `RequestId` (validates G1)
- [ ] Add to [docs/models/live-ipc.td](docs/models/live-ipc.td):
  ```
  union RequestId {
    Number { value: Int }
    String { value: String }
  }
  ```
  (Note: typeDiagram unions need a payload; with `untagged` serde will probe each variant. The single-field `value:` shape collapses to a bare number/string on the wire when combined with `serde(untagged)`.)
- [ ] Alternative if the above doesn't round-trip: declare two `String` placeholder fields and use `fieldOverrides` to coerce — *will validate by reading typediagram CLI output before writing TYPE_CONFIG.*
- [ ] Add TYPE_CONFIG entry for `RequestId` with `derives: ["Debug", "Clone", "PartialEq", "Eq", "Serialize", "Deserialize"]`, `serdeAttrs: ['untagged']`, `skipTs: true`.
- [ ] Regenerate; manually inspect `crates/deslop-core/src/wire_generated.rs` to confirm shape matches the existing hand-rolled enum.
- [ ] In [crates/deslop-mcp/src/protocol.rs](crates/deslop-mcp/src/protocol.rs): delete `pub enum RequestId { ... }`; add `pub use deslop_core::wire_generated::RequestId;`.
- [ ] `cargo build -p deslop-mcp` — green
- [ ] `cargo test -p deslop-mcp --test cli string_request_id_round_trips_through_dispatch` — green
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### M2 — Migrate `ErrorCode` (validates G2)
- [ ] Add to [docs/models/live-ipc.td](docs/models/live-ipc.td):
  ```
  union ErrorCode {
    ParseError
    InvalidRequest
    MethodNotFound
    InvalidParams
    InternalError
    UnparseableInput
    UnsupportedLanguage
    PathOutsideRoot
    BackendError
  }
  ```
- [ ] Add TYPE_CONFIG entry: `derives: ["Debug", "Clone", "Copy"]`, `variantDiscriminants: { ParseError: "-32_700", InvalidRequest: "-32_600", MethodNotFound: "-32_601", InvalidParams: "-32_602", InternalError: "-32_603", UnparseableInput: "-32_001", UnsupportedLanguage: "-32_002", PathOutsideRoot: "-32_003", BackendError: "-32_004" }`, `skipTs: true`. **Also add `#[repr(i32)]`** (via a new `reprAttr: "i32"` flag, OR by appending to `derives` as a string injected before — actually simpler: emit `#[repr(i32)]` whenever `variantDiscriminants` is set).
- [ ] Confirm `as i32` casts in `JsonRpcError::new` (currently `code as i32`) still compile — `#[repr(i32)]` makes this lossless.
- [ ] In [crates/deslop-mcp/src/protocol.rs](crates/deslop-mcp/src/protocol.rs): delete `pub enum ErrorCode { ... }`; add `pub use deslop_core::wire_generated::ErrorCode;`.
- [ ] `cargo build -p deslop-mcp` — green
- [ ] `cargo test -p deslop-mcp --test cli` (whole suite, since 24+ callsites use `ErrorCode::*`) — green
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### M3 — Migrate `JsonRpcError`
- [ ] Add to [docs/models/live-ipc.td](docs/models/live-ipc.td):
  ```
  type JsonRpcError {
    code: Int
    message: String
    data: Option<String>
  }
  ```
- [ ] Add TYPE_CONFIG entry: `derives: ["Debug", "Clone", "Serialize", "Deserialize"]`, `fieldOverrides: { code: "i32", data: "Option<serde_json::Value>" }`, `fieldSerdeAttrs: { data: ['skip_serializing_if = "Option::is_none"'] }`, `skipTs: true`.
- [ ] In [crates/deslop-mcp/src/protocol.rs](crates/deslop-mcp/src/protocol.rs): delete `pub struct JsonRpcError { ... }` and the `impl JsonRpcError { pub fn new(...) }`. Add `pub use deslop_core::wire_generated::JsonRpcError;` and a free function `pub fn jsonrpc_error(code: ErrorCode, message: impl Into<String>) -> JsonRpcError { JsonRpcError { code: code as i32, message: message.into(), data: None } }`.
- [ ] Replace every `JsonRpcError::new(` callsite with `jsonrpc_error(` across `server.rs`, `tools/mod.rs`, `tools/handlers.rs`, `resources.rs` (~24 sites). Add a single `use crate::protocol::jsonrpc_error;` to each consumer.
- [ ] `cargo build -p deslop-mcp` — green
- [ ] `cargo test -p deslop-mcp --test cli` — green
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### M4 — Migrate `JsonRpcRequest`
- [ ] Add to [docs/models/live-ipc.td](docs/models/live-ipc.td):
  ```
  type JsonRpcRequest {
    jsonrpc: String
    method: String
    params: Option<String>
    id: Option<RequestId>
  }
  ```
- [ ] TYPE_CONFIG: `derives: ["Debug", "Clone", "Deserialize"]`, `fieldOverrides: { params: "Option<serde_json::Value>" }`, `fieldSerdeAttrs: { params: ['default'], id: ['default'] }`, `skipTs: true`.
- [ ] In `protocol.rs`: delete the struct, add re-export. The existing `request.jsonrpc != JSONRPC_VERSION` comparison (currently `String` vs `&str`) keeps working unchanged because `String: PartialEq<&str>`.
- [ ] `cargo build -p deslop-mcp` — green
- [ ] `cargo test -p deslop-mcp --test cli` — green (especially `invalid_jsonrpc_version_returns_invalid_request`)
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### M5 — Migrate `JsonRpcResponse`
- [ ] Add to [docs/models/live-ipc.td](docs/models/live-ipc.td):
  ```
  type JsonRpcResponse {
    jsonrpc: String
    id: RequestId
    result: String
  }
  ```
- [ ] TYPE_CONFIG: `derives: ["Debug", "Clone", "Serialize"]`, `fieldOverrides: { result: "serde_json::Value" }`, `skipTs: true`.
- [ ] In `protocol.rs`: delete struct + `impl ok`. Add re-export and free function `pub fn jsonrpc_response_ok(id: RequestId, result: Value) -> JsonRpcResponse { JsonRpcResponse { jsonrpc: JSONRPC_VERSION.to_owned(), id, result } }`.
- [ ] Replace `JsonRpcResponse::ok(` callsite (single site at [server.rs:152](crates/deslop-mcp/src/server.rs#L152)) with `jsonrpc_response_ok(`.
- [ ] `cargo build -p deslop-mcp` — green
- [ ] `cargo test -p deslop-mcp --test cli` — green
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### M6 — Migrate `JsonRpcErrorResponse`
- [ ] Add to [docs/models/live-ipc.td](docs/models/live-ipc.td):
  ```
  type JsonRpcErrorResponse {
    jsonrpc: String
    id: Option<RequestId>
    error: JsonRpcError
  }
  ```
- [ ] TYPE_CONFIG: `derives: ["Debug", "Clone", "Serialize"]`, `skipTs: true`.
- [ ] In `protocol.rs`: delete struct + `impl new`. Add re-export and free function `pub fn jsonrpc_error_response(id: Option<RequestId>, error: JsonRpcError) -> JsonRpcErrorResponse { ... }`.
- [ ] Replace three `JsonRpcErrorResponse::new(` callsites in [server.rs](crates/deslop-mcp/src/server.rs) (~lines 102, 110, 153) with `jsonrpc_error_response(`.
- [ ] **Rewrite `protocol.rs` module docstring:** remove the "typeDiagram exception" section entirely. New docstring states: *"JSON-RPC 2.0 envelope re-exports + small constructor helpers. Wire types are generated from `docs/models/live-ipc.td`."*
- [ ] `cargo build -p deslop-mcp` — green
- [ ] `cargo test -p deslop-mcp --test cli` — green (full envelope wire contract verified)
- [ ] `cargo clippy --workspace --all-targets --no-deps` — green

### Final verification
- [ ] `grep -E "^#\[derive.*Serialize|^#\[derive.*Deserialize" crates/deslop-mcp/src/protocol.rs` — empty
- [ ] `grep -i "exception" crates/deslop-mcp/src/protocol.rs` — empty
- [ ] `git ls-files | grep -iE "wire_generated|wire-generated"` — empty (still gitignored, untracked)
- [ ] Full `cargo test --workspace` (excluding the pre-existing Ollama-dependent failures noted in earlier sessions) — green
