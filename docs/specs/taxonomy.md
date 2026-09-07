# Clone taxonomy — pair classifications and the cluster kind

## [CLONE-BUCKETS-NORTH-STAR] Taxonomy explains an explicit pair, and folds onto the cluster

Deslop's clone taxonomy classifies two explicitly identified occurrences. It explains pair admission evidence to humans and agents. It is never a weight, a ranking input, or a mass input.

A cluster is many pairs, so no single pair's label describes it. Every cluster instead carries a **clone kind** ([CLONE-KIND-FOLD]): the weakest pair classification between its canonical occurrence and any other member. Cluster surfaces title the cluster by that kind and colour it by that kind ([CLONE-KIND-LABELS], [CLONE-KIND-COLOR]). Pair surfaces name both endpoints and may render the pair's own classification and evidence.

## [CLONE-KIND] The clone kind of a cluster

### [CLONE-KIND-FOLD] The kind is the weakest relation to the canonical occurrence

The engine stamps every reported cluster with one `kind`, computed after ranking and before rendering. For each occurrence other than the first, the engine measures the pair `(canonical, occurrence)` exactly as an explicit `pair/compare` would — the same structural, token, embedding, and content axes, the same admission algebra, the same classification — and reads that pair's classification. The cluster's kind is the weakest of those readings:

| Reading for some member | Kind the cluster can be at most |
|---|---|
| Every member is byte-identical to the canonical | `Identical` |
| Every member is an admitted near-copy; at least one is not byte-identical | `NearlyIdentical` |
| At least one member matches on embedding evidence alone | `SameBehavior` |
| At least one member shares only normalised shape and fails the content guard | `StructuralOnly` |
| At least one member is a looser admitted relation, or is not admitted against the canonical at all (welded in only through other members) | `LooselySimilar` |

Strength order, strongest first: `Identical`, `NearlyIdentical`, `SameBehavior`, `StructuralOnly`, `LooselySimilar`. The fold never averages, never sums, and reads no cluster quantity. Mass, rank, and rank band never read the kind ([RANK-MASS-SUM]). The embedding axis of each folded pair is the cosine the embedding pass measured for that pair, or zero when the pass never measured it — the same evidence admission saw, never a fresh embedding.

A whole-file or whole-class occurrence that differs from the canonical only in a wrapper's name is not byte-identical, and the cluster is `NearlyIdentical`. Identity is a fact about the compared slices, not about the method inside them ([CLONE-BUCKETS-IDENTICAL]).

**For AI.** Wire: `ReportCluster.kind` and `ClusterSummary.kind`, one of `identical`, `nearly_identical`, `same_behavior`, `structural_only`, `loosely_similar`. Code: `crates/deslop-core/src/pipeline/session/pair_compare/cluster_kind.rs` (`ClusterKindMeasurer`, a `cluster::ClusterKindJudge`), folded through `buckets::ClusterKind::weaker`, stamped in `cluster::build_ranked_fused_clusters`, carried by `report_render::cluster_to_report`. Tests: `buckets::tests`, `crates/deslop/tests/cli/bucket_groups.rs`, `crates/deslop/tests/csharp_type1_type2_byte_truth.rs`, the golden `report-golden/expected-report.json`.

### [CLONE-KIND-LABELS] One registry names each kind

| Kind | Title every cluster surface shows | Taxonomy name (tooltips, agent context) | HTML class suffix |
|---|---|---|---|
| `identical` | Identical code | Type-1 exact clone | `identical` |
| `nearly_identical` | Nearly identical code | Type-2/3 near-copy | `nearly-identical` |
| `same_behavior` | Same behavior, different code | Type-4 semantic clone | `same-behavior` |
| `structural_only` | Same shape, different content | structural-only match | `structural-only` |
| `loosely_similar` | Loosely similar code | weak Type-3 relation | `loosely-similar` |

