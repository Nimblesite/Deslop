# Autofix — Mechanical (Zero-Risk) Deduplication

Deslop's mechanical autofixes rewrite duplicate code **without an AI in the loop and without changing behaviour.** The family, in escalating order of what each handles:

1. **[AUTOFIX-EXTRACT]** — true **Type-1** clusters (raw token streams byte-identical post-trivia): emit one shared method, rewrite every occurrence as a call. One refactor, N call-site replacements, one `WorkspaceEdit`.
2. **[AUTOFIX-MERGE]** — **leaf-gap Type-2 / constrained Type-3** clusters (the 50+-call-site case): anti-unify the occurrences into one parameterised helper whose differing leaves become parameters — with **default values** for positions that are constant across (almost) every site — then rewrite all sites.
3. **[AUTOFIX-CONSOLIDATE]** — an **identical definition duplicated across files**: keep one canonical copy, delete the duplicates, and rewrite imports/references everywhere.
4. **[AUTOFIX-EXTRACT-AI]** — the **fallback** for what is genuinely not mechanical (structural drift, Type-4, intent-laden naming).

## [AUTOFIX-ZERO-RISK] Why mechanical beats handing it to AI

Getting the right context to an AI is hard, and even when it lands the AI often gets the refactor wrong. **Every cluster Deslop can drain mechanically is one that never needs the lossy, error-prone AI handoff.** The mechanical path is preferable wherever it applies, and it applies far more often than the original `[AUTOFIX-EXTRACT]`-only framing assumed.

Two pillars make these actions zero-risk:

- **Correctness by construction.** Each action is a *behaviour-preserving transformation* in the sense of Opdyke — it is applied only when a machine-checkable precondition set holds ([AUTOFIX-MERGE-SAFETY]). `[AUTOFIX-MERGE]` computes its template and per-site argument lists by **anti-unification** (least general generalisation, Plotkin/Reynolds); the parameter *name* is cosmetic and never affects behaviour, so AI naming is at most an optional readability polish, never a correctness prerequisite.
- **The static type checker is the backstop.** In a type-safe language a mechanically-produced edit that is somehow wrong becomes a **compile error the developer sees immediately** — never a silent runtime behaviour change. The primary targets are therefore **Dart, C#, and Rust**. **Python qualifies only when strict type checking (basedpyright / pyright `strict`) is on** (gated via `session-config`); otherwise `[AUTOFIX-MERGE]` / `[AUTOFIX-CONSOLIDATE]` refuse for Python and route to the AI fallback.

When any precondition is undecidable or fails, the action **refuses and routes to `[AUTOFIX-EXTRACT-AI]`** — biased, per Opdyke, toward a false "unsafe" over a false "safe".

## [AUTOFIX-EXTRACT-NORTH-STAR] Scope and non-goals (the Type-1 verbatim action)

This section governs `[AUTOFIX-EXTRACT]` specifically — the simplest action, pure tree-sitter, no semantic model. Type-2 leaf-gap clusters and cross-file duplicates are **not** abandoned to AI; they are handled mechanically by `[AUTOFIX-MERGE]` and `[AUTOFIX-CONSOLIDATE]` below.

What this is:

- A best-effort refactor for the unambiguously-easiest clone bucket. No Roslyn / rust-analyzer / pyright dependency. The result is offered as a code action the user reviews and accepts.
- A power-user shortcut alongside the existing diagnostic, hover, and code lens — never the only suggested fix for a duplicate.

What this is **not**:

- **Not Type-2 — that is `[AUTOFIX-MERGE]`.** Renamed identifiers/literals need per-site argument lists; the verbatim single-signature extract here cannot produce them. The mechanical answer is anti-unification ([AUTOFIX-MERGE]), not AI.
- **Not cross-file — that is `[AUTOFIX-CONSOLIDATE]`.** This action requires all occurrences to share the same file URI; cross-file identical definitions are consolidated mechanically by `[AUTOFIX-CONSOLIDATE]`.
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

## [AUTOFIX-MERGE] Mechanical call-site merge (leaf-gap Type-2 / constrained Type-3)

