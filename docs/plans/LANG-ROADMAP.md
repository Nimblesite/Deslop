# Language Expansion Roadmap

> Scope: plan the order in which Deslop picks up new source languages beyond
> the original v1 set (C#, Rust, Python). Shipped since: F# (a first-class,
> integral language — see [LANG-CAND-FSHARP]), TypeScript, TSX, JavaScript,
> Dart, and PHP. Remaining slots go to low-hanging, high-impact languages
> whose tree-sitter grammars are already production-grade.

Core research conducted 2026-04-23; F# addendum 2026-07-05. Version/status
rows reflect crates.io + docs.rs + GitHub state at those dates.

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
| tree-sitter-php              | 0.24.2         | ^0.1       | ✅ SHIPPED (=0.24.2) |
| tree-sitter-dart (nielsenko) | 0.2.0          | ^0.1       | ✅ SHIPPED (=0.2.0) |
| tree-sitter-fsharp (ionide)  | 0.3.1          | ^0.1       | ✅ SHIPPED (=0.3.1) |
| tree-sitter-swift            | 0.7.1          | *(ts ^0.23)* | ⚠ generated-file workaround |
| tree-sitter-kotlin (fwcd)    | 0.3.8          | *(ts 0.21–0.22)* | ❌ incompatible |

**Implication.** TypeScript (and Go / Java / C++ / Ruby / Bash / PHP /
Dart / F#) can now build on the modern `LanguageFn` grammar surface.
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
the report output. (Note: this scoring ranks the *default* queue; grammar
quality and product priority can pull a language forward. F# shipped ahead
of the remaining queue on a clean grammar and is now integral to the
supported set.)

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

### [LANG-CAND-FSHARP] F# — ✅ SHIPPED (=0.3.1) — first-class

F# is a first-class, integral member of Deslop's supported set — held to at
least the same bar as every other language and wired end to end (parser,
registry, CST filters, HTML + live surfaces, VS Code activation). The
ionide grammar clears every constraint in [LANG-ROADMAP-CONSTRAINTS]
cleanly, so F# shipped 2026-07-05 with full Type-2, Type-3,
dissimilar-guard, and byte-for-byte AST-golden coverage — as thorough as
any language in the fleet, and more completely filter-wired than some.

- **Crate.** `tree-sitter-fsharp = "=0.3.1"` (ionide org — the same team
  behind the Ionide F# tooling). Published to crates.io; not a git-dep.
- **Runtime.** Depends on the `tree-sitter-language ^0.1` shim and
  dev-deps `tree-sitter 0.26.8` — exactly Deslop's pinned runtime. Loads
  natively via the modern `LANGUAGE_*` (`LanguageFn`) surface, no
  binding-version hacks.
- **Exports TWO grammars.** `LANGUAGE_FSHARP` (`.fs` implementation +
  `.fsx` script files) and `LANGUAGE_SIGNATURE` (`.fsi` signature files —
  the signature grammar extends the source grammar). We ship **one**
  `FSharpParser` on `LANGUAGE_FSHARP` covering `.fs`/`.fsx`, where all
  real F# code and its duplication live. The `.fsi` signature parser is a
  documented follow-up — see [LANG-CAND-FSHARP-FSI].
- **Normalisation knowns** ([PARSE-FSHARP-NORMALIZE], derived from the
  196-kind named grammar vocabulary, not a sampled subset):
  - `identifier`, `op_identifier` → `__ident__`. The compound wrappers
    (`long_identifier`, `long_identifier_or_op`, `identifier_pattern`)
    stay structural, so a dotted path `A.B.C` keeps its shape while each
    segment collapses (parity with the Python / TypeScript member-access
    handling).
  - `int`, `xint` (hex/oct/bin), `float`, `char`, `bool`, `unit` (`()`),
    and every string form — `string`, `triple_quoted_string`,
    `format_string`, `format_triple_quoted_string`, `verbatim_string` →
    `__literal__`. The interpolation-hole container `format_string_eval`
    stays structural, so `$"{a}"` and `$"{b}"` still match through the
    collapsed identifiers while a plain string reduces to a constant
    `__literal__` subtree (same treatment as Dart,
    [LANG-CAND-DART-RESULT]).
  - `line_comment`, `block_comment`, `xml_doc` dropped as trivia.
- **Grammar-map wiring — a latent gap F# surfaced.** The CST re-parse map
  `cluster_filters::snippets::grammar_for` is a *second* language→grammar
  table, separate from `default_parsers`, that the signature-only (#154)
  and other CST-walking filters depend on. F# was added there too, so the
  signature-only false positive (`let f (_: int) : int` headers
  collapsing identically, bodies differing) is correctly suppressed.
  (PHP is still missing from that map — a pre-existing gap, out of scope
  for the F# change.)
- **Fixtures.**
  - `fsharp-small/{alpha,beta}.fs` — Type-2 renamed clone (accumulate
    loop); clusters at structural = 1.0 at `--min-nodes 10`.
  - `fsharp-type3/{delta,epsilon}.fs` — Type-3 near-miss (loop body of two
    statements vs one); shared body subtrees cluster cross-file at
    structural = 1.0 at `--min-nodes 8`, signature-only match suppressed.
  - `fsharp-dissimilar-functions/{tally,describe}.fs` — zero-false-positive
    guard (`Map`-fold vs `if`/`elif` cascade); never clusters cross-file.
  - `ast-golden-fsharp/Sample.fs(.expected.ast)` — byte-for-byte golden
    exercising every literal form, all three comment kinds, and the nested
    `let … in` desugaring.
- **Zero `ERROR`/`MISSING` nodes** across all fixtures, covering idiomatic
  F#: `mutable`, `for … in … do`, `if`/`elif`/`else`, the pipe operator
  `|>`, `Map` operations, generic type annotations (`string list`,
  `Map<string, int>`), and every string-quote variant.
- **Estimate (actual).** ~half a day; the grammar cleared every gate on
  the first pass.
- **F# idiom references** — consult these before adding fixtures or
  touching `normalise_kind`; F# has idioms no other shipped language has,
  and fixtures must read like real F#, not C#-in-F#-syntax:
  - [F# language reference](https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/)
    — the authoritative, grammar-level reference for every construct.
  - [F# style guide](https://learn.microsoft.com/en-us/dotnet/fsharp/style-guide/)
    and [component design guidelines](https://learn.microsoft.com/en-us/dotnet/fsharp/style-guide/component-design-guidelines/)
    — the idiomatic conventions the fixtures follow.
  - [ionide/tree-sitter-fsharp `node-types.json`](https://github.com/ionide/tree-sitter-fsharp/blob/main/fsharp/src/node-types.json)
    — the exact node vocabulary `normalise_kind` matches against.
  - Idioms the normaliser deliberately keeps **structural** (never
    collapsed to `__ident__`/`__literal__`): active patterns
    (`(|Even|Odd|)`), computation expressions (`async { … }`,
    `seq { … }`, `task { … }`), pipelines and composition (`|>`, `>>`),
    cascades of `member`/`let`/`use` bindings, units of measure
    (`float<m/s>`), and quotations (`<@ … @>`). Only identifier / literal
    / comment leaves collapse; all of the above stays as tree shape, so a
    fixture must vary those shapes (not just names) to test structure.

#### [LANG-CAND-FSHARP-FSI] `.fsi` signature files — follow-up

`tree-sitter-fsharp` also exports `LANGUAGE_SIGNATURE` for `.fsi`
signature files. Signature files declare types/contracts and carry little
copy-paste duplication, so they are deferred: a second
`FSharpSignatureParser` (id `fsharp_signature`, ext `fsi`, sharing
`normalise_kind`) mirroring the TS/TSX two-impl split, plus its own
fixtures + golden. Not blocking; low value.

### [LANG-CAND-DART] Dart — ✅ SHIPPED (=0.2.0)

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
- **Outcome.** Spike passed on the newer `0.2.0` — see
  [LANG-CAND-DART-RESULT] in the execution log.
- **Fixtures.** `tests/fixtures/dart-small/{alpha,beta}.dart`,
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

### [LANG-CAND-PHP] PHP — ✅ SHIPPED (=0.24.2)

- **Crate.** `tree-sitter-php = "=0.24.2"` (tree-sitter org). `^0.24`.
  Exposes both `LANGUAGE_PHP` (full PHP with HTML) and
  `LANGUAGE_PHP_ONLY` (pure PHP, no HTML interleaving). The roadmap
  originally planned `LANGUAGE_PHP_ONLY`; the shipped `php.rs` uses
  `LANGUAGE_PHP`, which handles the `<?php … ?>`-tagged files agents
  actually emit. (Revisit if HTML-interleaved `text` nodes prove noisy.)
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

- **Scala, Haskell, OCaml, Elixir, Clojure, Nim, Zig** — low demand
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
2. **The tree-sitter runtime gets upgraded to `=0.26.x` before any new
   language lands.** Alternative — back-port grammars to 0.22 — is not
   feasible; most modern grammars don't publish 0.22-compatible
   versions and the legacy `::language()` API is gone.
3. **Dart is gated on a spike, not a commitment.** User flagged the
   risk explicitly. Spike passed on `0.2.0` ([LANG-CAND-DART-RESULT]).
4. **Kotlin is deferred** because its grammar is stuck on tree-sitter
   0.21–0.22 and its corpus parity is 61%.
5. **Swift is deferred** because the grammar requires a generated-file
   workaround in the build script.
6. **F# is a first-class shipped language.** One `FSharpParser` on
   `LANGUAGE_FSHARP` handles `.fs`/`.fsx`; the `.fsi` signature grammar
   (`LANGUAGE_SIGNATURE`) is a documented follow-up ([LANG-CAND-FSHARP-FSI]).
   Adding F# also wired it into `cluster_filters::snippets::grammar_for` —
   the CST re-parse map the signature-only (#154) filter depends on,
   distinct from `default_parsers` — so F# has the same filter coverage as
   every other language.

---

## [LANG-PER-LANG-CHECKLIST] Every new language must ship

A new-language PR is not done until all of the following are green:

- [ ] Grammar crate added with exact `=x.y.z` pin in
      `Cargo.toml`, `.github/workflows/ci.yml`, `.devcontainer/`.
- [ ] `LanguageParser` impl in `crates/deslop-core/src/lang/<name>.rs`
      (< 100 LOC; `normalise_kind` is the only language-specific logic;
      all shared plumbing reused from `lang::shared`).
- [ ] Registered in the pipeline's parser registry (`lang::mod`
      re-export + `pipeline::corpus::default_parsers`).
- [ ] Added to `cluster_filters::mod::function_kinds` (function-body node
      kinds) and `cluster_filters::snippets::grammar_for` (CST re-parse
      map) so the signature-only (#154) and CST filters apply.
- [ ] Human display name in `render::html::language_display_name` and the
      live extension map in `live::session::extension_to_language`.
- [ ] File-extension filter contributes to the discovery stage.
- [ ] E2E Type-2 fixture (renamed clone) — cluster asserted in report.
- [ ] E2E Type-3 fixture where a shared body subtree clusters cross-file
      while the signature-only sibling match is suppressed.
- [ ] E2E dissimilar-functions fixture — structurally unrelated functions
      never form a cross-file cluster (zero-false-positive guard).
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
  `.deslop.toml`. Same question for C↔C++ and `.fs`↔`.fsi`. Still open —
  TS/JS shipped language-scoped for now.
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

*F# addendum sources consulted 2026-07-05:*

- [tree-sitter-fsharp on crates.io](https://crates.io/crates/tree-sitter-fsharp) (0.3.1)
- [ionide/tree-sitter-fsharp on GitHub](https://github.com/ionide/tree-sitter-fsharp)
- [F# language reference (Microsoft Learn)](https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/)
- [F# style guide (Microsoft Learn)](https://learn.microsoft.com/en-us/dotnet/fsharp/style-guide/)
- [F# language specification (fsharp.org)](https://fsharp.org/specs/language-spec/)

---

## [LANG-EXECUTION] Phased execution — live TODO

All phases follow the existing PLAN.md shape: each bullet produces
code + e2e fixture + AST golden + grammar pin in `Cargo.toml`,
`.github/workflows/ci.yml`, and `.devcontainer/`.

### Shipped

- **P-LANG-0 — tree-sitter runtime upgrade** (COMPLETE, CI GREEN).
  Workspace `tree-sitter = "=0.26.8"`; `csharp` → `=0.23.5`, `rust` →
  `=0.24.2`, `python` → `=0.25.0` migrated to the modern `LANGUAGE`
  surface (Rust AST goldens refreshed for the new
  `lifetime_parameter`/`type_parameter` wrappers; C#/Python stable);
  `shared::parse_source` audited against the 0.26 API (`set_language`
  still takes `&Language`, unchanged); grammar-pin-drift check still
  accepts exact `=x.y.z`. `make ci` green — Rust coverage 96.1%, VSIX
  line coverage 90.11% against the ratcheted 90% floor.
- **P-LANG-1 — TypeScript + TSX** (COMPLETE). Two impls
  (`TypeScriptParser` id `typescript` ext `ts`; `TsxParser` id `tsx` ext
  `tsx`) sharing the `lang::ecmascript` normaliser; `typescript-small`,
  `tsx-small`, `typescript-type3` fixtures + goldens for both grammars;
  VS Code `onLanguage:{typescript,typescriptreact}` activation.
- **P-LANG-2 — JavaScript** (COMPLETE). One impl, exts
  `js`/`mjs`/`cjs`/`jsx`; fixture + golden; VS Code
  `onLanguage:{javascript,javascriptreact}` activation.
- **P-LANG-4 — Dart** (COMPLETE, spike GREEN — see
  [LANG-CAND-DART-RESULT]). `tree-sitter-dart = "=0.2.0"`;
  [`dart.rs`](../../crates/deslop-core/src/lang/dart.rs); `dart-small`,
  `dart-type3`, `dart-dissimilar-functions`, `ast-golden-dart`.
- **P-LANG-PHP — PHP** (COMPLETE, #265). `tree-sitter-php = "=0.24.2"`
  on `LANGUAGE_PHP`; `php.rs`; `php-small` + `ast-golden-php`. (Not yet
  wired into `grammar_for`/display-name maps — a follow-up.)
- **P-LANG-FSHARP — F#** (COMPLETE, 2026-07-05 — see [LANG-CAND-FSHARP]).
  `tree-sitter-fsharp = "=0.3.1"` on `LANGUAGE_FSHARP`;
  [`fsharp.rs`](../../crates/deslop-core/src/lang/fsharp.rs); `fsharp-small`,
  `fsharp-type3`, `fsharp-dissimilar-functions`, `ast-golden-fsharp`;
  wired into `default_parsers`, `function_kinds`, `grammar_for`, the HTML
  display-name map, and the live extension map; VS Code `onLanguage:fsharp`
  + `.fs`/`.fsx` activation. All four e2e tests green.

### [LANG-CAND-DART-RESULT] Dart spike outcome (2026-05-30) — GREEN

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

### Remaining

- **P-LANG-3 — Go** — `tree-sitter-go = "=0.25.0"` + `go.rs` + fixtures.
  The easiest remaining language (trivial normalisation, glacial grammar).
- **P-LANG-5 — Java** — `tree-sitter-java = "=0.23.5"` + plugin + fixtures.
- **P-LANG-6 — C / C++** — `tree-sitter-cpp = "=0.23.4"` +
  `tree-sitter-c = "=0.24.2"` + both plugins + fixtures.
- **P-LANG-7 — Ruby** — `tree-sitter-ruby = "=0.23.1"` + plugin + fixtures.
- **P-LANG-8 — Bash** — `tree-sitter-bash = "=0.25.1"` + plugin + fixtures.
- **P-LANG-FSHARP-FSI — F# `.fsi` signatures** — second parser on
  `LANGUAGE_SIGNATURE` ([LANG-CAND-FSHARP-FSI]); low priority.
- **P-LANG-PHP-WIRING — PHP filter parity** — add PHP to
  `cluster_filters::snippets::grammar_for` and the HTML/live maps so it
  reaches parity with the other languages.
- **(Deferred) Swift, Kotlin** — grammar/runtime blockers per
  [LANG-CAND-SWIFT], [LANG-CAND-KOTLIN].
