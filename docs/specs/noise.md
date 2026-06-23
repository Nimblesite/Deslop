# Cluster-noise suppression

`[CLONE-NOISE-*]` is the family of false-positive filters that run **after**
clustering and **before** ranking. Each one re-parses the real tree-sitter CST of
a cluster's members (never a regex over source) and suppresses the cluster when
its members match a pattern that is *shape-identical but not extractable
duplication* — language scaffolding, framework-mandated mirrors, schema/data
tables, or test idioms. Filters are **additive and conservative**: a filter only
ever hides a cluster, never re-routes a bucket ([taxonomy.md §CLONE-BUCKETS-ROUTING](taxonomy.md#clone-buckets-routing)),
and every filter that could hide genuine copy-paste carries a **verbatim escape
hatch** — if the members are byte-identical it is a real clone and must still
surface. The Dart collection-literal data-table filter is a sibling of this
family but lives with the ranking policy it feeds: see
[exclusion.md §CLONE-NOISE-DART-DATA-TABLE-LITERAL](exclusion.md#clone-noise-dart-data-table-literal).

## Language-agnostic filters

### [CLONE-NOISE-SIGNATURE-ONLY] Signature-only matches
A structural fingerprint can match entirely inside a function or method
signature — the parameter list and return type — without touching the body;
after normalisation `fn check_foo(ctx: &mut Ctx)` and `fn check_bar(ctx: &mut Ctx)`
reach `structural=1.0`, and token Jaccard cannot refute the match because the
distinguishing identifiers normalise away too. A cluster is suppressed (for any
language) when every member's matched range lies entirely before its enclosing
function's body and at least two of those bodies differ in AST node-kind shape.
Comparing bodies by node-kind sequence rather than raw bytes preserves genuine
near-miss clusters whose bodies share shape but differ only in literals or
identifiers.

### [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] Embedding role mismatch (type vs function)
An embedding-dominant `same_behavior` cluster may pair snippets that share topic
vocabulary but sit in structurally incompatible top-level constructs — a
class/type definition matched against a function or method. There is no safe
shared extraction across a type definition and a function, so the cluster is
suppressed. The role gate re-parses each member, resolves its innermost
enclosing construct to one of two roles (type definition or function/method,
descending through decorator wrappers), and hides the cluster only when the
members do not all resolve to the same role. It engages exclusively for the
`same_behavior` bucket, leaving the deterministic Type-1/2/3 buckets untouched,
and never suppresses when a member's role cannot be resolved.

## Python idioms

### [CLONE-NOISE-PY-ALL-EXPORTS] `__all__` export lists
A module-level `__all__ = [...]` export list is package-surface convention, not
duplicated logic: its shape is fixed across modules while the listed names always
differ, so after identifier normalisation two unrelated `__all__` lists collapse
to the same subtree. A cluster whose every member is a module-level `__all__`
assignment bound to a list or tuple literal is suppressed. The export surface
cannot be hoisted or extracted, so surfacing it as duplication is pure noise.

### [CLONE-NOISE-PY-GENERATED-OUTPUT] Generator template vs generated output
A hand-written code generator contains a template literal for a generated-file
header, and the file it emits carries that same `DO NOT HAND-EDIT` marker; the
two ranges cluster but their relationship is provenance, not a shared
implementation to extract. A cluster is suppressed when it spans at least two
files and contains both a template-side member (the marker appears in the
reported range but the file itself is not generated output) and a
generated-output member (the file head carries the marker behind a leading
docstring or comment). The template is already the source of truth and the
generated file is not a refactor target.

### [CLONE-NOISE-PY-JWT-HS256] Independent HS256/JWT verifiers
A test may independently re-implement HS256/JWT HMAC signing to verify a
production token minter as a black box; if the test shared the production helper
it would no longer prove the signing implementation. A cluster is suppressed when
it spans at least two files, every member's enclosing function body exhibits the
stdlib HS256 shape (an `hmac.new` digest over `hashlib.sha256` followed by
`urlsafe_b64encode`), and the cluster contains at least one test-module member
and at least one non-test member. Requiring all three signing calls keeps the
filter far tighter than a generic "uses hmac" suppression.

### [CLONE-NOISE-PY-MONKEYPATCH] `monkeypatch` setup scaffolding
pytest `monkeypatch` setup tests repeat `monkeypatch.setenv("KEY", "VALUE")`
scaffolding whose string literals differ only because they are environment keys
and values, so the tiny literal clusters are scaffolding rather than logic. A
cluster is suppressed when every member is a string literal sitting inside a
Python function whose parameter list declares the `monkeypatch` fixture. The
differing literals are test inputs, not extractable duplication.

### [CLONE-NOISE-PY-ASSERT-ONLY] Assertion-only test blocks
A block of pure Python `assert` statements in a test shares AST shape and token
alphabet with any other assert-only block, yet each intentionally checks
different concrete paths and values. A cluster is suppressed when it spans at
least two files, its members differ in raw bytes, and every member's reported
range covers only `assert` statements (no embedded calls or control flow). The
raw-byte-divergence requirement keeps a verbatim copy-pasted assertion block
visible as genuine duplication.

### [CLONE-NOISE-PY-DICT-ASSERT] Chained-subscript assertions
Chained-subscript assertions of the form `assert X[k1][k2] == V` are a Python
idiom for verifying nested response/payload shapes; after identifier and literal
normalisation every such assertion collapses to
`assert __var__[__str__][__str__] == __const__`, clustering unrelated tests. A
cluster is suppressed when it spans at least two files and every member lives
inside a `test_*` pytest function whose reported range contains only assertions
whose left-hand side is a subscript chain two or more levels deep. Distinct tests
verifying distinct contracts are not extractable duplication.

### [CLONE-NOISE-PY-DICT-FIXTURE] Dict-literal test fixtures
Dict-literal fixtures inside pytest tests carry the same AST shape and a
recurring key alphabet (`name`, `description`, …) across files even when they
encode unrelated request/response payloads. A cluster is suppressed when every
member is the sole dictionary literal enclosed by a `test_*` function, the
members span at least two files, and at least one member declares a different set
of top-level string keys. The differing-key-set requirement keeps a genuinely
copy-pasted identical fixture visible.

### [CLONE-NOISE-PY-PYTEST-FIXTURE] pytest fixture boilerplate
pytest fixtures that build ORM rows repeat the same session setup idiom (add /
commit / refresh / return) by design — the fixture is already the test
abstraction, so surfacing those bodies as refactor targets is noise. A cluster is
suppressed when it spans at least two files and every member's enclosing Python
function is decorated with `@fixture` or any dotted fixture decorator
(`@pytest.fixture`, `@pytest_asyncio.fixture`). The shared shape is fixed by the
fixture protocol, not by the program under analysis.

### [CLONE-NOISE-PY-PARAMETRIC-INVARIANT-TESTS] Parametric invariant tests
Parametric invariant tests — a family of `test_register_<variant>()` functions
that vary only by an enum-member access token such as `Kind.K8S` vs
`Kind.DOCKER` — each record a distinct spec assertion, so collapsing them would
silently lose coverage granularity even when their Type-2-normalised bodies are
identical. A cluster is suppressed when every member lies inside a `test_*`
pytest function and every member's reported range carries at least one
`Capitalised.UPPER_SNAKE` enum-member-access token. The enum-access signal
(matched by a byte scan, never a regex over source) keeps the filter from hiding
ordinary copy-paste inside test files.

### [CLONE-NOISE-PY-KWARGS-CTOR] Keyword-only constructor calls
ORM, dataclass, and Pydantic constructor calls of the shape
`Model(field1=val, field2=val, …)` cluster after identifier normalisation
collapses both the model name and the field names, even though each model
declares its own required columns. A cluster is suppressed when every member is
the sole class-constructor call (a capitalised-identifier callee) using only
keyword arguments, the members span at least two files, and at least one member
supplies a different keyword-name set. The constructor's purpose is to enumerate
per-model fields, so there is no shared refactor; the differing-keyword guard
keeps a genuine copy of one constructor visible.

### [CLONE-NOISE-PY-MAPPED-COLUMN] SQLAlchemy mapped-column declarations
SQLAlchemy column declarations of the form `attr: Mapped[T] = mapped_column(...)`
across distinct ORM models cluster on a shared token alphabet (`Mapped`,
`mapped_column`, `ForeignKey`, `UUID`, …) even though each block is a different
table schema. A cluster is suppressed when every member is either a single
`mapped_column(...)` call or a contiguous block whose every statement is a
`mapped_column` declaration, and at least two members declare different
attribute-name sets. Each model owns its own columns, so the declarations are
schema data rather than extractable logic.

### [CLONE-NOISE-PY-STRENUM-CLASS-SHAPE] StrEnum class shape
A `class X(StrEnum)` (or `class X(str, Enum)`) declaration always has the same
shape — an optional docstring followed by member assignments — so after
identifier normalisation unrelated enums cluster as duplicates, yet each enum is
a closed discriminator the program depends on by name. A cluster is suppressed
when every member is exactly one class definition whose superclass list names
`StrEnum` (or both `str` and `Enum`) and whose body contains only a docstring and
assignment statements. Distinct enum vocabularies are not extractable
duplication.

### [CLONE-NOISE-PY-PYDANTIC-PARTIAL] Pydantic partial-update mirror
Pydantic models commonly ship a `XCreate` model and a matching `XUpdate` mirror
in which every field is `T | None = None`, because Pydantic has no native
partial/PATCH model; after identifier normalisation the mirror clusters with its
source model. A cluster is suppressed when every member is a `BaseModel` subclass
whose body consists solely of an optional docstring and field declarations of the
form `name: T | None = None` (PEP 604 unions or `Optional[T]`) with at least one
such field. The mirror is mandated by the framework, not extractable duplication.

### [CLONE-NOISE-PY-MODULE-PREAMBLE] Module preamble definition runs
The sibling-window fingerprinter can emit one fingerprint over a contiguous run
of two or more module-level definitions, so two test modules whose opening
helpers share definition count and shape cluster at
`structural=1.00, token_jaccard=1.00` even though every helper body differs — the
matched unit is a "block of declarations", not a coherent code unit. A cluster is
suppressed when it spans at least two files, every member's range covers a run of
two or more sibling top-level function/decorated definitions, and no two members
share identical concatenated definition bodies. Keying on body divergence rather
than name divergence keeps a verbatim or renamed copy (whose bodies stay
byte-identical) visible as real duplication.

### [CLONE-NOISE-PY-MODULE-CONSTANT-TABLE] Module-level constant tables
A Python module that is just a run of module-level `NAME = <literal>` constant
assignments — a table of SQL query strings, registry values, or config
defaults — normalises to the same structural subtree as any other such table once
identifiers, literals, and comments are stripped, so two unrelated tables reach
`structural=1.00, token_jaccard=1.00`. A cluster is suppressed when every
member's reported range, at module top level, covers only comments, docstrings,
and bare-name constant assignments to plain literal values (with at least one
constant present), and the members differ in raw bytes. Interpolated f-strings or
any call/name/attribute right-hand side disqualify a member, and the
byte-divergence requirement keeps a constants module copied verbatim across files
visible.

### [CLONE-NOISE-PY-WORKSPACE-LOCAL-MIRROR] Workspace-local schema mirror (out of scope)
When a sandboxed workspace cannot import from the backend, it must keep a local
mirror of the backend's schemas, producing genuine cross-tree duplication whose
architectural justification is invisible to the analyser. This case is
deliberately **out of scope** for an automatic noise filter — no shape heuristic
can recover the import constraint. Users suppress the false positive through
configuration: an [exclusion.md §EXCLUSION-CONFIG](exclusion.md#exclusion-configuration)
`[language.python] exclude` glob over the workspace mirror path (e.g.
`exclude = ["workspaces/**/*"]`). This ID records the decision *not* to build a
generic filter, so reviewers do not re-litigate it.

## Rust idioms

### [CLONE-NOISE-RUST-LANGPARSER] LanguageParser trait adapters
Every first-party Rust language plug-in implements the same `LanguageParser`
trait, so each adapter carries an identical method outline (`id`,
`file_extensions`, `grammar`, `parse_and_normalize`) even though the bodies
differ entirely; the shape is mandated by the trait contract, not extractable
logic. A cluster is suppressed when it spans at least two files, every member is
an `impl LanguageParser for …` block whose directly declared methods match that
canonical set, and at least two members' impl bodies differ in raw bytes. The
byte-divergence guard keeps a verbatim-copied impl visible.

### [CLONE-NOISE-RUST-DECL] Bodiless top-level declarations
Bodiless Rust top-level declarations — `mod NAME;`, `use …;`, `pub use …;` —
cluster across module registries because Rust cannot macro-generate module
statements, so they must be written literally; they are language scaffolding, not
logic. A cluster is suppressed when every member is a single bodiless `mod_item`
or `use_declaration` whose byte range hugs the declaration (allowing a small
leading slack for a `pub` / `pub(crate)` modifier) and at least two members
differ in their declared identifier or use-path. The differing-identifier
requirement distinguishes scaffolding from a verbatim copy of one identical
declaration.

### [CLONE-NOISE-RUST-ITER-COLLECT] iter/map/collect idiom
The Rust chain `<expr>.iter().map(|x| x.field.method(...)).collect()` is a pure
language idiom that recurs across unrelated element types; extracting it would
require a trait on every unrelated struct, not deduplication. A cluster is
suppressed when it spans at least two files and every member contains a
`.collect()` (or `.collect::<…>()`) call whose receiver is `.iter().map(closure)`,
where the closure takes a single identifier parameter and its body is exactly a
`param.field.method(...)` projection. The idiom is language plumbing, not
actionable duplication.

### [CLONE-NOISE-RUST-MATCH-DISPATCH] Match-dispatch arm runs
A dispatch `match` routes distinct command keys to distinct handlers, but after
identifier and literal normalisation every arm collapses to the same
`<path>::<ident> => Ok(<call>(...))` shape, so the sibling-window pass matches one
window of arms against another within the same `match`. A cluster is suppressed
when every member is a contiguous run of `match` arms under a single
`match_block`, the matched arm patterns across the cluster are pairwise distinct
(a routing table, not repeated arms), and at least two members differ in raw
bytes. The distinct-pattern and byte-divergence guards keep a verbatim
copy-pasted run of arms visible as a genuine clone.

## Performance

### [CLONE-NOISE-REPARSE-CACHE] Parse-once filter cache
The cluster-noise and role-compatibility filters re-parse each member's original
source to inspect the real CST, which would be ruinously expensive if repeated
per cluster — a single large generated file (e.g. a 30k-line FFI binding
clustered hundreds of ways) would be re-parsed once per cluster and dominate
analysis time. The render pass therefore holds a per-report parse cache that
parses each source file's tree-sitter CST at most once, keyed by file id and
shared across every cluster's checks. Two further economies build on it:
language-specific filters are dispatched by language so a cluster is only walked
by matchers that can fire for it, and a cluster whose every occurrence sits in a
report-hidden path is dropped on the cheap path before any re-parse runs.
