# PR #310 — `feat: add Go language support` — Review

**Verdict: not slop — but do not merge as-is.**

This is genuinely careful work. The Go parser is correct, the normalisation
table is right, the AST golden is one of the more thorough ones in the repo,
and the `detection.rs` dedup refactor is behaviour-preserving. The problem is
that "add a language" in this repo is a ~12-site wiring task, and the PR
completed the Rust half and skipped most of the TypeScript half. **A `.go` file
will be analysed by the engine but will be invisible to almost every VS Code
surface.**

| | |
|---|---|
| PR | https://github.com/Nimblesite/Deslop/pull/310 |
| Author | lhaig (Lance Haig) |
| Head | `feat/go-language-support` @ `721ad640d` |
| Size | +712 / −204 across 38 files |
| **CI status** | **never run — `gh pr checks 310` → "no checks reported on the branch"** |
| Reviews | none |
| Merge state | BLOCKED |

Review method: 4 independent review lenses over the diff, then one adversarial
verifier per candidate finding tasked with *refuting* it. 16 candidates raised,
8 survived. The 8 that were killed are listed at the bottom so nobody re-litigates
them.

> **Note on the branch.** It is not checked out locally — the repo is on `main`
> with no `feat/go-language-support` ref. This review is against `gh pr diff 310`
> plus `main` as the merge base. **Nothing here has been compiled or executed.**

---

## 🔴 BLOCKER — Go is missing from the VSIX language registry

**[clients/vscode/src/types/languages.ts](clients/vscode/src/types/languages.ts)** — *not touched by this PR.*

