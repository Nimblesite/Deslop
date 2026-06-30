# Language Expansion Roadmap

> Scope: plan the order in which Deslop picks up new source languages beyond
> the v1 set (C#, Rust, Python). TypeScript is the top priority; JavaScript
> rides with it if the grammar permits. Dart is desired but gated on grammar
> quality. Remaining slots go to low-hanging, high-impact languages whose
> tree-sitter grammars are already production-grade.

Research conducted 2026-04-23. Version/status rows reflect crates.io +
docs.rs + GitHub state at that date.

---

## [LANG-ROADMAP-CONSTRAINTS] Non-negotiable constraints

1. **Grammar must be tree-sitter.** Regex on source is illegal per
   [CLAUDE.md](../../CLAUDE.md). No hand-rolled or ANTLR shims.
2. **Grammar must be on crates.io** as a Rust crate. No vendored
   `parser.c` blobs, no git-dep grammars.
3. **Grammar must pin an exact `=x.y.z` version.** The CI drift check in
   [.github/workflows/ci.yml](../../.github/workflows/ci.yml) grep-asserts
   this for every `tree-sitter-*` dependency in `Cargo.toml`.
4. **Grammar must handle the language as it's actually written in 2026.**
   Dart 3 records/patterns, TS 5.x satisfies/const-type-params, Kotlin
   context receivers — if the grammar is a 2022 snapshot it's a no-go.
5. **One `LanguageParser` impl per language, following the existing
   shape** ([python.rs](../../crates/deslop-core/src/lang/python.rs) is
   the canonical reference). `normalise_kind` is the only per-language
   logic; everything else lives in
   [`lang::shared`](../../crates/deslop-core/src/lang/shared.rs).
6. **No linter suppressions, no `unwrap`, functions < 20 lines,
   files < 500 lines** — per [CLAUDE.md](../../CLAUDE.md).

---

## [LANG-ROADMAP-RUNTIME-UPGRADE] The tree-sitter 0.22 → 0.26.8 upgrade

P-LANG-0 upgraded the workspace from `tree-sitter = "=0.22.6"` to
`tree-sitter = "=0.26.8"` and migrated the current language modules from
`tree_sitter_<x>::language()` to each grammar's `LANGUAGE` constant.
From 2024 onwards, new tree-sitter grammars depend on the
**`tree-sitter-language ^0.1`** shim crate and expose a `LANGUAGE`
constant (a `LanguageFn`) convertible to `tree_sitter::Language` via
`.into()`. The shim is stable across `tree-sitter` 0.24 / 0.25 / 0.26,
so a grammar declaring `tree-sitter ^0.25` (in its `dev-dependencies`,
for its own tests) loads cleanly against the **0.26.8 runtime now
targeted by Deslop**.

Latest stable runtime as of 2026-04-23: **`tree-sitter = 0.26.8`**
(released 2026-03-31, eight patch releases past 0.26.0). The upgrade
plan is complete; this roadmap keeps the durable baseline.

| Grammar                      | Latest version | Shim       | Loads on 0.26.8 |
|------------------------------|----------------|------------|-----------------|
| tree-sitter-c-sharp (pinned) | 0.21.3         | *(old API)*| ❌ rev to 0.23.5 |
| tree-sitter-rust (pinned)    | 0.21.2         | *(old API)*| ❌ rev to 0.24.2 |
| tree-sitter-python (pinned)  | 0.21.0         | *(old API)*| ❌ rev to 0.25.0 |
| tree-sitter-c-sharp (new)    | 0.23.5         | ^0.1       | ✅               |
| tree-sitter-rust (new)       | 0.24.2         | ^0.1       | ✅               |
| tree-sitter-python (new)     | 0.25.0         | ^0.1       | ✅               |
| tree-sitter-typescript       | 0.23.2         | ^0.1       | ✅               |
| tree-sitter-javascript       | 0.25.0         | ^0.1       | ✅               |
| tree-sitter-go               | 0.25.0         | ^0.1       | ✅               |
| tree-sitter-java             | 0.23.5         | ^0.1       | ✅               |
| tree-sitter-cpp              | 0.23.4         | ^0.1       | ✅               |
| tree-sitter-c                | 0.24.2         | ^0.1       | ✅               |
| tree-sitter-ruby             | 0.23.1         | ^0.1       | ✅               |
| tree-sitter-bash             | 0.25.1         | ^0.1       | ✅               |
| tree-sitter-php              | 0.24.2         | ^0.1       | ✅               |
| tree-sitter-dart (nielsenko) | 0.2.0          | ^0.1       | ✅ SHIPPED (=0.2.0) |
| tree-sitter-swift            | 0.7.1          | *(ts ^0.23)* | ⚠ generated-file workaround |
| tree-sitter-kotlin (fwcd)    | 0.3.8          | *(ts 0.21–0.22)* | ❌ incompatible |

**Implication.** TypeScript (and Go / Java / C++ / Ruby / Bash / PHP /
Dart) can now build on the modern `LanguageFn` grammar surface.
Kotlin is still stuck on the old runtime and Swift has a build-script
caveat — both deferred per [LANG-DECISIONS].

---

## [LANG-ROADMAP-SCORING] How we rank candidates

Four signals:

1. **Demand in AI-coding workloads.** LLMs emit TypeScript / JS / Python /
   Go / Java at roughly an order of magnitude above Swift / Kotlin / Dart.
   Deslop's job is catching the copy-paste slop these agents produce, so
   demand dominates.
2. **Grammar maturity.** Real-world corpus pass rate, release cadence,
   whether the parser is "the" grammar (`tree-sitter` org) or a fork.
3. **Normalisation complexity.** How many node kinds we have to classify
   as identifier / literal / trivia. TypeScript and C++ are the hardest
   because of templates/generics and overload resolution; Go and Bash
   are the easiest.
4. **Feature drift risk.** How often does the language change? TS ships
   quarterly; Go is glacial.

Weighted combination: `demand * 0.5 + maturity * 0.3 + (1/complexity) * 0.1
+ (1/drift) * 0.1`. Worst-offender-first ranking, same philosophy as
the report output.

---

## [LANG-ROADMAP-CANDIDATES] Per-language findings

### [LANG-CAND-TYPESCRIPT] TypeScript — ✅ SHIPPED (=0.23.2)

- **Crate.** `tree-sitter-typescript = "=0.23.2"` (tree-sitter org).
- **Runtime.** `tree-sitter ^0.24` — forces [LANG-ROADMAP-RUNTIME-UPGRADE].
- **Exports TWO languages.** `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX`.
  `.ts` files get the former, `.tsx` files the latter. We ship two
  `LanguageParser` impls sharing the same `normalise_kind` but differing
  in `id()` / `file_extensions()`. Both map to the same `language_id`
  for cross-language comparison purposes — TSX is a strict superset.
- **Normalisation knowns.** `identifier`, `type_identifier`,
  `property_identifier`, `shorthand_property_identifier_pattern` →
  `__ident__`. `string`, `template_string`, `number`, `regex`, `true`,
  `false`, `null`, `undefined` → `__literal__`. `comment` dropped.
  Type annotations are structural — keep them.
- **Risk.** Grammar evolves quarterly alongside TS releases. Grammar pin
  drift check already catches this.
- **Fixtures.**
  - `tests/fixtures/typescript-small/{alpha,beta}.ts` — Type-2 renamed
    clone pair (Promise chain + arrow fn).
  - `tests/fixtures/typescript-type3/{delta,epsilon}.ts` — Type-3 hole
    (extra log line).
  - `tests/fixtures/tsx-small/{Card,Tile}.tsx` — JSX component clone.
  - `tests/fixtures/ast-golden-typescript/Sample.ts(x).expected.ast` —
    byte-for-byte golden dumps (both variants).
- **Estimate.** 2–3 days including the runtime upgrade. TS alone ~1 day
  once the upgrade lands.

### [LANG-CAND-JAVASCRIPT] JavaScript — ✅ SHIPPED (=0.25.0)

- **Crate.** `tree-sitter-javascript = "=0.25.0"` (tree-sitter org).
- **Runtime.** `tree-sitter ^0.25`. Fully compatible with the upgrade.
- **Option A — separate plugin.** One more `LanguageParser` impl,
  `normalise_kind` nearly identical to TS minus the type-annotation
  nodes.
- **Option B — reuse `tree-sitter-typescript` for `.js`.** The TS
  grammar accepts JS as a subset; untyped `.js` parses cleanly. Cuts a
  dependency. Downside: TS-only node kinds leak into the
  `normalise_kind` match and we carry keywords JS doesn't have.
- **Recommendation.** **Option A.** Keeping JS on `tree-sitter-javascript`
  is cleaner per-language — it's what the tree-sitter org maintains for
  this exact purpose and it keeps each `normalise_kind` honest about the
  grammar it's matching. `.js` / `.mjs` / `.cjs` / `.jsx` extensions.
  JSX uses the same grammar (JS grammar has a `jsx` extras field).
- **Fixtures.** `tests/fixtures/javascript-small/{alpha,beta}.js` +
  AST golden.
- **Estimate.** 0.5 day on top of TS.

### [LANG-CAND-DART] Dart — PRIORITY 3 (gated on grammar audit)

- **Crate.** Two real candidates:
  - `tree-sitter-dart = "0.1.0"` (nielsenko fork) — Dart 3.11 support,
    records, patterns, class modifiers, extension types, null-aware
    elements. Claims 100% pass on the official `dart-lang/language`
    corpus (4,135 files from pub.dev). Ships Rust bindings on
    `tree-sitter ^0.25`. **50% documented** per docs.rs, which is a
    yellow flag, and 0 releases on GitHub (only the crate publish).
  - `tree-sitter-dart` (UserNobody14) — older community grammar, 17
    open issues, no recency signal. Don't use.
- **Recommendation.** **Build a spike first, don't commit.** Before
  committing the plugin to `deslop-core`, run a 1-day spike:
  1. Parse a real Flutter app repo (e.g. flutter/gallery) end-to-end
     and assert zero `ERROR` nodes.
  2. Verify Dart 3 patterns (`switch (x) { (a, b) => ... }`) and
     records (`(int, int)`) round-trip.
  3. Verify null-aware operators and nullable types.
  4. Confirm the `LanguageFn` surface matches `tree-sitter ^0.25`
     without binding-version hacks.
- **If the spike fails any of those:** park Dart. The user explicitly
  flagged this — "if the tree sitter is not very good, we may need to
  pause this one." Spec-ID `[LANG-CAND-DART-PARKED]` in the TODO log.
- **Fixtures (if we proceed).** `tests/fixtures/dart-small/{alpha,beta}.dart`,
  `tests/fixtures/dart-type3/{delta,epsilon}.dart`, AST golden.
- **Estimate.** 1 day spike + 1 day implementation if green.

### [LANG-CAND-GO] Go — PRIORITY 4 (low-hanging)

- **Crate.** `tree-sitter-go = "=0.25.0"` (tree-sitter org). 100%
  documented. `tree-sitter ^0.25`. Rock solid — Go's grammar changes
  rarely and the tree-sitter grammar has been production-grade for
  years (used by GitHub semantic, Zed, Helix).
- **Normalisation.** Trivial. `identifier`, `field_identifier`,
  `type_identifier`, `package_identifier` → `__ident__`.
  `interpreted_string_literal`, `raw_string_literal`, `int_literal`,
  `float_literal`, `imaginary_literal`, `rune_literal`, `true`, `false`,
  `nil` → `__literal__`. `comment` dropped.
- **Fixtures.** `tests/fixtures/go-small/{alpha,beta}.go` + AST golden.
- **Estimate.** 0.5 day. The easiest language to add.

### [LANG-CAND-JAVA] Java — PRIORITY 5

- **Crate.** `tree-sitter-java = "=0.23.5"` (tree-sitter org).
  `tree-sitter ^0.24`. 100% documented. Canonical grammar.
- **Normalisation.** Straightforward. Records (Java 16+), sealed classes
  (Java 17+), pattern matching for switch (Java 21+) all covered.
- **Fixtures.** `tests/fixtures/java-small/{Alpha,Beta}.java` + AST golden.
- **Estimate.** 1 day. Slightly more node kinds than Go (annotations,
  generics, switch expressions) but nothing surprising.

### [LANG-CAND-CPP] C++ — PRIORITY 6

- **Crate.** `tree-sitter-cpp = "=0.23.4"` (tree-sitter org). `^0.24`.
  Depends on `tree-sitter-c`.
- **Caveat.** Templates and overload resolution make the grammar harder
  to normalise without losing signal. Identifier collapse across
  template parameters is fine; across operator overloads is fine;
  across `requires` clauses and concepts needs a careful
  `normalise_kind`. Budget for a bigger normaliser.
- **Estimate.** 1.5 days.

### [LANG-CAND-C] C — PRIORITY 7 (free rider on C++)

- **Crate.** `tree-sitter-c = "=0.24.2"` (tree-sitter org). `^0.25`.
  Already pulled in transitively by tree-sitter-cpp.
- **Estimate.** 0.5 day after C++ lands.

### [LANG-CAND-RUBY] Ruby — PRIORITY 8

- **Crate.** `tree-sitter-ruby = "=0.23.1"` (tree-sitter org). `^0.24`.
  Mature grammar, used by GitHub.
- **Normalisation.** Ruby has more syntactic sugar than the others.
  Symbols, blocks, heredocs all need attention. `identifier`,
  `constant`, `instance_variable`, `class_variable`, `global_variable`
  → `__ident__`.
- **Estimate.** 1 day.

### [LANG-CAND-PHP] PHP — PRIORITY 9

- **Crate.** `tree-sitter-php = "=0.24.2"` (tree-sitter org). `^0.24`.
  Exposes both `LANGUAGE_PHP` (full PHP with HTML) and
  `LANGUAGE_PHP_ONLY` (pure PHP, no HTML interleaving). We use
  `LANGUAGE_PHP_ONLY` — `.php` files in agent-generated code are
  overwhelmingly pure PHP and the HTML-interleaved mode introduces
  noisy `text` nodes we'd have to special-case.
- **Estimate.** 1 day.

### [LANG-CAND-BASH] Bash — PRIORITY 10

- **Crate.** `tree-sitter-bash = "=0.25.1"` (tree-sitter org). `^0.25`.
- **Value.** Agents generate a LOT of shell. CI scripts, Dockerfile
  RUN blocks, setup.sh files — all prime copy-paste territory.
- **Estimate.** 0.5 day.

### [LANG-CAND-SWIFT] Swift — DEFERRED

- **Crate.** `tree-sitter-swift = "0.7.1"` (alex-pinkus fork; not
  tree-sitter org). `^0.23`. Release June 2025, so active.
  Deliberately ships without generated `parser.c`/`grammar.json` — users
  must either regenerate via the tree-sitter CLI or fetch the CI
  artifact. That's an extra step we'd have to automate in our build
  script. Docs coverage is ~43%.
- **Verdict.** Valuable (iOS agent code) but the build story is
  awkward. Defer until after the easy wins.

### [LANG-CAND-KOTLIN] Kotlin — DEFERRED

- **Crate.** `tree-sitter-kotlin = "=0.3.8"` (fwcd fork).
  Runtime `>=0.21, <0.23` — **incompatible with the 0.25 upgrade**.
- **Maturity.** The README admits 61.2% structural-match rate vs. the
  JetBrains compiler reference parser, with a `TODO.md` of known
  grammar gaps. That's below our bar.
- **Verdict.** Wait for a 0.24+ compatible grammar with better corpus
  parity. Revisit in 6 months.

### [LANG-CAND-REJECTED] Languages we explicitly skip

- **Scala, Haskell, OCaml, Elixir, Clojure, F#, Nim, Zig** — low demand
  in AI-coding workloads relative to the implementation cost.
- **SQL, HTML, CSS, Markdown, YAML, JSON, TOML** — structural but not
  "code" in the duplicate-detection sense. Clone reports on YAML are
  noise.

---

## [LANG-DECISIONS] Decisions baked into this plan

1. **TypeScript and JavaScript are separate `LanguageParser` impls on
   separate grammars.** TSX is a second TS impl sharing
   `normalise_kind`. See [LANG-CAND-JAVASCRIPT] for the rejected
   merge-into-one-grammar alternative.
2. **The tree-sitter runtime gets upgraded to `=0.25.x` before any new
   language lands.** Alternative — back-port grammars to 0.22 — is not
   feasible; most modern grammars don't publish 0.22-compatible
   versions and the legacy `::language()` API is gone.
3. **Dart is gated on a spike, not a commitment.** User flagged the
   risk explicitly. Spike task is `[LANG-CAND-DART-SPIKE]`; if it
   fails, Dart parks until a better grammar lands.
4. **Kotlin is deferred** because its grammar is stuck on tree-sitter
   0.21–0.22 and its corpus parity is 61%.
5. **Swift is deferred** because the grammar requires a generated-file
   workaround in the build script.

---

## [LANG-EXECUTION] Phased execution

All phases follow the existing PLAN.md shape: each bullet produces
code + e2e fixture + AST golden + grammar pin in `Cargo.toml`,
`.github/workflows/ci.yml`, and `.devcontainer/`.

### Phase P-LANG-0 — tree-sitter runtime upgrade (COMPLETE, CI GREEN)

- [x] Bump `tree-sitter = "=0.26.8"` in workspace `Cargo.toml`.
- [x] Migrate `csharp.rs`, `rust_lang.rs`, `python.rs` to newer grammar
      versions that target the modern `LanguageFn` surface:
      - `tree-sitter-c-sharp` → `=0.23.5`.
      - `tree-sitter-rust` → `=0.24.2`.
      - `tree-sitter-python` → `=0.25.0`.
      Rust AST goldens were refreshed for the new
      `lifetime_parameter` / `type_parameter` wrappers; C# and Python
      stayed stable.
- [x] Audit `lang::shared::parse_source` against the 0.26 API.
      `Parser::set_language` still takes `&Language`, so the shared
      signature remains unchanged.
- [x] Re-run validation. `make test` passes and Rust-side coverage rose
      from 96.0% to 96.1%. Full `make ci` is green after the follow-up
      VSIX coverage push: VSIX line coverage is 90.11% against the
      ratcheted 90% threshold.
- [x] Verify grammar-pin-drift check still accepts exact `=x.y.z`
      runtime and grammar pins; no CI regex edit required.

### Phase P-LANG-1 — TypeScript + TSX — COMPLETE

- [x] Add `tree-sitter-typescript = "=0.23.2"` (or newer if a 0.25-compat
      release ships).
- [x] `crates/deslop-core/src/lang/typescript.rs` — two impls:
      `TypeScriptParser` (id `"typescript"`, exts `["ts"]`) and
      `TsxParser` (id `"tsx"`, exts `["tsx"]`). Both call the shared
      `normalise_kind` defined in the shared ECMAScript module.
- [x] Fixtures: `tests/fixtures/typescript-small/`,
      `tests/fixtures/tsx-small/`, `tests/fixtures/typescript-type3/`.
- [x] AST goldens for both grammars.
- [x] Activation in `clients/vscode/package.json`:
      `onLanguage:{typescript,typescriptreact}` plus `.ts` / `.tsx`
      `workspaceContains` entries.

### Phase P-LANG-2 — JavaScript — COMPLETE

- [x] Add `tree-sitter-javascript = "=0.25.0"`.
- [x] `crates/deslop-core/src/lang/javascript.rs` — one impl, exts
      `["js", "mjs", "cjs", "jsx"]`.
- [x] Fixture + golden.
- [x] VS Code activation: `onLanguage:{javascript,javascriptreact}` plus
      `.js` / `.mjs` / `.cjs` / `.jsx` `workspaceContains` entries.

### Phase P-LANG-3 — Go

- [ ] Add `tree-sitter-go = "=0.25.0"`.
- [ ] `crates/deslop-core/src/lang/go.rs`.
- [ ] Fixture + golden.

### Phase P-LANG-4 — Dart (SPIKE FIRST) — COMPLETE, SPIKE GREEN

- [x] `[LANG-CAND-DART-SPIKE]` — grammar audit per [LANG-CAND-DART].
      Outcome documented under `[LANG-CAND-DART-RESULT]`.
- [x] Spike passed: added `tree-sitter-dart = "=0.2.0"`, implemented
      [`dart.rs`](../../crates/deslop-core/src/lang/dart.rs), shipped
      fixtures (`dart-small`, `dart-type3`, `dart-dissimilar-functions`,
      `ast-golden-dart`) and e2e tests.
- [x] `[LANG-CAND-DART-PARKED]` not needed — the grammar cleared every
      spike gate.

### [LANG-CAND-DART-RESULT] Spike outcome (2026-05-30) — GREEN

The nielsenko `tree-sitter-dart` grammar shipped **`0.2.0`** on
2026-04-26 (after the 2026-04-23 research above), superseding the
`0.1.0` the roadmap evaluated. `0.2.0` declares `tree-sitter ^0.26` in
its dev-dependencies and exposes the modern `LANGUAGE` (`LanguageFn`)
constant via `tree-sitter-language ^0.1`, so it loads natively against
Deslop's `=0.26.8` runtime (ABI v15, 483 node kinds) — no
binding-version hacks, removing the `0.1.0` yellow flag.

Spike gates (all passed against an isolated `tree-sitter 0.26.8`
harness):

1. **Zero `ERROR`/`MISSING` nodes** across 11 Dart-3 samples: records
   (`(int, int)`), every pattern form (record / list / map / variable /
   constant / wildcard / rest, plus `when` guards), `sealed` / `base` /
   `final` class modifiers, extension types, enhanced enums, typedefs,
   all nine string-quote variants (single/double/triple/raw +
   interpolation), getters/setters/operators/factory ctors, cascades
   (`..`), spreads (`...`), collection-`if`/`for`, generic bounds, and
   `async*` / `sync*` generators.
2. **Records and patterns round-trip** structurally (verified in the
   AST golden `Sample.dart`).
3. **Null-aware operators / nullable types** (`?.`, `??`, `T?`,
   null-aware elements `[?x]`) parse cleanly.
4. **`LanguageFn` surface matches `tree-sitter ^0.26`** with no
   workaround.

`normalise_kind` was derived from the full 221-kind named-and-visible
grammar vocabulary (not just the sampled subset): identifier leaves
(`identifier`, `identifier_dollar_escaped`, `type_identifier`) collapse
to `__ident__`; every numeric/boolean/`null`/symbol literal and every
string-quote variant plus its `template_chars_*` text chunks collapse
to `__literal__` (so `'x'` and `"x"` fingerprint identically while
`template_substitution` interpolation expressions stay structural);
`comment` / `block_comment` / `documentation_block_comment` drop.
End-to-end: Type-2 renamed clones reach `structural = 1.0` and
`token_jaccard = 1.0`; a whole-function near-miss yields a cross-file
cluster with `token_jaccard > 0`; structurally-unrelated functions
never cluster across files.

### Phase P-LANG-5 — Java

- [ ] `tree-sitter-java = "=0.23.5"` + plugin + fixtures.

### Phase P-LANG-6 — C / C++

- [ ] `tree-sitter-cpp = "=0.23.4"` + `tree-sitter-c = "=0.24.2"` +
      both plugins + fixtures.

### Phase P-LANG-7 — Ruby

### Phase P-LANG-8 — PHP

### Phase P-LANG-9 — Bash

### (Deferred) Swift, Kotlin

---

## [LANG-PER-LANG-CHECKLIST] Every new language must ship

A new-language PR is not done until all of the following are green:

- [ ] Grammar crate added with exact `=x.y.z` pin in
      `Cargo.toml`, `.github/workflows/ci.yml`, `.devcontainer/`.
- [ ] `LanguageParser` impl in `crates/deslop-core/src/lang/<name>.rs`
      (< 100 LOC; `normalise_kind` is the only language-specific logic;
      all shared plumbing reused from `lang::shared`).
- [ ] Registered in the pipeline's parser registry (`lang::mod`
      re-export + wherever the default set is assembled).
