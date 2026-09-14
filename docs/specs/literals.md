# Literal & constant duplication — the value-level finding family

Detects repetition that fragment-level clone detection is structurally blind to: duplicated inline literals, constants redeclared across files and classes, constants whose values drifted under one name, and one value hiding under several names. These are `LiteralFinding` records, not admitted-pair closure components, so they cannot acquire clone-pair evidence or classification.

Why the family was missing, and the evidence base for every default here: [DECISION-LITERALS]
(decisions.md) and [reading-list.md](reading-list.md#read-list-literals). Ranking policy:
[RANK-LITERAL-FAMILY] and [RANK-UNUSED-PUBLIC] (pipeline.md). Category registry:
[CLONE-CATEGORY-REGISTRY] (taxonomy.md). Surfaces: [FACET-MODEL] (facets.md),
[MCP-TOOL-DUPLICATES] (mcp.md).

### [LITERAL-DETECT] Site capture rides the existing walk

One tree-sitter parse per file — the parse that already exists. Extraction happens **inside the
existing normalisation walk** (`deslop-core::lang::shared`, `normalise_node`), **before** the
`__literal__` collapse destroys value identity ([PIPELINE-NORMALIZE-AST] is not weakened; Type-2
detection depends on it). No second traversal, no regex, no new global state.

The walk carries what capture needs: `build_normalised_root` / `normalise_node` receive
`source: &[u8]` and one per-language hooks struct (`WalkHooks`) bundling the language's
`literal_kind()` fn, its constant recogniser, and `&mut SiteCollector` (module
`deslop-core::literals`) — one struct, never loose parameters. Two capture points:

1. **Literal leaves.** When the raw kind maps to a literal, push
   `LiteralSite { byte_range, kind: LiteralKind, interpolated: bool }`.
   `LiteralKind ∈ { Str, Number, Bool, Null, Char }` is derived from the raw kind string via one
   per-language `literal_kind(raw: &str) -> Option<LiteralKind>` function. That function is the
   **single source of truth** for "what is a literal" in each language — `normalise_kind`'s
   literal arm, any `is_literal_kind` helper, and the highlight renderer all delegate to it; no
   second literal-kind list exists anywhere (DRY hard rule).
2. **Constant declarations.** When the raw kind matches the language's declaration head, the
   per-language recogniser (`deslop-core::literals::<lang>`) returns
   `ConstSite { name_range, value_range, decl_range, container, value_kind,
   self_identifier_count }`.
   `container: ContainerSite { kind: ContainerKind { Module, Class, Function }, name_range:
   Option<ByteRange> }` is derived from the walk's ancestor kinds (`name_range` is the enclosing
   class/function name node; `None` for module scope) — it powers the OOP "declared in N classes"
   evidence, and "different containers" always means different `(kind, resolved name text)` pairs.
   `value_kind: Option<LiteralKind>` is `None` when the initialiser is not a bare literal (e.g. a
   `const` constructor call) — such declarations are recorded but **excluded from value joins**,
   which keeps Dart `IconData` registries (#169) out of the alias category by construction.
   `self_identifier_count` counts identifier nodes inside `decl_range` whose text equals the
   constant name (consumed by [LITERAL-UNUSED-MARKER]).

Sites store **byte ranges only** — never value text, never `FileId`. Text resolves lazily at join
time from the retained in-memory sources, the same pattern the embedding snippet path uses.
**Capture is config-independent**: every literal and constant site is captured and cached; all
config-driven gates evaluate at join time ([LITERAL-NOISE]), so a config change never serves stale
cached decisions. Tree-shape facts the join cannot recover from byte ranges are captured as flags
on the site: `interpolated`, `docstring`, `in_cfg_test_module`, `in_annotation_argument`,
`in_type_annotation`, `in_import_context`, `param_name_match`, `in_const_value_range`, and
`in_data_subtree` (computed in the same per-file pass by a containment check against the
`is_literal_data_subtree` ranges once the normalised tree exists — that predicate needs the
completed tree, so the flag is stamped after the walk, before caching).

### [LITERAL-DETECT-SITES] Per-language node kinds and constant recognisers

Grammar-pinned node kinds (versions per `crates/deslop-core/Cargo.toml`). Every kind below gets a
fixture in the E2E suite ([LITERAL-TESTING]); an unknown kind is simply not captured — never a panic.

| Language | Literal leaf kinds | Constant declaration rule |
|---|---|---|
| C# | `string_literal`, `verbatim_string_literal`, `raw_string_literal`, `integer_literal`, `real_literal`, `character_literal`, `boolean_literal`, `null_literal` | `field_declaration` with a `const` modifier, or both `static` and `readonly` modifiers (modifier text via byte equality on the modifier node — not regex); `local_declaration_statement` with `const`. Name from `variable_declarator`'s name field; value from its initialiser expression. |
| Rust | `string_literal`, `raw_string_literal`, `char_literal`, `integer_literal`, `float_literal`, `boolean_literal` | `const_item`; `static_item` without a `mutable_specifier` child. Name/value via `child_by_field_name("name")` / `("value")`. |
| Python | `string`, `concatenated_string`, `integer`, `float`, `true`, `false`, `none` | Module- or class-level `expression_statement > assignment` whose `left` is an `identifier` in UPPER_SNAKE (a byte scan over the identifier text) and whose `right` is a constant-shaped value per the `assignment_is_constant` / `is_constant_value` predicates, which live in `literals::python` and are re-exported to `cluster_filters`. |
| Dart | the scalar + string kinds already enumerated in `lang/dart.rs` | `static_final_declaration` / `initialized_identifier` under a `const` or `static const` context, top-level or class member. |

Interpolated strings (C# interpolated, Python f-strings — a `string` with an `interpolation` child,
Dart `${}` templates) are captured with `interpolated: true` and **never participate in value
grouping**. Python docstrings (a `string` that is the sole child of the first `expression_statement`
of a module/class/function body — a pure tree-shape check) are skipped at capture.

### [LITERAL-VALUE-NORM] Value normalisation (the matching key)

Matching key = `(language, LiteralKind, normalized_value)`. Cross-language values never group
([DECISION-CROSS-LANGUAGE]).

- **Strings** — the value is the concatenation of the grammar's string **content** node ranges, so
  `'x'` / `"x"` and raw-vs-plain quoting match on content. Escape sequences are compared **as
  spelled** (byte-identical escapes match; `"\x0A"` vs `"\n"` do **not** merge). Conservative by
  design: a false value-merge fabricates a finding, a false split merely under-reports.
- **Numbers** — strip `_` digit separators; parse `0x`/`0o`/`0b` and decimal via `i128`; floats via
  `f64` with canonical (Ryu) formatting, so `0x10` ≡ `16`, `1_000` ≡ `1000`, `1e3` ≡ `1000.0`.
  Any parse failure (e.g. `u128`-range literals) falls back to raw-text equality — never a panic.
  A literal leaf whose parent is the language's unary-minus/negation node is captured with the sign
  included in `byte_range` and normalised as negative — this is what makes the `-1` ignore entry
  reachable, and what lets `const OFFSET = -1` count as a literal-valued declaration. Integer and
  float spellings never cross-group: `16` and `16.0` normalise to distinct keys.
- **Bool / null / `None`** — never participate in findings (hardcoded).

Threshold semantics everywhere in this spec are **inclusive**: a rule reading "≥ 3" fires at exactly
3. Stated explicitly because the inclusive/exclusive ambiguity is a documented source of user pain
in shipping analysers (sonar-dotnet #9647).

### [LITERAL-CATEGORY] The five finding categories

All five are dedicated literal finding kinds under [CLONE-CATEGORY-REGISTRY]. They are not clone-cluster classifications and never attach similarity evidence to a closure component. The cross-file join runs beside clone materialisation and emits literal-family finding records. One declaration may legitimately appear in findings of different kinds: an `{A, A, B}` value set under one name yields both `constant_duplicate` over the two `A`s and `constant_drift` over all three.

Three rules apply to every join: (1) **language-scoped, unconditionally** — name joins and value
joins never cross language ids ([DECISION-CROSS-LANGUAGE] has no opt-in for this family); (2)
**gate applicability** — `magic_literal`, `shadowed_constant`, and `constant_alias` apply the
[LITERAL-NOISE] value-distinctiveness gates; `constant_duplicate` and `constant_drift` apply none
(the byte-equal name is the evidence); (3) **precedence** — a value key matched by ≥ 1 recognised
constant declaration is reported only as `shadowed_constant`, never additionally as
`magic_literal`: the higher-precision finding supersedes the lower for that key.

#### [LITERAL-CATEGORY-MAGIC] `magic_literal` — repeated inline value

≥ `min_occurrences` (default **3**) inline literal sites with the same key, spread across
≥ 2 files — **or** ≥ `single_file_min_occurrences` (default **5**) sites within one file —
where every site survives the noise gates ([LITERAL-NOISE]) and none sits inside a recognised
constant declaration's `value_range`. Strings are on by default; numbers are opt-in
(`[literals] magic_numbers`, [LITERAL-CONFIG]) per the unanimous shipping-analyser verdict
([DECISION-LITERALS]). Occurrences are the literal tokens. The two-tier floor reconciles Sonar's
per-file threshold (3) with the evidence that cross-file duplication is the fault-prone kind
(Juergens ICSE 2009): cross-file repetition fires at 3, same-file-only style fires only at 5.

#### [LITERAL-CATEGORY-SHADOWED] `shadowed_constant` — the value already has a name

≥ 1 inline literal site (passing exactly the `magic_literal` gate set — including the
`magic_numbers` opt-in, so with numbers off this category fires on strings only) whose key equals
the value of ≥ 1 recognised constant declaration in the same language. The finding's occurrences are the constant declaration (canonical, first) plus every inline site, so the finding has at least two occurrences naturally. Per the [LITERAL-CATEGORY] precedence rule, these keys never also produce a `magic_literal` finding. This is the
highest-precision, highest-value prevention finding in the family (goconst `match-constant` is the
only default-on named-constant precedent in any shipping tool — near-zero false positives because
the canonical name already exists). It is the literal-family analogue of `find-similar`: *the
canonical already exists; use it.*

#### [LITERAL-CATEGORY-CONST-DUP] `constant_duplicate` — same name, same value, declared twice

≥ 2 recognised constant declarations with byte-equal name text and equal normalised literal value,
in different files **or** different containers within one file. When every occurrence's container is `Class`, the human renderer states "declared in N classes" and suggests hoisting to one shared declaration. These sentences are derived from the dedicated finding and are not wire fields. Occurrences are whole declarations.

#### [LITERAL-CATEGORY-CONST-DRIFT] `constant_drift` — same name, conflicting values

≥ 2 recognised constant declarations with byte-equal name text and ≥ 2 **distinct** normalised
values. The finding contains **all** declarations of that name; each occurrence carries its value on
the wire (`constant_value`) so an agent resolves the conflict without opening files. This is a
correctness warning, not extractable duplication — same-name-different-value is the
forgotten-update signature (Engler SOSP 2001 z-ranking; CP-Miner's unchanged-ratio heuristic;
fault grounding transferred by inference, no direct study exists — recorded in
[DECISION-LITERALS]). Its priority is never multiplied or demoted ([RANK-LITERAL-FAMILY]).

#### [LITERAL-CATEGORY-CONST-ALIAS] `constant_alias` — same value, different names

≥ 2 recognised constant declarations with equal normalised literal value but ≥ 2 distinct names.
**Strings only by default** (`[literals] alias_numbers` opt-in): equal numeric values under
different names are usually coincidence (`MAX_RETRIES = 3`, `MIN_ITEMS = 3`), the
highest-false-positive kind in the family. The shared value must pass the same distinctiveness
gates as `magic_literal`, so `ZERO`/`NIL` pairs over `0` never fire.

### [LITERAL-NOISE] Noise suppression defaults

Source-verified from shipping analysers ([reading-list.md](reading-list.md#read-list-literals));
every named default is a
`[literals]` config knob unless marked hardcoded.

- **Bool / null / `None` literals** — never findings (hardcoded).
- **Numbers** — built-in ignore set: integers `{-1, 0, 1, 2}`, floats `{0.0, 1.0}` (the convergent
  core of Sonar / Checkstyle / clang-tidy / go-mnd allowlists), compared post-normalisation.
  Extendable via `ignored_values`; the larger conventional sets (`10`, `100`, `1000`, powers of two)
  are deliberate non-defaults users opt into.
- **Strings** — content length < `min_string_length` (default **5** content chars, quotes/prefixes
  excluded — the Sonar S1192 value for all four current languages); empty / whitespace-only
  (hardcoded); identifier-like content (every char in `[A-Za-z0-9_-]`, a char-class scan over the
  content text, never regex on source — knob `suppress_identifier_like`, default `true`: flip it
  off to enumerate serialisation-key-style slop); format/placeholder-only content (e.g. `{}`,
  `%s`, `{0}`) and `#`-prefixed 3/6/8-digit hex colours (knob `suppress_format_strings`, default
  `true`); interpolated strings and Python docstrings (hardcoded — captured flags).
- **Context** (all hardcoded — these are correctness rules, not taste) — literals inside a
  recognised constant declaration's `value_range` (the fix site must never re-trigger the rule);
  annotation / attribute / decorator arguments; Python type annotations; import / `using` /
  directive contexts (already boilerplate carriers per [PIPELINE-BOILERPLATE-FILTER]); sites inside
  `is_literal_data_subtree` containers (prevents a 500-row table flooding the family); a literal that byte-equals the name of an
  enclosing function parameter (sonar-dotnet precedent).
- **Tests** — excluded by default (`include_tests = false`): Rust `#[cfg(test)]` modules (tree-shape
  check) and `tests/` directories; Python `test_*.py` / `*_test.py` / `tests/` directories; C#
  `*Test.cs` / `*Tests.cs`; Dart `*_test.dart` / `test/` directories. Test fixtures legitimately
  repeat values (go-mnd / sonar-python / goconst precedent).
- **Exclusion tiers** — `[EXCLUSION-CONFIG]` applies unchanged: `exclude`d files are never parsed;
  `report_hide` sites get `hidden: true` through the existing occurrence path, and fully hidden findings count in `literal_findings_hidden`.
- **Volume cap** — at most `max_findings` (default **100**, validated ≥ 0 where **0 = unlimited**)
  findings per literal-family kind, mass-first; overflow is counted in `literal_findings_hidden`, and the report records that the literal-finding list was capped plus the configured `[literals] max_findings` value so a consumer knows enumeration is incomplete and how to widen it. The supported
  widening flow is editing `.deslop.toml` (the live loop picks the change up on the next refresh /
  `rescan`) — "find ALL the duplicate literals" is always reachable via `max_findings = 0` plus
  `suppress_identifier_like = false`.

### [LITERAL-CANONICAL] Canonical-target pick (deterministic)

`occurrences[0]` is the canonical suggestion, chosen by a **successive-filter cascade**: start from
all visible occurrences; each rule keeps only its argmax subset; stop when one remains — (a) being
a recognised constant declaration (for `shadowed_constant` this is the existing constant; for
`constant_duplicate` prefer a declaration with ≥ 1 repo reference when the
[LITERAL-UNUSED-MARKER] index ran); (b) being in the file containing the most occurrences of the
finding; (c) lexicographically smallest relative path; (d) smallest `start_byte` — always unique
within one file, so the cascade always terminates deterministically.

The finding also carries a structured `canonical_target { path, line, constant_name? }` wire field so agents never parse prose to find the keep-site, and the human renderer names it directly ("Keep `MAX_RETRIES` at `src/config.rs:12` and replace the other 6 occurrences."). Per
category: for `magic_literal`, `canonical_target` points at `occurrences[0]` read as *"declare the
constant here, nearest the most uses"* with `constant_name` absent; for `constant_drift` it is
**`None`** — the conflict is unresolved by definition, and the human renderer lists the variant values instead.

### [LITERAL-WIRE] Dedicated literal-finding wire model

All wire changes originate in `docs/models/live-ipc.td` and are regenerated. `LiteralFinding` carries `id`, `kind`, `mass`, `occurrences`, `constant_name`, `literal_value`, and `canonical_target`. `LiteralOccurrence` carries path, byte range, optional constant value, container, and unused-reference confidence. The top-level report carries `literal_findings`, `literal_findings_total`, and `literal_findings_hidden` separately from `clusters`, `clusters_total`, and `clusters_hidden`. Literal and constant text is capped at 80 characters on the wire and never logged.

The id is the first eight bytes of `blake3(kind_wire_label ‖ language ‖ key)` in hex. The key is normalized value for `magic_literal`, `constant_alias`, and `shadowed_constant`; name plus value for `constant_duplicate`; and name alone for `constant_drift`.

Literal findings carry no clone-pair structural, Jaccard, embedding, content, admission, or pair-classification field. Their mass is canonical node count times additional visible occurrences under [RANK-LITERAL-FAMILY]. `RepoMetrics.duplicated_loc`, `duplication_percent`, and `clusters_total` exclude them; `literal_findings_total` reports their count separately.

### [LITERAL-CACHE] Incremental & live loop

Sites are two vectors on the cached per-file entry (`CachedFile`), encoded in the
fingerprint-cache blob. Byte ranges only — no `FileId` rewrite on decode; format changes are
auto-invalidated by the tool-version cache key, so no migration code exists. The live loop needs
**no extra machinery**: a file change re-parses exactly one file (the walker re-emits that file's
sites in the same pass), the per-file map insert/remove replaces or drops them atomically, and the
cross-file join re-runs inside the render stage every generation — a few hash-map passes over
in-memory vectors, orders of magnitude cheaper than the LSH pass it sits beside, inside the existing
latency budget. Deterministic ids make `ReportDelta` classification work unchanged: editing
`TIMEOUT = 30 → 60` flips a `constant_duplicate` to removed and a `constant_drift` to added within
one watcher → scheduler → session → broadcast → UI cycle (Deslop.live means the whole loop).

### [LITERAL-UNUSED-MARKER] Unused public constants (monorepo)

A confidence-calibrated **marker**, never a deletion directive (vulture's confidence model; Romano
TSE 2018: developers distrust deletion of possibly-dynamically-used code).

**Index.** During the same walk, each file contributes (a) a multiset of identifier-node texts and
(b) the set of word tokens split from string-literal contents (split on non-identifier chars — a
char scan, not regex). Both live on the cached file entry and merge by summation, so the index is
incrementally maintained per file change. The index covers **every parsed file, including test
files regardless of `include_tests`** — over-counting biases the marker toward silence, the correct
failure direction, so a constant referenced only by tests is never marked. For each recognised
**public** constant declaration:
`repo_references` = corpus identifier count for the name − the declaration's own
`self_identifier_count` ([LITERAL-DETECT] — counted during the walk, so type-position mentions
like `const MAX: u32` subtract correctly).

**Precision asymmetry (the keystone).** Textual matching without symbol resolution systematically
*over*-counts references (same name in two classes, shadowing) — which biases the marker toward
silence. For an "unused" claim that is exactly the right failure direction: false negatives are
free, false positives burn trust.

**Suppression cascade** (any hit kills the marker): name < 4 identifier chars; name appears as a
word token in any string literal (serialisation keys, `getattr`/reflection/DI registration — the
Android resource-shrinker heuristic); the declaring package is publishable (Rust crate without
`publish = false`; Dart package without `publish_to: none`; C# project without
`IsPackable=false`; Python package outside the workspace) — **published public constants are the
product**; the declaration is re-exported at the public surface (Rust `pub use` reaching the crate
root; Dart declaration under `lib/` not `lib/src/`; Python `__all__` / package `__init__.py`
import); generated or FFI code (Rust `#[no_mangle]` / `extern`).

**Confidence** (never 100). Publishability is three-valued: *provably publishable* (manifest says
so) → suppressed by the cascade above; *provably non-publishable* (manifest explicitly says so:
`publish = false`, `publish_to: none`, `IsPackable=false`); *unknown* (no manifest signal either
way) → the marker survives with no bonus. The ladder: base **60** when `repo_references == 0`;
**+15** when the declaring package is provably non-publishable (auto-detected monorepo counts —
no separate "confirmed" state); **+15** when the name is ≥ 8 chars and constant-shaped
(SCREAMING_SNAKE / PascalCase). Cap **90**, so the reachable values are 60 / 75 / 90. Carried as
`LiteralOccurrence.unused_confidence`; the human badge is the factual phrase **"0 references found
in this repo"** — never the word "unused" as an absolute. The marker only activates when the
workspace is a monorepo: `[workspace] monorepo = "auto"` detects Cargo `[workspace].members`,
multi-project `.sln`, Dart workspace lists, or Python workspace tables; `true`/`false` override.

**Effect.** Per-occurrence badge on constant-family findings. [RANK-UNUSED-PUBLIC] forbids any mass change. A standalone unused-constant finding is out of scope for v1; the marker needs a repeated family to ride.

### [LITERAL-CONFIG] Configuration

`.deslop.toml` carries the `[literals]` and `[workspace]` sections, loaded by `deslop-core::config_literals`. Literal ranking multipliers are invalid; all surviving findings use mass. Invalid values are named errors, never silent fallbacks.

```toml
[literals]
enabled = true                  # false skips the cross-file join; sites are still captured
                                # ([LITERAL-DETECT] capture is config-independent, so toggling
                                # never serves a stale cache)
magic_numbers = false           # numbers in magic_literal/shadowed_constant: opt-in
alias_numbers = false           # numbers in constant_alias: opt-in
min_occurrences = 3             # magic_literal cross-file floor; validated >= 2
single_file_min_occurrences = 5 # magic_literal same-file-only floor; validated >= min_occurrences
min_string_length = 5           # string content chars; validated >= 1
ignored_values = []             # added to the built-in ignore sets, compared post-normalisation
suppress_identifier_like = true # drop identifier-like string values ([LITERAL-NOISE])
suppress_format_strings = true  # drop placeholder-only strings and hex colours
include_tests = false           # include test files in literal-family detection
max_findings = 100              # per-kind finding cap; validated >= 0, 0 = unlimited

[workspace]
monorepo = "auto"               # "auto" | true | false — gates [LITERAL-UNUSED-MARKER]
```

The editor exposes `deslop.literals.enabled` as `default | on | off`; `default` defers to `.deslop.toml`. The LSP launch mirror is `--literals-enabled <on|off>` and the CLI opt-out is `--no-literals`. Literal ranking multipliers and boost flags do not exist.

### [LITERAL-CENSUS] Calibration gate on the default

`enabled = true` is only a valid shipped default while a recorded census holds ([DECISION-LITERALS]
carries the numbers): under default config, a mid-size real repo (this repository's own crates are
the reference corpus) yields fewer than ~50 literal-family findings with maintainer-judged ≥ 80%
actionability in the top 20. The bound is enforced by a **permanent census E2E** — a fixture run
asserting the literal-family finding count stays within the calibrated bound — so noise regressions
fail CI. When the census does not hold, the valid default is `enabled = false`; a silently-noisy
default is never valid. The tuning procedure lives with the decision record
([DECISION-LITERALS]), mirroring how [DECISION-MIN-NODES] carries the min-nodes tuning procedure.

### [LITERAL-TESTING] E2E proof (coarse, black-box, spec-ID-tagged)

Fixture-driven CLI suites per the testing rules; every assertion targets positive human-readable
values, not AI-label absence:

1. **Magic + noise traps** — one fixture per language: a distinctive string inline 4× across 3
   files, plus every suppression trap in one file (`0`, `1`, `true`, `""`, two-site repeats, an
   interpolated string, a docstring, a literal data table, a string inside a const initialiser, test
   files). Assert: exactly one `magic_literal` finding with the expected `literal_value`,
   occurrence lines, `canonical_target`; zero literal findings for every trap; `metrics.duplicated_loc`
   identical to a literal-free control. // [LITERAL-CATEGORY-MAGIC] [LITERAL-NOISE]
2. **Constants** — `constant_duplicate` per language incl. a C# three-class fixture asserting the
   summary contains "3 classes" and occurrence `container` values; `constant_drift` with both
   `constant_value`s asserted plus the `{A, A, B}` overlap fixture (duplicate **and** drift findings,
   distinct stable ids); `constant_alias` with an alias pair, a suppressed `ZERO`/`NIL` = 0 pair,
   and a Dart `IconData` registry asserting non-literal consts never alias;
   `shadowed_constant` with the constant + inline bypasses, canonical = the declaration, **and
   zero `magic_literal` findings for that key** (the [LITERAL-CATEGORY] precedence rule).
   // [LITERAL-CATEGORY-CONST-DUP] [LITERAL-CATEGORY-CONST-DRIFT] [LITERAL-CATEGORY-CONST-ALIAS]
   [LITERAL-CATEGORY-SHADOWED]
3. **Value normalisation** — hex/underscore/float fixtures: `0x10` groups with `16`, `1_000` with
   `1000`, `1e3` with `1000.0`; escape spellings do not cross-merge. // [LITERAL-VALUE-NORM]
4. **Policy + cache** — `magic_literals = ignore` increments `literal_findings_hidden`; every kept finding preserves its mass; `enabled = false` yields zero literal-family findings;
   same-dir double run: second report byte-identical with `cache_stats.hits > 0`.
   // [RANK-LITERAL-FAMILY] [LITERAL-CACHE]
5. **Unused marker** — workspace fixtures: cross-crate-used const not flagged; string-key-referenced
   const suppressed; orphaned const in a `publish = false` crate flagged without changing mass, badge text "0
   references found in this repo" asserted verbatim; `lib.rs`-re-exported const suppressed;
   same-name-two-classes-one-used → neither flagged. // [LITERAL-UNUSED-MARKER] [RANK-UNUSED-PUBLIC]
6. **Live loop** — real LSP over a fixture: edit `TIMEOUT 30 → 60`, assert the delta carries the
   drift finding added and the duplicate finding removed within one generation. // [LITERAL-CACHE]
7. **Census** — the calibrated noise-bound regression run. // [LITERAL-CENSUS]

Existing regression fixtures (#61/#62/#64/#66/#112/#169 suites) must stay green — the family must
not resurrect any suppressed data-table or fixture noise.