This file is the VSIX's hand-maintained mirror of the core parser registry. Its
own header comment says it "Mirrors the core language set" and cites the
`[FACET-MODEL]` anti-drift rule (#170/#198). PR #310 adds Go to the Rust registry
and to `package.json` activation events, but never adds it here. All three maps
are stale:

- [`EXTENSION_LANGUAGE`](clients/vscode/src/types/languages.ts#L8-L22) — no `go: "go"`
- [`LANGUAGE_DISPLAY`](clients/vscode/src/types/languages.ts#L32-L42) — no `go: "Go"`
- [`ANALYSED_LANGUAGE_IDS`](clients/vscode/src/types/languages.ts#L63-L74) — no `"go"`

### Blast radius (each traced to a call site)

| Consequence | Call site |
|---|---|
| **LSP never syncs `.go` buffers** — `didOpen`/`didChange` are not sent, so the live loop (the product's headline feature) is dead for Go | [extension.ts:370](clients/vscode/src/extension.ts#L370) `documentSelector: ANALYSED_DOCUMENTS` |
| **No hover clone-card on Go** | [extension.ts:233](clients/vscode/src/extension.ts#L233) |
| **No inlay bubble on Go** | [bubble/live.ts:71](clients/vscode/src/bubble/live.ts#L71) |
| **Go clusters group under "Other"** in Top Offenders — `languageForPath` returns `"unknown"`, `languageDisplayName` falls through to `?? "Other"` | [tree/language.ts:32](clients/vscode/src/tree/language.ts#L32), [languages.ts:47](clients/vscode/src/types/languages.ts#L47) |
| **Report webview language filter has no Go option** — the `<select>` maps over `LANGUAGES` | [webview-ui/src/report/main.tsx:39](clients/vscode/webview-ui/src/report/main.tsx#L39) |

The bitter part: `onLanguage:go` **does** activate the extension, so the user gets
a Deslop that starts up on their Go repo and then does nothing in the editor.
Diagnostics may still appear (the server pushes those by URI), which makes the
inconsistency *more* confusing, not less.

This is precisely the #170/#198 → F#/PHP bug class the repo has now hit three
times. The PR fixed the identical drift on the **JetBrains** side (it adds the
long-missing `php`/`fs`/`fsx`) and then reproduced it on the VS Code side.

### Fix

```ts
// EXTENSION_LANGUAGE
go: "go",
// LANGUAGE_DISPLAY
go: "Go",
// ANALYSED_LANGUAGE_IDS
"go",
```

---

## 🟠 MAJOR — `vendor/` is not excluded, so Deslop is unusable on real Go repos

**[crates/deslop-core/src/config.rs:63](crates/deslop-core/src/config.rs#L63)** — `BUILTIN_EXCLUDE_COMPONENTS` — *not touched by this PR.*

```rust
const BUILTIN_EXCLUDE_COMPONENTS: &[&str] = &[
    "node_modules", "target", "dist", "build", ".venv", "__pycache__",
    ".cargo", ".git", ".claude", ".dart_tool", ".pub-cache",
];
```

That list is one entry per shipping language's in-repo dependency copy —
`node_modules` (JS), `.cargo` (Rust), `.venv`/`__pycache__` (Python),
`.dart_tool`/`.pub-cache` (Dart). Every prior language slice added its own.
Go's equivalent is `vendor/`, and it is missing.

**Why this bites harder than the others:** `go mod vendor` output is
*conventionally committed to git* (GitHub's `Go.gitignore` ships `vendor/`
commented out), and `vendor` is not dot-prefixed. So **neither** the `ignore`
crate's gitignore pass **nor** the hidden-dir pass prunes it. A vendored repo —
the norm in the Kubernetes/Docker/etcd ecosystem and most enterprise Go — hands
Deslop tens of thousands of third-party `.go` files to parse, fingerprint, embed
and rank.

Ranking is worst-offenders-first, so third-party library duplication the user
cannot act on will **outrank every genuine first-party finding**. The product's
headline output becomes unusable on exactly the large Go repos that most need it.

The repo already treats this failure mode as a showstopper class — see
`crates/deslop/tests/showstoppers.rs:187` ("Boilerplate clones in the vendored
cargo cache") and `crates/deslop-mcp/tests/wrong_root.rs` ("refuse to scan
vendored Cargo cache trees").

> Correction to one line of the reasoning here: the doc comment at config.rs:50-53
> claims the live watcher has no gitignore filter. That is stale — #287 added
> `ignore_matcher` (`pipeline/session/mod.rs:65-70`, consumed at
> `live/watcher.rs:154`). It does not rescue the case, because a *committed*
> `vendor/` is not gitignored anyway.

### Fix

Add `"vendor"` to `BUILTIN_EXCLUDE_COMPONENTS` with a justifying doc comment
matching the `.pub-cache` / `.cargo` entries above it.

---

## 🟡 MINOR

### 1. `go-type3` cannot fail — it is satisfied by plain Type-2 matching
[crates/deslop/tests/cli/detection.rs](crates/deslop/tests/cli/detection.rs) · fixtures `go-type3/{delta,epsilon}.go`

`detects_type3_clone_in_go_fixture` asserts only that *some* cross-file cluster
spans `delta.go`/`epsilon.go` at `structural: 1.0`. But the two files share two
**byte-identical-after-normalisation** subtrees, both above `--min-nodes 8`:

- the guard `if bound < 0 { return 0 }` → 9 nodes
- the `for _ := 0; _ <= _; _++` header → `for_clause` = 11 nodes

Either alone satisfies the assertion via ordinary Type-1/Type-2 matching. You
could delete Go's entire near-miss path and this test stays green. The test
comment claims it proves `[FUSION-SIGNALS-THREE-LAYER]` and the
`[CLONE-NOISE-SIGNATURE-ONLY]`/#154 suppression; it proves neither.

Compare the F# sibling it was modelled on — same weakness, but F# at least has
the signature-only sibling the comment describes.

### 2. Second hand-maintained extension map, also missing Go
[clients/vscode/src/commands/treeMenus.ts:275-291](clients/vscode/src/commands/treeMenus.ts#L275-L291)

A private `LANGUAGE_BY_EXT` + a module-local `languageForPath` — a straight
duplicate of `types/languages.ts`, in the repo whose entire purpose is detecting
this. It drives the fence tag for the **Copy Source Snippet** context action, so
Go occurrences copy out as an untagged ``` fence — unhighlighted and untyped when
pasted to a PR or an AI agent, defeating the mandated Copy-Context-For-AI surface.

Pre-existing (it is already missing `.php`/`.fs`/`.fsx`), but this PR is the third
consecutive language slice to walk past it. **Delete the duplicate and import the
shared registry.**

### 3. Marketplace keywords omit `go`
[clients/vscode/package.json:22-34](clients/vscode/package.json#L22-L34)

`keywords` carries one entry per shipping language and stops at `"fsharp"`. A Go
dev searching the Marketplace for "go duplicate code" will not find Deslop.
(`"php"` is missing too — same drift, one slice earlier.)

### 4. Site publishes "Go is not registered" — in two languages
- [site/src/docs/research-background.md:77](site/src/docs/research-background.md#L77) — verbatim: *"Go and other languages are not registered in the current core pipeline."* Also stale at `:91-92` (file index omits `go.rs`) and `:200` (evidence table).
- [site/src/zh/docs/research-background.md:78](site/src/zh/docs/research-background.md#L78) — *"Go 及其他语言尚未在当前核心流水线中注册"*, and `:201`.
- [site/src/blog/deduplicating-dart-code-ai-flutter.md:98](site/src/blog/deduplicating-dart-code-ai-flutter.md#L98) — FAQ answer ends *"Go is on the roadmap."* Same in the zh mirror.

The PR updated `index.njk`, `how-it-works.md` and `ai-integration.md` (+zh) but
missed these. After merge the homepage advertises Go while the page whose stated
purpose is *auditor verification against the code* denies it. That blog FAQ line is
also the site's only remaining direct sentence about Go support — i.e. the one an
AI search summariser will quote.

### 5. `LANG-ROADMAP.md` says Go is both shipped and unshipped
[docs/plans/LANG-ROADMAP.md:274](docs/plans/LANG-ROADMAP.md#L274)

The PR adds "P-LANG-3 — Go (COMPLETE)" and removes Go from *Remaining*, but leaves
`### [LANG-CAND-GO] Go — PRIORITY 4 (low-hanging)` and *"Estimate. 0.5 day. The
easiest language to add."* The file's convention for shipped languages is to
retitle them `— ✅ SHIPPED (=x.y.z)`.

CLAUDE.md points agents at this file to pick the next language. A future agent
greps `^### \[LANG-CAND-`, sees Go sitting above Java at PRIORITY 4, and starts
implementing `tree-sitter-go` a second time — the exact duplicated-work failure
this repo exists to prevent.

---

## 🧪 Tests — the real gap

The PR ships four Go E2E tests. Every one of them is a copy of the sibling-language
template, and the template is thinner than this repo's stated bar. Meanwhile the
*new production branches* this PR adds have **zero** coverage.

### Confirmed-untested production code added by this PR

| Added code | Test that exercises it |
|---|---|
| `func_literal` in `function_kinds(b"go")` ([cluster_filters/mod.rs:277](crates/deslop-core/src/cluster_filters/mod.rs#L277)) | **none** — no Go fixture in the PR contains a closure |
| `go_carrier`'s `import_declaration` arm ([boilerplate.rs:259](crates/deslop-core/src/boilerplate.rs#L259)) | **none** — all six clustering fixtures have `package X` and zero imports. The AST golden *has* imports but `debug_ast_dump` never runs the boilerplate filter (`pipeline/run.rs:54-73`, "not part of the analysis pipeline") |
| `go_carrier`'s `package_clause` arm | same |
| The `[CLONE-NOISE-SIGNATURE-ONLY]`/#154 suppression the `go-type3` comment claims to prove | **none** — unreachable at `--min-nodes 8` |

Both `func_literal` and `go_carrier` are *correct* — verified against the
tree-sitter-go grammar. `func_literal` exposes a `body` field, and including it is
genuine parity with the JS/TS arm's `function_expression`. `is_import_boilerplate_only_subtree`
returns on the carrier node without descending, so matching only the outer
`import_declaration` is right. **The code is fine; it is just unproven.** In a repo
that mandates E2E proof per feature and ~95% coverage floors, shipping three
untested new branches is the thing to fix.

### Tests to add before merge

**Blocker guard — VSIX registry parity.** The existing guard
([analysedLanguages.unit.test.ts](clients/vscode/src/test/unit/analysedLanguages.unit.test.ts))
hardcodes `fsharp`/`php`, so it passes with Go missing and will pass with Java
missing. Extend it for Go now, and separately open an issue to make it
registry-derived (the TS side has no language manifest to import, and regex on
source is banned — so this needs a generated artifact, not a quick fix):

```ts
test("Go editor id is analysed (hover + inlay + LSP sync attach)", () => {
  assert.ok(ANALYSED_LANGUAGE_IDS.includes("go"));
});
test("Go source extension resolves to its language id", () => {
  assert.equal(languageForPath("/repo/main.go"), "go");
});
test("Go carries a human display name", () => {
  assert.equal(languageDisplayName("go"), "Go");
});
```

**`vendor/` exclusion — E2E.** New fixture `go-vendored/` with a first-party
`main.go` clone pair plus `vendor/example.com/dep/dep.go` containing an obvious
clone. Assert: `files_analysed` counts only the first-party files, and no cluster
occurrence path contains `/vendor/`. Model it on `showstoppers.rs:187`.

**Make `go-type3` actually test Type-3.** Rewrite the fixture so *no* subtree is
identical above `--min-nodes 8` — vary the guard (`if bound < 0` vs
`if limit <= 0`) and the loop form (three-clause `for` vs `for … range`) so the
only cross-file match is a genuine near-miss. Then tighten the assertion beyond
"a cluster exists": assert the `token_jaccard` value sits in a near-miss band, not
merely that the key is present.

**`func_literal` — E2E.** Fixture `go-closure-signature-only/` with two files each
declaring `func f(_ int) int` returning a closure with *differing* bodies. Assert
the signature-only match is suppressed and the bodies do not cluster. This is the
only thing that will make deleting the `func_literal` entry turn the suite red.

**`go_carrier` — E2E.** Give at least one clustering fixture a real grouped
`import ( … )` block plus a `package` clause, with enough shared import shape that
an unfiltered run *would* cluster them. Assert no cluster occurrence lands inside
the import block. Today the arm is dead weight from the suite's perspective.

**Go's actual identity is untested.** Nothing anywhere in this PR — golden or
fixture — exercises `go` statements, `defer`, `select`, channel send/receive,
`range` clauses, type switches, variadics, generics (`type_parameter_list`),
interface types, or struct tags. Struct tags are the interesting one: a raw string
literal used as a *tag* rather than a value. At minimum, extend
`ast-golden-go/Sample.go` to cover them so a grammar bump cannot silently change
their normalisation.

**Assertion strength in the shared helpers.** `report_clusters` uses
`.unwrap_or_default()`, so a report with **zero** clusters makes
`assert_every_cluster_single_file` pass vacuously — for Go, Dart, Python *and* F#.
The refactor did not introduce this (the inline code did the same), but it now sits
in one place where a single `assert!(!clusters.is_empty(), …)` fixes it for four
languages at once. Do it.

---

## ✅ What is solid

Credit where due — I checked these and they hold up:

- **The normalisation table is correct.** Every kind in `normalise_kind` exists as a *named* node in tree-sitter-go 0.25.0. `nil`/`true`/`false`/`iota` really are named grammar nodes (not bare identifiers), which is the easy thing to get wrong here.
- **The AST golden is unusually thorough** — labels, `iota`, rune/imaginary/raw-string literals, escape sequences, qualified types, blank identifier, expression switch, composite literal. Better than several existing goldens.
- **The core registry claim is true.** `language_ids()`, `language_for_path()` and `source_extensions()` all derive from `default_parsers()`, so discovery, the live extension map and the MCP `language` enum genuinely pick Go up for free. The `corpus.rs` change is a `use` block + the `vec!` — both correct, not two registries.
- **`func_literal` is the right call** — parity with the JS/TS `function_expression` arm, and `function_name_node` degrades to `None` gracefully rather than panicking.
- **The `detection.rs` refactor is behaviour-preserving.** I diffed the assertions: none removed, and the F#/Dart guards actually *gain* the `cluster_id` in their failure message. 482 → 471 lines while adding three tests.
- **The pin is current** — `tree-sitter-go 0.25.0` is the latest release on crates.io.
- **The JetBrains drift fix is a real improvement** (`php`/`fs`/`fsx` had silently drifted from the site docs' claims).

---

## Refuted — do not chase these

The adversarial pass killed 8 candidates. Recorded so they do not get re-raised:

| Candidate | Why it died |
|---|---|
| Go string literals nest a variable number of `__literal__` children, breaking Type-2 invariance across escape sequences | **Real, but repo-wide and pre-existing.** The committed JS golden shows the identical `__literal__ → __literal__` nesting. Go's `*_content`/`escape_sequence` handling is exactly the claimed ECMAScript parity. Not a Go divergence. |
| `normalise_kind` is 25 lines, breaking the <20-line rule | It is a verbatim copy of `fsharp.rs:90-111`. The repo has ~265 production functions over 20 lines, no `clippy::too_many_lines`, no gate. Not enforced for rustfmt-exploded `matches!` alternations. |
| `Cargo.lock` windows-sys 0.60.2 → 0.61.2 is unrelated churn | In-range for every dependent; both versions already existed pre-PR (`mio` was on 0.61.2); this *reduces* fragmentation 8/1 → 1/8. `windows-sys` is transitive with no pin obligation. CI's windows-latest `cargo check --locked` would catch breakage. |
| The `ANALYSED_LANGUAGE_IDS` guard is hardcoded and cannot fail | True, but it double-counts the blocker above — same root cause, no independent failure mode. Folded into the blocker's fix. |
| `boilerplate_hints_use_default_recommendation_for_future_languages` still uses `"go"` as its unsupported stand-in | Untouched by the PR; if it ever breaks it fails loudly and gets retargeted, exactly as `live.rs` did in this PR. |

---

## Recommended disposition

**Request changes.** Required before merge:

1. Add Go to `clients/vscode/src/types/languages.ts` (all three maps) + the three guard tests.
2. Add `"vendor"` to `BUILTIN_EXCLUDE_COMPONENTS` + the `go-vendored` E2E.
3. Fix `go-type3` so it can actually fail.
4. Cover `func_literal` and `go_carrier` with real fixtures.
5. Fix the two `research-background.md` files, the blog FAQ (+zh), and the `LANG-ROADMAP.md` heading.
6. **Get CI to run.** It has never run on this branch. The author states `make ci` passed locally but also that no JVM was available, so `DeslopSupportedFilesTest` — which this PR modifies — is unverified everywhere. The JetBrains CI job does exist and does run it (`ci.yml:334-393`), so this is purely a matter of triggering the workflow.

Deferrable to follow-up issues: the `treeMenus.ts` duplicate map, marketplace keywords, the registry-derived parity harness, and Go-idiom golden coverage (`defer`/`select`/channels/generics/struct tags).
