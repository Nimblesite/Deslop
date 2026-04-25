# Autofix — Extract Method (true Type-1 only)

Deslop's first mechanical autofix: an LSP `textDocument/codeAction` of kind `refactor.extract` that, given a cluster of **true Type-1 occurrences** (raw token streams byte-identical post-trivia), emits a single shared method and rewrites every occurrence in the cluster as a call to it. **One refactor, N call-site replacements, one `WorkspaceEdit`.**

This spec covers v1: pure tree-sitter, no semantic model. Type-2 (renamed-identifier) clusters, cross-file extraction, and type inference are explicit non-goals — see [AUTOFIX-EXTRACT-NORTH-STAR].

## [AUTOFIX-EXTRACT-NORTH-STAR] Scope and non-goals

What this is:

- A v1 best-effort refactor for the unambiguously-easiest clone bucket. No Roslyn / rust-analyzer / pyright dependency. The result is offered as a code action the user reviews and accepts.
- A power-user shortcut alongside the existing diagnostic, hover, and code lens — never the only suggested fix for a duplicate.

What this is **not**:

- **Not Type-2.** Renamed identifiers/literals require per-site argument lists; a single shared method has a single signature. Type-2 needs a real refactor engine and lands later.
- **Not cross-file in v1.** All occurrences must share the same file URI.
- **Not cross-class in v1.** Even within a file, occurrences in two different classes are skipped.
- **Not type-aware.** Parameter and return types are syntactic placeholders. The user accepts the result may not compile.
- **Not destination-configurable.** Per-language destination policy is fixed; the user moves the helper afterward if they want.

User-facing copy on the action: *"Extract identical code to shared method"*. Caveat shown in the action description: *"Result may need manual type or scope fixes."*

## [AUTOFIX-EXTRACT-PRECONDITIONS] When the action is offered

For a cluster `C` to be eligible, **all** of these must hold:

1. `C.kind == ClusterKind::Identical` **and** `C.kind_detail == Type1` — the post-#42 split that distinguishes true Type-1 (raw token bytes match) from Type-2 (normalised tokens match). Without the split this action is unsafe to offer; see [AUTOFIX-EXTRACT-DEPENDENCIES].
2. `C.occurrences.len() ≥ 2`.
3. Every occurrence resolves to the **same file URI**.
4. Every occurrence's enclosing scope is a method/function (C#: method / property accessor / local function; Rust: `fn` / `impl fn`; Python: `def` / `async def` / module top-level), and every occurrence shares the **same enclosing parent one level up** (C#: same containing class; Rust: same `impl` block or same module; Python: same containing class or same module).
5. The block aligns with statement boundaries in the parse tree — start and end byte ranges sit between statements, not mid-expression. Mid-expression occurrences are silently skipped.

If any precondition fails, no action is offered for the cluster. Failures are silent — there is no diagnostic.

## [AUTOFIX-EXTRACT-FREE-VARS] Free-variable analysis

For each occurrence, compute the **free-variable list** by walking the tree-sitter parse subtree of the byte range:

1. Initialise an empty scope stack.
2. Pre-order walk the subtree.
3. At every node that introduces a binding (parameter list, `let` / `var` / `const` / `val`, `for`-loop binding, pattern binding, lambda parameters, Python assignment to a never-before-seen name in the block), push the bound names into the current scope frame.
4. At every identifier-reference node, if no scope frame in the stack — including frames pushed during this walk — declares the name, record it as free.
5. Emit free names in **first-reference textual order**, deduplicated.

For Type-1 the same algorithm applied to every occurrence yields the **same identifier list**, because the raw token streams are byte-identical. The emitter relies on this: free-vars computed from any one occurrence become the parameter list, and the call sites pass the same identifiers verbatim.

The walk is **purely syntactic**:

