# Examples

Substantial fixtures that exercise every clone type CodeDedup can
detect. Each subfolder is a self-contained scenario. Run CodeDedup
against a folder to see the clusters.

```bash
# Deterministic passes only (structural + LSH) — misses Type-4:
cargo run --release -- examples/csharp/repository --min-nodes 15

# With embeddings — surfaces Type-4 (semantic) clones:
cargo run --release -- examples/csharp/repository --min-nodes 15 \
    --embeddings required
```

See [`docs/specs/REPORTING-CONTEXT.md`](../docs/specs/REPORTING-CONTEXT.md)
for signal semantics. Each scenario below is annotated with the
expected cluster *taxonomy* — Type-1 (identical), Type-2 (renamed
identifiers), Type-3 (near-miss, structurally similar), Type-4
(semantically equivalent, syntactically different).

## C#

### [`csharp/repository/`](csharp/repository/)

CRUD repositories for three entities plus a LINQ rewrite.

| Files | Relation | Expected signals |
|---|---|---|
| `UserRepository.cs` ↔ `ProductRepository.cs` | **Type-2** — identical shape, renamed identifiers | `structural=1.0`, `token_jaccard=1.0` |
| `UserRepository.cs` ↔ `OrderRepository.cs` | **Type-3** — same shape plus cache-invalidation hook | `structural=0.0`, `token_jaccard≈0.9`, high `embedding_cos` |
| `ProductRepository.cs` ↔ `ProductRepositoryLinq.cs` | **Type-4** — imperative vs LINQ | `structural=0.0`, low `token_jaccard`, high `embedding_cos` |

### [`csharp/validators/`](csharp/validators/)

Email + credit-card validators, each implemented three ways.

| Files | Relation | Expected signals |
|---|---|---|
| `EmailValidatorImperative.cs` ↔ `EmailValidatorRegex.cs` ↔ `EmailValidatorParser.cs` | **Type-4** family — imperative vs regex vs parser | only `embedding_cos` |
| `CreditCardValidatorLuhn.cs` ↔ `CreditCardValidatorFunctional.cs` | **Type-4** — Luhn imperative vs functional | only `embedding_cos` |

### [`csharp/collections/`](csharp/collections/)

Common collection aggregates and group-bys, imperative vs LINQ.

| Files | Relation | Expected signals |
|---|---|---|
| `StatisticsImperative.cs` ↔ `StatisticsFunctional.cs` | **Type-4** — mean/variance/max via `for` vs `Sum`/`Average`/`Max` | only `embedding_cos` |
| `GroupByImperative.cs` ↔ `GroupByLinq.cs` | **Type-4** — dictionary vs `GroupBy().ToDictionary()` | only `embedding_cos` |

### [`csharp/async_patterns/`](csharp/async_patterns/)

Same two file-processing operations in three concurrency idioms.

| Files | Relation | Expected signals |
|---|---|---|
| `FileProcessorSync.cs` ↔ `FileProcessorAsync.cs` | **Type-3 / Type-4** — same loops, `await` threaded through | high `embedding_cos`, moderate `token_jaccard` |
| `FileProcessorSync.cs` ↔ `FileProcessorTaskContinuation.cs` | **Type-4** — imperative vs Task.ContinueWith | only `embedding_cos` |

## Rust

### [`rust/iterators/`](rust/iterators/)

Five aggregates (sum, product, count_positive, max_value,
running_total) in three styles.

| Files | Relation | Expected signals |
|---|---|---|
| `aggregates_loop.rs` ↔ `aggregates_iter.rs` | **Type-4** — `for` loops vs iterator chains | only `embedding_cos` |
| `aggregates_loop.rs` ↔ `aggregates_recursive.rs` | **Type-4** — loops vs tail recursion | only `embedding_cos` |
| `aggregates_iter.rs` ↔ `aggregates_recursive.rs` | **Type-4** — iterator chains vs recursion | only `embedding_cos` |