The title names the relation and nothing else: no advice, no "safe to merge", no evidence sentence. The text report writes `kind=<wire label>`; the markdown, HTML, terminal summary, LSP diagnostic (`<title> × <count> — mass <mass>`), MCP summary, editor bubble, hover, tree row, webview, cluster document, and copy-for-AI payload all use the title from this table. The extension mirrors the table in `clients/vscode/src/types/report.ts`; its CLI parity test scans a fixture with the bundled CLI and asserts both sides print the same words.

**For AI.** Code: `crates/deslop-core/src/buckets.rs` (`ClusterKind::labels`, `KindLabels`, `wire_label`). TypeScript: `kindTitle`, `kindTaxonomy`, `CLUSTER_KINDS` in `clients/vscode/src/types/report.ts`; parity pinned by `clients/vscode/src/test/unit/kind.unit.test.ts`.

### [CLONE-KIND-COLOR] Colour follows the kind; the glyph follows the rank band

Every cluster surface has two visual channels and never mixes them:

- **Colour** is the clone kind, from one table. Crimson `#b3261e` is `identical` — the only kind whose evidence is character-by-character proof. Amber `#e8912d` is `nearly_identical`. Violet `#a98cff` is `same_behavior`. Muted `#a9a2a0` is `structural_only`. Blue `#00619e` is `loosely_similar`. The Top Offenders tree also gives each kind its own icon (`circle-filled`, `circle-large-filled`, `sparkle`, `circle-slash`, `circle-outline`) so kinds stay distinct without colour.
- **Glyph density** is the mass rank band ([SEVERITY-BAND]): `●●` worst, `●` top 10%, `◐` mid, `○` faint.

A shape-only family that ranks first by mass is muted; a byte-identical cluster in the faint tail is crimson. Rank never chooses a colour.

One table is the paint. The extension declares it once in `clients/vscode/src/design.ts` (`KIND_COLOR`); the tree reads the contributed VS Code theme colours `deslop.kind.*`, whose defaults in `package.json` are the same values; the webviews import that same table; the HTML report declares the same values as `--kind-*` CSS variables. The parity test in `kind.unit.test.ts` holds the copies together.

### [CLONE-KIND-TESTING] Acceptance

Tests assert: a byte-identical pair folds to `identical` and a renamed pair to `nearly_identical`, in rank order; a wrapper-renamed whole-class occurrence is `nearly_identical`, not `identical`; a shape-only rank-1 cluster is painted muted while a smaller byte-identical cluster is crimson; every kind has a distinct colour and icon; the tree, bubble, decoration, and webview read the one table; the extension's titles equal the CLI's; every renderer titles the cluster by its kind and none prints the retired neutral `Duplicate code`; no cluster surface renders pair evidence values.

## [CLONE-BUCKETS] Canonical pair classifications

| Pair kind | Plain title | Technical label | Meaning for the exact pair |
|---|---|---|---|
| `Identical` | Identical code | Type-1 | The two raw slices are byte-equivalent after ASCII-whitespace folding. |
| `NearlyIdentical` | Nearly identical code | Type-2/3 | The pair has strong normalized shape or token evidence and content support. |
| `SameBehavior` | Same behavior, different code | Type-4 | The pair has strong embedding support despite low syntactic similarity. |
| `StructuralOnly` | Same shape, unsupported content | structural-only candidate | Normalized shape agrees but the required pair-content support is absent; classification does not admit the pair. |
| `LooselySimilar` | Weakly similar candidate | weak candidate | The measured pair lacks enough corroboration; classification does not admit the pair. |

Pair classification and pair admission are distinct outputs. [FUSED-STRATEGY-BOUNDED-MAX] alone decides `admitted`; a label never admits a pair. An admitted edge never donates its label to a component: the component's kind is the fold over every member's comparison with the canonical occurrence ([CLONE-KIND-FOLD]), not any one edge.

### [CLONE-BUCKETS-DUAL-LABEL] Labelling policy