- [ ] File-extension filter contributes to the discovery stage.
- [ ] E2E Type-2 fixture (renamed clone) — cluster asserted in report.
- [ ] E2E Type-3 fixture where structural sim < 1.0 and token jaccard > 0.
- [ ] AST-golden fixture under
      `tests/fixtures/ast-golden-<name>/Sample.<ext>.expected.ast`
      with byte-for-byte equality test. Grammar bumps must trip this.
- [ ] Boilerplate classification documented for imports / module
      headers per [pipeline.md §PIPELINE-BOILERPLATE](../specs/pipeline.md).
- [ ] VS Code extension `package.json` activation event
      (`onLanguage:<id>` + `workspaceContains:**/*.<ext>`).
- [ ] README / site docs mention the language in the supported-set
      list.
- [ ] Coverage threshold in `coverage-thresholds.json` does not drop.

---

## [LANG-OPEN-QUESTIONS] Things still to decide

- **Cross-language clone comparison scope.** Current default per
  [CONFIG-CROSS-LANGUAGE] is language-scoped. Should TS↔JS compare by
  default (they share a grammar family)? Proposal: yes, opt-out via
  `.deslop.toml`. Same question for C↔C++. Decide before P-LANG-2 ships.
- **TS type-annotation nodes — structural or trivia?** Initial
  recommendation is structural (a method with a type annotation is
  genuinely different from one without). Revisit after fixtures expose
  false positives.