- Does not resolve member access (`this.Foo`, `self.foo`, `Foo` as an implicit member in C#).
- Does not consult symbol tables.
- Does not check whether a name is in scope outside the block.
- Does not infer types.

Bare identifiers that happen to be member references are treated as free variables and emitted as parameters; in those cases the produced refactor will not compile and the user must adjust. This is a documented limitation, not a bug — see [AUTOFIX-EXTRACT-CAVEATS].

Per-language node-kind tables (binding-introducing nodes, identifier-reference nodes) live with each `LanguageParser` implementation. **New languages add free-var support by extending the trait — same single extension point as parsing.**

## [AUTOFIX-EXTRACT-EMITTER] Code emission

A new method declaration plus N call-site rewrites. Per-language emitters produce the textual form; the LSP layer assembles the `WorkspaceEdit`.

**Method-name strategy:** derive a deterministic name from the cluster id — `ExtractedFromCluster_<6-char-prefix>` (C# / Rust pascal-ish via the cluster id), `extracted_from_cluster_<6-char-prefix>` (Python). Stable across runs so re-applying the action on the same cluster doesn't churn names; required for golden tests.

### [AUTOFIX-EXTRACT-EMITTER-CSHARP]

```csharp
private static <retType> <Name>(<params>)
{
    <body>
}
```

- `<retType>`: `void` if the block contains no value-producing `return` or `yield` statements, otherwise `object` followed by `// TODO: deslop — fix return type`. (Tree-sitter detects the presence of value-producing returns; it cannot infer the type.)
- `<params>`: each free var rendered as `object <name> /* TODO: deslop — fix type */`.
- Method placed at the **top of the body of the enclosing class**, just inside the opening brace, indented one step beyond the class declaration's leading whitespace.
- Call site replacement: `<Name>(<freeVar1>, <freeVar2>, ...);` — single-statement replacement.
- Modifier rationale: `private static` is the safest default. The user can re-scope after the fact. Never `public` because that crosses an API boundary.

### [AUTOFIX-EXTRACT-EMITTER-RUST]

```rust
// TODO: deslop — replace `DeslopTodo` with real types.
type DeslopTodo = ();

fn <name>(<params>) -> DeslopTodo {
    <body>
}
```

- Each free var rendered as `<name>: DeslopTodo`. Single placeholder alias is the cleanest way to keep the emitted code parseable while flagging every type slot the user must fix.
- Free function placed at **module scope**, immediately above the function containing the first occurrence. No `impl` migration in v1 — instance-method extraction is out of scope.
- Call site replacement: `<name>(<freeVar1>, <freeVar2>, ...)`. Trailing `;` if the original site had one.

### [AUTOFIX-EXTRACT-EMITTER-PYTHON]

```python
def <name>(<params>):
    <body>
```

- No annotations, no return type. Python's optional typing makes this the easiest of the three.
- `<params>`: bare names, no defaults.
- Function placed at **module scope**, immediately above the function or class containing the first occurrence. Two blank lines above and below, matching PEP 8.
- Call site replacement: `<name>(<freeVar1>, <freeVar2>, ...)`. The original surrounding statement context (assignment LHS, `return`, expression statement) is preserved — the emitter only replaces the byte range of the block.

## [AUTOFIX-EXTRACT-DESTINATION] Destination policy

Fixed per-language; no user prompt in v1.

- **C#** — top of the body of the enclosing class.
- **Rust** — module scope, immediately above the function containing the first occurrence.
- **Python** — module scope, immediately above the function or class containing the first occurrence.

The user moves the helper after extraction if they want it elsewhere — the IDE's normal "move method" refactor is the right tool for that, not Deslop.

## [AUTOFIX-EXTRACT-WORKSPACE-EDIT] LSP `WorkspaceEdit` shape

The action returns one `WorkspaceEdit` containing:

- One `TextEdit` inserting the method declaration (and any required type alias) at the destination range.
- N `TextEdit`s, one per occurrence, replacing the occurrence byte range with the call-site form.

All edits target the **same document**. The edits are applied atomically by the editor (`workspace.applyEdit`); if any individual edit fails (e.g. the file changed between code-action computation and apply), the entire refactor is rejected and re-offered on the next compute cycle.

Edits are emitted in **descending start position** so earlier edits don't shift later edits' offsets — standard LSP convention.

## [AUTOFIX-EXTRACT-CODE-ACTION] LSP integration

`backend.rs` advertises `codeActionProvider` with `resolveProvider: false` (action computation is cheap; no resolve round-trip needed) and `codeActionKinds: ["refactor.extract"]`.

`textDocument/codeAction` computes:

1. Look up clusters intersecting the requested range via `LiveApi`.
2. For each intersecting cluster, evaluate [AUTOFIX-EXTRACT-PRECONDITIONS].
3. For each eligible cluster, compute the `WorkspaceEdit` per [AUTOFIX-EXTRACT-WORKSPACE-EDIT].
4. Return one `CodeAction` per eligible cluster: `kind: "refactor.extract"`, `title: "Extract identical code to shared method"`, `edit: <WorkspaceEdit>`.

If no cluster is eligible, return an empty list. Never return a code action with a missing or partial edit — the user must always be able to apply the action atomically.

## [AUTOFIX-EXTRACT-CAVEATS] What may break

The user is told upfront on the action title's caveat line and in this spec section that any of the following may produce non-compiling code:

- Free variables that are member references (`this.X`, `self.x`, implicit-`this` in C#) — emitted as parameters by mistake.
- Captured `this` / instance-state usage — extracted method is `static` and won't see instance members.
- Free variables whose runtime types differ across occurrences but happen to share the same name (rare for true Type-1).
- Return-type inference is best-effort — `object` (C#), `DeslopTodo` (Rust), unannotated (Python).

The action is **never the only suggested fix** for a duplicate. The diagnostic, the hover, and the existing manual workflow remain. This is a power-user shortcut, not a replacement for review.

## [AUTOFIX-EXTRACT-TESTING] E2E coverage

Coarse end-to-end only, per CLAUDE.md. `crates/deslop-lsp/tests/code_action.rs` spawns the real LSP binary and:

1. Opens a fixture C# file containing two byte-identical method bodies in the same class. Asserts a `refactor.extract` code action is offered, the `WorkspaceEdit` inserts a `private static` method, both occurrences become call sites, and the resulting buffer matches a golden snapshot.
2. Same shape for Rust (free function inserted at module scope plus the `DeslopTodo` alias) and Python (function at module scope).
3. Asserts **no** action is offered when:
   - The cluster is Type-2 (renamed identifiers between occurrences).
   - Occurrences span two files.
   - Occurrences are in different classes within the same file (C#).
   - Occurrence count is 1.
   - The block straddles a statement boundary (mid-expression).
4. Asserts the cluster id appears in the inserted method name (deterministic naming).

Goldens live under `crates/deslop-lsp/tests/fixtures/code_action/`. Test references the `[AUTOFIX-EXTRACT-*]` ID it covers, per CLAUDE.md.

## [AUTOFIX-EXTRACT-DEPENDENCIES] Hard prerequisites

1. **Issue [#42](https://github.com/Nimblesite/Deslop/issues/42)** — bucket must distinguish true Type-1 from Type-2. Without the split, this action would silently fire on Type-2 clusters and produce structurally-wrong refactors at every site (not just type-wrong — the call-site argument lists would not match the method signature, because Type-2 free-var names differ across occurrences). Implementation is **blocked** on #42.
2. **`LanguageParser` trait extension** — new methods for free-variable computation (`binding_node_kinds`, `identifier_reference_kinds`) and for emitting language-shaped method declarations (`emit_extract_method`). The trait is the single extension point for languages; this work belongs there.

---

## [AUTOFIX-EXTRACT-AI] AI-assisted extraction for Type-2 and Type-3

The Type-1 path above stays mechanical because byte-identical tokens guarantee identical free-variable names at every site. **Type-2 (renamed identifiers) and Type-3 (small structural drift) break that invariant** — site A's free vars are `(customer, name)` while site B's are `(admin, label)` — so a single shared signature needs **chosen** parameter names, not derived ones. Choosing names is intent work, not syntax work.

This section specifies the **AI-assisted path**: maximally mechanical, with a tightly bounded AI slot for the non-deterministic bits. The AI never writes code, never edits files, and never sees the broader workspace. It fills named placeholders in a Deslop-built scaffold, and Deslop synthesises the final edit deterministically.

### [AUTOFIX-EXTRACT-AI-NORTH-STAR] What is and isn't AI

**Mechanical (always, AST-driven):**

- Cluster selection — already done by the pipeline.
- Per-occurrence free-variable extraction via the same scope walk as Type-1 ([AUTOFIX-EXTRACT-FREE-VARS]).
- **Parameter-slot inference** — site A's `customer` and site B's `admin` are recognised as the **same parameter slot** because they sit at the same source position in the normalised AST. There are N parameter slots, each with M candidate names (one per occurrence).
- Method-body skeleton with parameter slots referenced by canonical placeholder names (`__deslop_param_0`, `__deslop_param_1`, …). The body is identical to one canonical occurrence's body (chosen by lowest byte offset for determinism), with a token-by-token rewrite from per-site identifiers to placeholders.
- Per-site call-site argument lists, computed from each site's free-vars in slot order.
- Destination policy — same rules as Type-1 ([AUTOFIX-EXTRACT-DESTINATION]).
- Final `WorkspaceEdit` synthesis from the AI-chosen names.

**AI-filled (non-deterministic, bounded slots):**

- One **method name** that describes intent.
- One **canonical name per parameter slot** — chosen from the M candidates, or a generalising name (`customer` + `admin` → `entity`).
- Optionally, a one-line summary doc-comment for the helper.

That is the entire AI surface. The AI **does not**:

- Choose what to extract.
- Modify the method body.
- Pick the destination.
- Generate file edits or write source code.
- Add or remove statements.
- Modify types beyond chosen parameter names (in v1).

### [AUTOFIX-EXTRACT-AI-SCAFFOLD] The mechanical scaffold

Given a Type-2 or Type-3 cluster, Deslop computes:

1. Slot count, ordered by first-source-position appearance, with each slot's candidate names (one per occurrence).
2. The body string, rendered with `__deslop_param_<i>` placeholders in place of per-site identifiers, derived from the canonical occurrence.
3. Per-site argument lists — each occurrence's free-var names in slot order.
4. The destination byte range and surrounding context (enclosing class for C# / module for Rust / Python).
5. Per-language sentinel character set — the placeholder names are guaranteed to not occur anywhere else in the destination file, so the substitution at apply time is collision-free.

The scaffold is fully serialisable JSON — no source-text fragments embedded beyond the body string and the call-site identifier names; just byte ranges, names, and slot indices.

### [AUTOFIX-EXTRACT-AI-MCP-TOOLS] MCP tool surface

This work ships as **two new MCP tools** rather than a generic LSP code action, because the AI must remain inside a bounded slot and never produce arbitrary text. Tool naming follows the existing kebab-case convention from [mcp.md §[MCP-TOOLS]](mcp.md#mcp-tools).

| Tool | Inputs | Output | Purpose |
|---|---|---|---|
| `extract-method-plan` | `{ cluster_id }` | `ExtractScaffold` | Returns the mechanical scaffold for the cluster: slot count, candidate names per slot, body with placeholders, per-site argument lists, destination byte range. The agent reads this and decides on a method name + canonical per-slot names. **Read-only.** |
| `extract-method-apply` | `{ cluster_id, method_name, parameter_names: [string], summary?: string }` | `WorkspaceEdit` | Validates the supplied names per [AUTOFIX-EXTRACT-AI-VALIDATION], substitutes them into the scaffold, returns the final atomic `WorkspaceEdit`. **Does not write files** — the agent's host applies the edit through its normal channel. |

The agent's responsibility: read the scaffold, pick names that read well in the host language, call `extract-method-apply`. Deslop's responsibility: validate + assemble. Neither side trusts the other to do the other's job.

The LSP **does not** expose this as a `textDocument/codeAction` in v1 — code actions are synchronous and the AI round-trip is not. A future addition could surface a *deferred* code action that triggers an agent through the host's MCP client, but the canonical surface is the tool pair.

### [AUTOFIX-EXTRACT-AI-PRECONDITIONS] When the AI path applies

For a cluster `C` to be eligible:

1. `C.kind == ClusterKind::Identical` with `kind_detail == Type2` (post-#42), **or** `C.kind == ClusterKind::NearlyIdentical` (Type-3) **and** every occurrence's free-variable list agrees on slot count and slot source-position.
2. Same single-file constraint as Type-1 ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 3).
3. Same single-class / single-module constraint ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 4).
4. **Slot alignment succeeded** — every occurrence agrees on the count and source-position of free-variable slots. Type-3 clusters where occurrences differ in free-var arity are skipped (the scaffold is undefined).

### [AUTOFIX-EXTRACT-AI-VALIDATION] Validation on AI output

`extract-method-apply` rejects with a typed error and the scaffold reduced for retry if any of:

- `method_name` is not a valid identifier in the cluster's language.
- `method_name` collides with an existing member at the destination (C# class, Rust module, Python module).
- `parameter_names.len() != slot_count`.
- Any `parameter_names[i]` is not a valid identifier.
- `parameter_names` contains a duplicate.
- Any `parameter_names[i]` collides with a name already in scope at the destination's enclosing context.

Apply is **idempotent**: same `cluster_id` + same valid inputs → same `WorkspaceEdit` byte-for-byte. Required for golden tests and agent retries.

### [AUTOFIX-EXTRACT-AI-NON-GOALS]

- AI does **not** generate body code — the scaffold body is final.
- AI does **not** decide whether to extract — the user / agent host invokes the tool.
- AI does **not** see the surrounding file beyond what the scaffold exposes.
- AI does **not** modify types in v1 — placeholders carry over from [AUTOFIX-EXTRACT-EMITTER].
- The MCP tools **never** write files — the host applies the returned `WorkspaceEdit`.
- No "freeform extract" tool that takes arbitrary code from the agent. Every input flows through Deslop-computed scaffolds.

### [AUTOFIX-EXTRACT-AI-DEPENDENCIES]

1. **Type-1 path** — [AUTOFIX-EXTRACT-NORTH-STAR] through [AUTOFIX-EXTRACT-WORKSPACE-EDIT] ships first. The free-variable walk, the per-language emitter trait, and the `WorkspaceEdit` assembly are reused by the AI path with a placeholder-substitution layer added on top.
2. **`LanguageParser` slot-alignment method** — takes N parse subtrees, returns `Option<SlotMapping>`. Pure syntax, no AI. Single extension point per language.
3. **MCP server new tools** — `extract-method-plan` and `extract-method-apply` added alongside `find-similar` / `cluster-by-id`. Tool descriptions follow [MCP-AGENT-PROMPT-GUIDANCE] — they're prompt engineering for the host agent.
