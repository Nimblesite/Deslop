# Pair evidence taxonomy

## [CLONE-BUCKETS-NORTH-STAR] Taxonomy explains an explicit pair

Deslop's clone taxonomy classifies two explicitly identified occurrences. It explains pair admission evidence to humans and agents. It is not a cluster identity, severity, facet, title, weight, or ranking input.

Cluster surfaces use the neutral title `Duplicate code` and render membership plus mass. Pair surfaces name both endpoints and may render the taxonomy below.

## [CLONE-BUCKETS] Canonical pair classifications

| Pair kind | Plain title | Technical label | Meaning for the exact pair |
|---|---|---|---|
| `Identical` | Identical code | Type-1 | The two raw slices are byte-equivalent after ASCII-whitespace folding. |
| `NearlyIdentical` | Nearly identical code | Type-2/3 | The pair has strong normalized shape or token evidence and content support. |
| `SameBehavior` | Same behavior, different code | Type-4 | The pair has strong embedding support despite low syntactic similarity. |
| `StructuralOnly` | Same shape, unsupported content | structural-only candidate | Normalized shape agrees but the required pair-content support is absent; classification does not admit the pair. |
| `LooselySimilar` | Weakly similar candidate | weak candidate | The measured pair lacks enough corroboration; classification does not admit the pair. |

Pair classification and pair admission are distinct outputs. [FUSED-STRATEGY-BOUNDED-MAX] alone decides `admitted`; a label never admits a pair and an admitted edge never donates its label to a component.

### [CLONE-BUCKETS-DUAL-LABEL] Pair-only labelling policy

An explicit visual pair view uses the plain title. Shared text uses `Plain title [technical label]`. Machine pair records carry the enum, title, technical label, endpoints, evidence, and admission result.

No surface may render a pair label without identifying both endpoints. Cluster cards, trees, diagnostics, reports, MCP cluster results, and AI cluster context use no pair title, evidence sentence, enum, or colour.

One core helper owns the pair enum's titles and technical labels. Pair UI and pair serializers reuse it; cluster renderers cannot call it.

### [CLONE-BUCKETS-ROUTING] Evidence to pair classification

Classification reads the same exact pair evidence as admission. It runs for explicit comparison whether the pair was admitted or rejected.

| Condition, evaluated top-down | Pair kind |
|---|---|
| Raw slices are byte-equivalent after ASCII-whitespace folding | `Identical` |
| `embedding_cos ≥ embedding_support_floor` and syntactic shape is low | `SameBehavior` |
| The pair is admitted through strong normalized shape or token evidence with applicable content support | `NearlyIdentical` |
| Normalized shape is strong but applicable content support fails | `StructuralOnly` |
| Otherwise | `LooselySimilar` |

All numeric thresholds are named configuration values defined by [FUSED-TUNING-LEVERS]. Cluster construction and ranking never call pair classification.

Embedding-carried pairs still obey [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]. Literal comparisons produced by the value-level join use raw-value equality for `Identical`; otherwise they are `NearlyIdentical` when admitted.

### [CLONE-BUCKETS-IDENTICAL] Identity is a pair proof

`Identical` requires byte-equivalence of the two compared raw slices after folding ASCII whitespace. Normalized structural and token equality are insufficient because normalization collapses identifiers and literals. Missing source bytes cannot prove identity. No component-wide identity label is inferred.

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

A closure-component cluster record does not carry this kind. Dedicated literal-finding records may carry it under [LITERAL-WIRE].

## [CLONE-TYPE-TAXONOMY] Academic reference

The Type-1 through Type-4 taxonomy is standard in clone-detection literature (Bellon/Koschke; Roy/Cordy). It describes a relation between code fragments, which is why Deslop applies it to pairs rather than transitive components.

- Type-1: identical code aside from layout and comments.
- Type-2: identical structure with identifier, literal, or type renaming.
- Type-3: Type-2 plus added, removed, or modified statements.
- Type-4: semantically equivalent code with different syntax or algorithms.

Embeddings provide optional Type-4 candidate and admission evidence. With embeddings off, that evidence is unavailable; deterministic pair admission continues unchanged on the other axes.