An explicit visual pair view uses the plain title. Shared text uses `Plain title [technical label]`. Machine pair records carry the enum, title, technical label, endpoints, evidence, and admission result.

No surface may render a pair's classification or evidence without identifying both endpoints. Cluster cards, trees, diagnostics, reports, MCP cluster results, and AI cluster context render the cluster's folded kind ([CLONE-KIND-LABELS]) and never a single pair's classification, evidence sentence, or measured values.

One core registry owns the titles and technical labels ([CLONE-KIND-LABELS]); pair UI, pair serializers, and cluster renderers all reuse it.

### [CLONE-BUCKETS-ROUTING] Evidence to pair classification

Classification reads the same exact pair evidence as admission. It runs for explicit comparison whether the pair was admitted or rejected.

| Condition, evaluated top-down | Pair kind |
|---|---|
| Raw slices are byte-equivalent after ASCII-whitespace folding | `Identical` |
| `embedding_cos ≥ embedding_support_floor` and syntactic shape is low | `SameBehavior` |
| The pair is admitted through strong normalized shape or token evidence with applicable content support | `NearlyIdentical` |
| Normalized shape is strong but applicable content support fails | `StructuralOnly` |
| Otherwise | `LooselySimilar` |

All numeric thresholds are named configuration values defined by [FUSED-TUNING-LEVERS]. Admission and ranking never read pair classification; the report build reads it only to fold the cluster kind after ranking ([CLONE-KIND-FOLD]).

Embedding-carried pairs still obey [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]. Literal comparisons produced by the value-level join use raw-value equality for `Identical`; otherwise they are `NearlyIdentical` when admitted.

### [CLONE-BUCKETS-IDENTICAL] Identity is a pair proof

`Identical` requires byte-equivalence of the two compared raw slices after folding ASCII whitespace. Normalized structural and token equality are insufficient because normalization collapses identifiers and literals. Missing source bytes cannot prove identity. A cluster is `identical` only when every member's slice is byte-equivalent to the canonical slice ([CLONE-KIND-FOLD]); nothing is inferred from normalised equality.

## [CLONE-CATEGORY-REGISTRY] Finding kinds do not classify closure components

Logic/data-table and literal-family kinds describe how a dedicated detector found a repetition. They may control detection-time visibility or an occurrence-level action, but they never classify a pair, appear as a cluster similarity label, or change mass.

| Finding kind | Wire label | Purpose |
|---|---|---|
| `Logic` | `logic` | Ordinary code repetition. |
| `DataTable` | `data` | Repeated data-table shape. |
| `MagicLiteral` | `magic_literal` | Repeated inline literal. |
| `ShadowedConstant` | `shadowed_constant` | Inline value already named by a constant. |
| `ConstantDuplicate` | `constant_duplicate` | Same constant declared repeatedly. |
| `ConstantDrift` | `constant_drift` | Same constant name resolves to conflicting values. |
| `ConstantAlias` | `constant_alias` | One value has several constant names. |

A closure-component cluster record does not carry this finding kind; it carries the clone kind of [CLONE-KIND-FOLD]. Dedicated literal-finding records may carry the finding kind under [LITERAL-WIRE].

## [CLONE-TYPE-TAXONOMY] Academic reference

The Type-1 through Type-4 taxonomy is standard in clone-detection literature (Bellon/Koschke; Roy/Cordy). It describes a relation between code fragments, which is why Deslop measures it on pairs and folds it onto a component only through the component's canonical occurrence ([CLONE-KIND-FOLD]).

- Type-1: identical code aside from layout and comments.
- Type-2: identical structure with identifier, literal, or type renaming.
- Type-3: Type-2 plus added, removed, or modified statements.
- Type-4: semantically equivalent code with different syntax or algorithms.

Embeddings provide optional Type-4 candidate and admission evidence. With embeddings off, that evidence is unavailable; deterministic pair admission continues unchanged on the other axes.