- **JSX component name identifiers** — currently collapse to
  `__ident__`. That means `<Card />` and `<Tile />` fingerprint identical,
  which is usually what we want for Type-2 detection but may over-merge
  on component libraries. Flag to revisit with real-world React
  corpora.

---

*Research sources consulted 2026-04-23:*

- [tree-sitter-typescript on docs.rs](https://docs.rs/tree-sitter-typescript)
- [tree-sitter-javascript on docs.rs](https://docs.rs/tree-sitter-javascript)
- [tree-sitter-go on docs.rs](https://docs.rs/tree-sitter-go)
- [tree-sitter-java on docs.rs](https://docs.rs/tree-sitter-java)
- [tree-sitter-cpp on docs.rs](https://docs.rs/tree-sitter-cpp)
- [tree-sitter-c on docs.rs](https://docs.rs/tree-sitter-c)
- [tree-sitter-ruby on docs.rs](https://docs.rs/tree-sitter-ruby)
- [tree-sitter-bash on docs.rs](https://docs.rs/tree-sitter-bash)
- [tree-sitter-php on docs.rs](https://docs.rs/tree-sitter-php)
- [tree-sitter-swift on docs.rs](https://docs.rs/tree-sitter-swift)
- [tree-sitter-kotlin on docs.rs](https://docs.rs/tree-sitter-kotlin)
- [tree-sitter-dart on docs.rs](https://docs.rs/tree-sitter-dart)
- [nielsenko/tree-sitter-dart on GitHub](https://github.com/nielsenko/tree-sitter-dart)
- [UserNobody14/tree-sitter-dart on GitHub](https://github.com/UserNobody14/tree-sitter-dart)
- [fwcd/tree-sitter-kotlin on GitHub](https://github.com/fwcd/tree-sitter-kotlin)
- [alex-pinkus/tree-sitter-swift on GitHub](https://github.com/alex-pinkus/tree-sitter-swift)