The 50+-call-site case: N occurrences sharing one skeleton, differing only in a handful of leaf positions. Merge them into a single parameterised helper whose differing leaves become parameters, and rewrite every site. Bibliography: [reading-list.md §READ-LIST-MERGE](reading-list.md#read-list-merge).

**Reuses, without restating:** the free-variable walk ([AUTOFIX-EXTRACT-FREE-VARS]); the slot-alignment + scaffold machinery ([AUTOFIX-EXTRACT-AI-SCAFFOLD], plus the slot bullets of [AUTOFIX-EXTRACT-AI-NORTH-STAR]); the per-language emitter trait; and the `WorkspaceEdit` assembly ([AUTOFIX-EXTRACT-WORKSPACE-EDIT]). `[AUTOFIX-MERGE]` **is** that scaffold with the AI name-selection step replaced by mechanical name derivation + default-value computation.

### [AUTOFIX-MERGE-GATE] Which clusters are mechanically mergeable

`decide_mergeability(cluster) -> Mechanical | AiOrHuman(reason)` (Baker p-match; Roy/Cordy taxonomy; Baxter similarity; Bellon thresholds):

```
0  size guard: AST mass >= MassThreshold AND span >= 6 lines (Bellon); >=2 survivors.
1  skeleton: strip trivia; replace param-position leaves (ident/literal/type, leaf arg-exprs)
   with PARAM, keep values in traversal order; remaining tree = skeleton.
2  hash skeleton ignoring PARAM leaves (Baxter); confirm exact skeleton equality in-bucket.
   no PARAM differs       -> Type-1 -> defer to [AUTOFIX-EXTRACT] (verbatim extract).
   skeletons not equal     -> Type-3+ -> AiOrHuman.
3  consistency (Baker prev-encoding): consistent bijective rename -> liftable; else treat each
   differing leaf as its own parameter.
4  gate (ALL must hold) -> Mechanical, else AiOrHuman:
   (a) min Similarity(Fi,rep) >= SIM_THRESHOLD (~0.95)         [Baxter 2S/(2S+L+R)]
   (b) max DIFF_LEAVES(Fi) <= MAX_DIFF_LEAVES (a handful)
   (c) PARAM_ARITY <= MAX_PARAMS
   (d) EVERY differing position is a LEAF/argument position — no structural/control-flow diff.
```

Unconstrained Type-3 (statements added/removed, control-flow drift) and Type-4 route to AI: Bellon shows experts disagree on most Type-3 candidates and Type-3 similarity is non-transitive — judgement, not auto-merge.

### [AUTOFIX-MERGE-ANTIUNIFY] The template and the per-site arguments

First-order syntactic anti-unification / least general generalisation (Plotkin 1970; Reynolds 1970; rule form per Cerna & Kutsia 2023; applied to ASTs by Bulychev & Minea 2008; shipped end-to-end by Li & Thompson 2009 / Wrangler):

```
STATE = (g, P, store)   g: template (starts as one fresh var); P: pending x:s~u;
                        store: (s,u)->var  (the coalesce map that makes the result the *least* gen)
DECOMPOSE (heads agree f/n):  g[x:=f(x1..xn)]; recurse on children pairwise   (keep f literally)
SOLVE     (heads differ):     store has (s,u)->y ? g[x:=y]          (REUSE the parameter)
                                                  : store[(s,u)]:=x  (NEW parameter; leaf in g)
RESULT  g = the helper body; each surviving leaf var x carries sigma_j(x) at site j.
N sites: fold pairwise carrying the store (or n-ary: reuse a var iff the whole N-tuple matches).
   Each leaf var's N-vector of values, read per site, IS that site's argument list.
GUARD   reject if #leaf-vars is large vs #preserved nodes (tiny skeleton + many holes = bad merge).
```

### [AUTOFIX-MERGE-SAFETY] Behaviour-preservation preconditions

Declare MECHANICAL only if ALL pass (Opdyke behaviour-preservation; Komondoor & Horwitz extraction; Schäfer binding soundness; Tsantalis refactorability; value-vs-thunk per arXiv:2512.21511). Any failure or undecidable check → REFUSE, route to [AUTOFIX-EXTRACT-AI]:

```
A STRUCTURAL : occurrences identical up to consistent local renaming + fixed holes at the SAME
   tree-aligned positions.
B EXTRACTABLE: single-entry/single-exit; no return/break/continue/goto/throw-caught-outside/
   yield/await/`?` crossing the boundary; no local declared-inside read-after.
C BINDING    : simulate the move; lookup(ref)_after == lookup(ref)_before for every reference and
   every generated call expr (Schäfer) — no new shadowing/capture/overload change; preserve
   read/write order of free vars; do not straddle a lock/await/transaction/Rust-borrow boundary.
D HOLE SAFETY: each hole pure & evaluation-timing-unchanged -> VALUE parameter; else -> a DEFERRED
   THUNK (C# Func<>, Python lambda, Dart closure, Rust Fn/FnOnce) invoked at the original program
   point (preserves order, defers side effects); if even a thunk can't preserve order -> REFUSE.
   All variants of a hole must unify to one type T (supertype/interface/generic/trait bound); no
   common type without an unchecked cast -> REFUSE.
E ACCESS     : helper reachable/visible from EVERY site; every symbol it references accessible from
   the helper's location for every site; unique non-colliding name.
F ATOMIC     : rewrite ALL n sites in one change -> no default needed, default hazards vanish.
```

### [AUTOFIX-MERGE-NAMES] Mechanical naming, defaults, and the type-safety backstop

- **Names (no AI).** Per slot, the **modal candidate identifier** across sites if it is one valid identifier, else positional (`arg0`, `arg1`). Deterministic — same cluster id, same names — for golden tests. AI naming ([AUTOFIX-EXTRACT-AI]) is an optional readability post-pass, never required.
- **Defaults.** Per slot, inspect its N-vector. If all-but-≤K sites share one value, that value is the slot's **default** and only the outliers pass it; otherwise the slot is **required**. Because (F) rewrites every site, defaults are a *readability* win (common sites collapse to a bare call), never a correctness mechanism.
- **Types (the backstop).** The parameter type is the unified type from (D). In Dart/C#/Rust the compiler verifies the merged result — a mis-typed merge is a compile error, never a silent change. This **replaces** the `object`/`DeslopTodo` placeholder approach for the mechanical path: if no type unifies, REFUSE rather than emit guess-typed code.

### [AUTOFIX-MERGE-DEFAULTS] Per-language default feasibility (type-safe first)

Defaults are only consulted when some callers cannot be rewritten atomically (e.g. a public API used outside the change set); otherwise (F) makes them moot.

| Lang | Mechanical default rule |
|---|---|
| **C#** | Optional param only if the default is a compile-time const **and** all callers recompile in the change set; else add a forwarding **overload** (avoids the baked-in cross-assembly default hazard). |
| **Dart** | Named/optional default only if the value is `const`; else nullable param + `?? computeDefault()`. |
| **Rust** | No defaults, no overloading. Equivalents (all need atomic rewrite, F): pass explicitly; `Option<T>` + branch on `None`; a forwarding wrapper fn; or a builder / `Default`. |
| **Python** | (strict-typing only) immutable-literal default OK; mutable/computed default uses the `None`-sentinel idiom (`def f(x=None): x = x if x is not None else ...`). |

## [AUTOFIX-CONSOLIDATE] Cross-file identical-definition consolidation

When a cluster's occurrences are **whole top-level definitions** (function / class / impl) that are Type-1/Type-2 identical but live in ≥2 files (differing only by filename / surrounding imports): keep one canonical copy, **delete the duplicate definitions**, and **rewrite every reference** in the duplicates' dependents to the canonical symbol.

This is the one mechanical action needing **semantic depth** beyond tree-sitter: a per-language **import/symbol resolver** that builds the reference graph — the same primitive a language server's *rename symbol* / *move* uses. Its correctness invariant is Schäfer's: `lookup(ref)_after == lookup(ref)_before` for every reference; Steimann's constraint solving computes the extra import/qualification edits required.

### [AUTOFIX-CONSOLIDATE-GATE] Preconditions

REFUSE unless all hold: every duplicate definition is reference-resolvable; consolidation introduces no name collision at the canonical location; no visibility/orphan-rule break (Rust `pub`/crate visibility, trait orphan rule); every dependent reference is in the change set or reachable via the workspace index. The type-safety backstop applies — in typed languages a missed/incorrect import is an immediate compile error (Python only under strict typing).

### [AUTOFIX-CONSOLIDATE-EDIT] The WorkspaceEdit

`documentChanges` mixing resource operations and text edits, executed in array order: a `DeleteFile` for any duplicate file that becomes empty (else a `TextDocumentEdit` deleting the definition), plus one `TextDocumentEdit` per dependent rewriting its import/reference to the canonical symbol. Versioned identifiers; `changeAnnotations` for the preview tree; `failureHandling: transactional`; single undo label.

## [AUTOFIX-CATALOG] The zero-risk mechanical-fix catalog

The full surface of behaviour-preserving, no-AI deduplication actions and their status:

| Fix | Mechanism | Depth | Status |
|---|---|---|---|
| Type-1 verbatim extract ([AUTOFIX-EXTRACT]) | one shared method, rewrite sites | tree-sitter | specced; blocked on #42 |
| Call-site merge ([AUTOFIX-MERGE]) | anti-unification + default params | binding + types | this spec |
| Cross-file consolidation ([AUTOFIX-CONSOLIDATE]) | move canonical + delete dups + rewrite refs | import/symbol graph | this spec |
| Redirect to existing canonical | a fragment duplicating an *existing* named helper → replace with a call to it (Fowler *Replace Inline Code with Function Call*) | binding + types | spec'd here; after [AUTOFIX-MERGE] |
| Consolidate duplicate constant/literal | repeated literal → one shared `const`/`static`, refs updated | trivial | catalog (degenerate parameterise-by-constant) |
| Pull Up Method / Form Template Method | now-identical sibling methods → superclass; differing steps overridable (Hotta/Higo PDG; Fowler) | class hierarchy | catalog (OO-only; needs hierarchy model) |
| Identical import/using dedup | collapse duplicate import lines | trivial | catalog |

Rows below "this spec" are documented opportunities, not yet implemented; each ships with its own gate when scheduled.

## [AUTOFIX-MERGE-MCP] MCP tool surface

One new tool, cloning the `cluster-by-id` end-to-end shape ([mcp.md §MCP-TOOLS](mcp.md#mcp-tools)); naming + prompt engineering per [MCP-AGENT-PROMPT-GUIDANCE]:

| Tool | Inputs | Output | Description (prevention-first, ≤200 chars) |
|---|---|---|---|
| `merge-plan` | `{ cluster_id }` | `MergePlan` | Compute the mechanical merge for a cluster: parameterised helper, derived param names/types, per-site arguments, defaults, the `mechanical`/`ai_or_human` verdict, and the `WorkspaceEdit`. **Read-only — never writes files.** |

`MergePlan` is the only addition; the mechanical path has no AI round-trip, so no second tool is needed (the AI fallback keeps its `extract-method-plan` / `extract-method-apply` pair). The host applies the returned `WorkspaceEdit`. Wire type added to [live-ipc.td](../models/live-ipc.td); the request reuses `ClusterIdParams`.

## [AUTOFIX-MERGE-CODE-ACTION] LSP code action (IDE auto-fix)

`backend.rs` advertises `codeActionProvider { codeActionKinds: ["refactor.extract", "refactor.rewrite"], resolveProvider: true }` (LSP 3.17 lists `refactor.rewrite` for "add/remove parameter, move method"). Flow:

1. **Offer** (`textDocument/codeAction` at an occurrence range): a `CodeAction` literal with `edit` **omitted**, `{ kind: "refactor.rewrite", isPreferred: true, data: { cluster_id } }`.
2. **Resolve** (`codeAction/resolve`, only for the chosen action): compute the `WorkspaceEdit` — `documentChanges` of versioned `TextDocumentEdit`s (insert helper + rewrite each site; for `[AUTOFIX-CONSOLIDATE]`, plus `DeleteFile`/import rewrites), `changeAnnotations` labelling the groups. Lazy resolve keeps the inner loop cheap on a big repo.
3. **Preview**: the client renders the annotated affected-files tree + per-file diff; nothing is written until the user confirms (Eclipse LTK / JDT model; HCI: Murphy-Hill & Black; Vakilian et al.).
4. **Apply**: `failureHandling: transactional` (all-or-nothing); a single `label` so the whole N-file merge reverts in one undo; versioned ids reject a stale buffer.

The `refactor` logic lives in `deslop-core`; the LSP layer only assembles the `WorkspaceEdit`. AST subtrees for anti-unification come from new **in-process** `AnalysisSession` accessors (`subtree_at_range`, `source_bytes_for`) — never serialised to the wire (per [PRINCIPLES-AUDIENCE-AGENT]).

---

## [AUTOFIX-EXTRACT-AI] AI-assisted extraction — the fallback after [AUTOFIX-MERGE]

The mechanical paths above handle Type-1 ([AUTOFIX-EXTRACT]), leaf-gap Type-2 / constrained Type-3 ([AUTOFIX-MERGE]), and cross-file identical definitions ([AUTOFIX-CONSOLIDATE]) **with no AI**. The AI path is the **fallback** for the residue `[AUTOFIX-MERGE-GATE]` routes to `AiOrHuman`: clusters with **structural / control-flow drift** (gaps that are not confined to leaf positions), Type-4 semantic clones, or cases where a generalising parameter **name** materially aids readability. Even then the AI never writes code — it fills name slots in a Deslop-built scaffold (below), and Deslop synthesises the edit deterministically. (Renamed-identifier Type-2 is **not** a reason to invoke AI: `[AUTOFIX-MERGE]` derives parameter names mechanically; AI naming is only a readability polish.)

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