### [`rust/error_handling/`](rust/error_handling/)

Three parsers returning success / failure via different idioms.

| Files | Relation | Expected signals |
|---|---|---|
| `parse_option.rs` ↔ `parse_result.rs` | **Type-3** — same walk, `Option::None` vs `Result::Err` | high `token_jaccard`, high `embedding_cos` |
| `parse_option.rs` ↔ `parse_sentinel.rs` | **Type-4** — idiomatic vs sentinel-values | only `embedding_cos` |

### [`rust/state_machines/`](rust/state_machines/)

Traffic-light FSM represented three ways.

| Files | Relation | Expected signals |
|---|---|---|
| `traffic_light_enum.rs` ↔ `traffic_light_bool.rs` | **Type-4** — enum + match vs bool flags | only `embedding_cos` |
| `traffic_light_enum.rs` ↔ `traffic_light_table.rs` | **Type-4** — enum + match vs lookup table | only `embedding_cos` |
| The `run` / `ticks_until` methods across all three | **Type-2** — identical shells around different `next` | `structural=1.0` |

### [`rust/parsing/`](rust/parsing/)

CSV row parser in three implementations.

| Files | Relation | Expected signals |
|---|---|---|
| `csv_hand.rs` ↔ `csv_state.rs` | **Type-4** — ad-hoc bool vs explicit state enum | only `embedding_cos` |
| `csv_hand.rs` ↔ `csv_split.rs` | **Type-3** — same intent, naïve implementation misses edge cases | moderate `token_jaccard`, high `embedding_cos` |

## Python

### [`python/data_processing/`](python/data_processing/)

Five data-pipeline operations in three styles.

| Files | Relation | Expected signals |
|---|---|---|
| `pipeline_loop.py` ↔ `pipeline_comprehension.py` | **Type-4** — `for` loops vs list comprehensions + `sum` | only `embedding_cos` |
| `pipeline_loop.py` ↔ `pipeline_generator.py` | **Type-4** — loops vs `functools.reduce` | only `embedding_cos` |

### [`python/algorithms/`](python/algorithms/)

Factorial / Fibonacci / binomial recurrences, three ways.

| Files | Relation | Expected signals |
|---|---|---|
| `recurrences_recursive.py` ↔ `recurrences_iterative.py` | **Type-4** — recursion vs iteration | only `embedding_cos` |
| `recurrences_recursive.py` ↔ `recurrences_memoised.py` | **Type-3** — identical body, `@lru_cache` added | `structural` close to 1.0, `embedding_cos` close to 1.0 |

### [`python/apis/`](python/apis/)

HTTP client with identical surface, three HTTP libraries.

| Files | Relation | Expected signals |
|---|---|---|
| `client_requests.py` ↔ `client_urllib.py` | **Type-4** — request/session vs stdlib urllib | only `embedding_cos` |
| `client_requests.py` ↔ `client_httpx.py` | **Type-3** — shape nearly identical, different library | high `token_jaccard`, high `embedding_cos` |

### [`python/transforms/`](python/transforms/)

Four text-transformation steps composed three ways.

| Files | Relation | Expected signals |
|---|---|---|
| `text_pipeline_class.py` ↔ `text_pipeline_functional.py` | **Type-4** — stateful chain vs `reduce` | only `embedding_cos` |
| `text_pipeline_class.py` ↔ `text_pipeline_decorator.py` | **Type-4** — class chain vs decorator registry | only `embedding_cos` |
| Each `strip_punctuation` / `deduplicate_words` body | **Type-2** across files | `structural=1.0` |

## What "only `embedding_cos`" means in practice

When the only non-zero signal is `embedding_cos`, the deterministic
two-pass pipeline (structural hash + token LSH) cannot detect the
clone. These clusters appear only when the pipeline is invoked with
`--embeddings required` (or `--embeddings auto` against a reachable
provider). Running without embeddings on any scenario in this folder
misses the Type-4 families entirely.
