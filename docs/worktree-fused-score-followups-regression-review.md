# Regression and test-weakening review: `worktree-fused-score-followups`

## Result

This review found eight concrete regression findings and eight assertion holes.
No corpus tests were run.

The most urgent defects are silent false negatives: several noise filters make
one cluster-wide decision from an existential difference, so a third, different
member can cause a genuine byte-identical pair in the same cluster to disappear.
There are also paths that assign perfect confidence to semantically different
code, serve incompatible cached reports, and omit visible report changes from
live deltas.

## Production regressions

### P0 — Operators are absent from the evidence and semantic changes can score `fused = 1.0`

Evidence:

- `crates/deslop-core/src/lang/shared.rs:75-98,131-175` recursively visits only
  `named_children`. Tree-sitter represents operators such as `+`, `-`, `==`,
  `!=`, `&&`, and `||` as anonymous tokens in the affected grammars.
- `crates/deslop-core/src/tokens.rs:216-260` derives content evidence only from
  the normalized identifier/literal frontier. It has no operator evidence.
- `crates/deslop-core/src/buckets/gate.rs:181-215` permits a shape-saturated,
  agreement-saturated non-verbatim match to render `fused = 1.0`.

Concrete failure:

```rust
fn adjust(a: i32, b: i32) -> i32 { a + b }
fn adjust(a: i32, b: i32) -> i32 { a - b }
```

The identifiers and literals are identical. The only behavioral difference is
the anonymous operator, so structural, token, and content agreement can all be
`1.0` even though the bytes and behavior differ.

Required patch and regression pin:

- Preserve behavior-bearing anonymous tokens in normalization/content evidence
  while continuing to discard punctuation-only framing.
- Add focused parser/evidence cases for `+/-`, `==/!=`, and `&&/||`.
- Add a two-file fixture asserting an operator-only change never reaches the
  `identical` bucket or `fused = 1.0`. If operator drift remains reportable as a
  near clone, assert its exact non-saturated bucket and signal vector.

### P0 — Three suppression filters can erase an exact duplicate subgroup

All three filters apply a cluster-wide predicate. In each case, one varying
member is enough to hide every member, including an exact pair.

#### Literal-varying calls

- `crates/deslop-core/src/cluster_filters/calls.rs:25-40,400-423` treats “at
  least one later literal differs from the baseline” as sufficient variation.
- `crates/deslop-core/src/cluster_filters/mod.rs:172-176` then hides the entire
  cluster.

```python
# a.py and b.py are byte-identical
def emit():
    persist("invoice")

# c.py has the same normalized shape
def emit():
    persist("refund")
```

The third member can cause the A/B copy to vanish.

#### Python collection cells

- `crates/deslop-core/src/cluster_filters/python_collection_cells.rs:25-36`
  requires only one raw-text difference across members sharing a collection
  home, despite the module contract saying byte-identical repeated entries must
  surface.

```python
values = [
    normalize(invoice.amount),
    normalize(invoice.amount),
    normalize(refund.amount),
]
```

The third entry can hide the exact first/second pair.

#### Constant tables

- `crates/deslop-core/src/cluster_filters/constant_table.rs:37-45` hides a whole
  constant-only cluster when any raw snippets differ.

```python
# a.py and b.py are byte-identical
API_TIMEOUT = 30
MAX_RETRIES = 5

# c.py has the same normalized assignment-run shape
PRIMARY_COLOR = "blue"
FONT_SIZE = 12
```

The unrelated table can erase the proven A/B copy.

Required patch and regression pins:

- Partition candidate members into the actual noise family before suppression;
  never discard a byte-identical subgroup because another member differs.
- Add one three-member integration test per filter. Each test must assert that
  A/B form exactly one visible size-2 `identical` cluster, both exact paths and
  ranges are present, C is absent from that cluster, signals are saturated for
  A/B, and duplicated LOC is positive.

### P0 — Cache seeding accepts incompatible reports and then marks them fresh

Evidence:

- `crates/deslop-core/src/live/session_helpers.rs:286-304` accepts any cached
  JSON that deserializes as `Report`; it validates no tool version, `min_nodes`,
  configuration, embedding mode/provider, or workspace identity.
- `crates/deslop-core/src/live/session.rs:184-211` receives those current
  settings but loads the cache by root alone.
- `crates/deslop-core/src/live/session.rs:303-328` and
  `crates/deslop-core/src/live/freshness.rs:59-69` record current mtimes against
  occurrences from the accepted cached report. Stale offsets and clusters can
  therefore be treated as fresh until replacement analysis completes.
- `crates/deslop-lsp/tests/state_file_and_ipc.rs:79-117,656-696` currently
  requires a synthetic `tool_version: "test-cache"`, `min_nodes: 4` report to be
  served even though the live session uses different settings. The test blesses
  the compatibility bug.

Required patch and regression pin:

- Persist a cache compatibility key containing at least schema/tool version,
  normalized root identity, `min_nodes`, effective config digest, and embedding
  mode/provider identity. Reject a mismatch before constructing the session.
- Replace the permissive cache test with table-driven cases that mutate each
  compatibility component and assert `try_seeded_from_cache` returns `None`.
  Keep one matching-cache case that asserts immediate service.

### P0 — Live deltas omit user-visible cluster changes

Evidence:

- `crates/deslop-core/src/delta.rs:97-110` omits `bucket`, `category`,
  `occurrences_total`, `occurrences_truncated`, `intersects_diff`, and
  `is_newly_introduced` from cluster equality.
- `crates/deslop-core/src/delta.rs:113-120` omits `agreement`,
  `rename_consistency`, and `literal_fraction` from signal equality.
- `crates/deslop-core/src/delta.rs:125-137` omits occurrence `start_line`,
  `end_line`, and `in_diff`.

A stable cluster ID can change bucket, evidence, displayed lines, diff status,
or truncation while `ReportDelta::between` emits no `clusters_updated`. Live
clients then retain stale data.

Required patch and regression pin:

- Compare the complete serialized/user-visible semantic payload, excluding
  only fields explicitly proven irrelevant to subscribers.
- Add a table-driven unit test that mutates every `ReportCluster`,
  `ReportSignals`, and `ReportOccurrence` field one at a time and requires
  exactly that cluster ID in `clusters_updated`.
- Add one LSP integration case for a same-ID update. The current notification
  test at `crates/deslop-lsp/tests/notifications.rs:48-77` covers removal only.

### P1 — A majority verbatim subgroup certifies unrelated cluster members

Evidence:

- `crates/deslop-core/src/content.rs:222-233,387-401` marks an entire cluster
  `verbatim_dominated` when its largest verbatim family is merely a strict
  majority.
- `crates/deslop-core/src/content.rs:297-311` then forces the whole cluster's
  agreement to `1.0`.
- `crates/deslop-core/src/buckets/gate.rs:181-215` can use that value to
  saturate fused confidence for every member.

A five-member cluster containing three identical copies plus two different,
shape-compatible members can therefore give all five the proof earned only by
the three-copy subgroup.

Required patch and regression pin:

- Split the verbatim family before scoring, or compute evidence per member/pair
  so minority members cannot inherit majority proof.
- Add a five-member 3+2 fixture. Assert either a size-3 saturated cluster for
  the exact copies plus a separate result, or a mixed cluster whose agreement
  and fused score are both below `1.0`.

### P1 — Literal-call suppression ignores duplicated non-call logic

Evidence:

- `crates/deslop-core/src/cluster_filters/calls.rs:105-117` decides from the
  extracted call sequence alone.
- `crates/deslop-core/src/cluster_filters/calls.rs:122-165` collects calls but
  never checks the executable AST residue around them.

```python
def invoice_total(subtotal, tax):
    total = subtotal + tax
    record_metric("invoice")
    return total

def refund_total(amount, fee):
    result = amount + fee
    record_metric("refund")
    return result
```

The literal-varying call can cause the copied calculation and return flow to be
hidden.

Required patch and regression pin:

- After removing accepted call expressions, require the remaining AST to match
  a narrow, explicit inert-scaffolding allowlist.
- Add a two-function fixture asserting this business-logic clone remains
  visible with exact paths/ranges and positive duplicated LOC. Keep a pure
  call-scaffolding fixture that remains hidden.

### P1 — Python sibling-cell suppression protects only lambdas

Evidence:

- `crates/deslop-core/src/cluster_filters/python_collection_cells.rs:25-36`
  suppresses raw-different siblings in one collection.
- `crates/deslop-core/src/cluster_filters/python_collection_cells.rs:52-62`
  exempts only a subtree containing `lambda`. Calls, operators,
  comprehensions, and conditional expressions remain suppressible.

```python
payload = [
    normalize(invoice.amount) + apply_tax(invoice.tax),
    normalize(refund.amount) + apply_tax(refund.tax),
]
```

This is extractable repeated logic, not inert record data, but the current
predicate hides it.

Required patch and regression pin:

- Replace the lambda blacklist with a positive AST allowlist for inert cells.
- Add focused call, operator, comprehension, and conditional-expression cases;
  assert exact visible occurrences for each. Keep a simple identifier/literal
  record-cell case that remains hidden.

### P1 — The polymorphic comparator aliases different implementations

Evidence:

- `crates/deslop-core/src/cluster_filters/body_shape.rs:26-56` compares named
  node kinds only. It deliberately erases identifier/literal text and also
  loses anonymous operators.
- `crates/deslop-core/src/cluster_filters/polymorphic.rs:49-64` uses that stream
  as the sole body-difference check.
- `crates/deslop-core/src/cluster_filters/mod.rs:398-416` uses the same stream
  for signature-only suppression.

Same-signature implementations that use different backends but share one AST
shape can compare equal and escape the filter:

```python
return self.container.run(job)
return self.machine.launch(job)
```

Required patch and regression pin:

- Extend the comparator with behavior-bearing operators and a normalized
  vocabulary relation capable of distinguishing unrelated backend access while
  retaining consistent-renaming clones.
- Add same-shaped backend implementations and operator-only implementations to
  the polymorphic fixture. Assert no cross-file cluster for them and, in the
  same scan, assert the consistently renamed control remains visible with its
  exact bucket, size, paths, and signals.

## Test weakening and vacuous assertions

### P0 — Noise-filter tests do not prove the target filter fired

`crates/deslop/tests/common/negative_pin.rs:47-59` asserts that the target files
do not span a visible cluster and that the report-wide `clusters_hidden` is at
least one. An unrelated hidden cluster satisfies the counter. The positive
control at lines 65-87 proves only that some unrelated clone is detectable.

The same pattern appears in `crates/deslop/tests/fused_golden_bands.rs:291`: if
the target family disappears, any hidden cluster can satisfy the fallback.

Required assertion:

- Record test-visible suppression provenance containing filter reason and exact
  member paths/ranges. Require the target candidate to exist before filtering
  and to be hidden by the expected filter. A global hidden count is not an
  acceptable substitute.

### P0 — The #69/#421 test passes if detection goes blind

`crates/deslop/tests/python_issue_69_abstract_method.rs:16-54` loops over an
empty result, then asserts `cluster_count == 0`. It has no same-run positive
control and no `files_analysed` assertion. Parser failure, candidate-generation
failure, or global suppression all pass.

`crates/deslop/tests/polymorphic_gate_hides_rename_clone.rs:24-106` describes a
same-run two-sided contract but performs two separate scans; the all-empty
contract scan is still unguarded by a local positive candidate.

Required assertion:

- Put the noise family and a byte-identical control in one fixture and one
  scan. Assert exact files analysed, exact control paths/ranges, size, bucket,
  signals, and duplicated LOC, plus target-specific suppression provenance.

### P0 — The saturation invariant accepts proof belonging to only two members

`crates/deslop/tests/fused_golden_invariants.rs:189-205` accepts a saturated
non-`identical` cluster if any byte-identical occurrence pair exists. It does
not require every saturated member to belong to that verbatim family. This test
therefore passes the 3+2 majority-certification regression described above.

Required assertion:

- For `fused = 1.0`, require all shown occurrences to be byte-equivalent to the
  canonical occurrence, or require the cluster to have been split to the exact
  verbatim family.

### P1 — Corpus determinism compares only cluster IDs

`crates/deslop/tests/corpus_repos.rs:119-170` says the reports must agree but
compares only ordered ID vectors. `duplication_percent` is printed, not
asserted, and `filter_map` silently drops clusters missing an ID. Stable IDs
with changed spans, buckets, signals, ranks, hidden flags, or metrics pass.

Required assertion:

- Compare canonical full semantic reports, stripping only genuinely
  nondeterministic measurements. Treat a missing ID as malformed input.
- Unit-test equal IDs with changed occurrence ranges, bucket/signals, order,
  and duplication percentage; every mutation must fail determinism.

### P1 — Curated recall identifies a file pair, not the curated clone

`crates/deslop-test-support/src/corpus_confidence.rs:228-305,342-378` accepts any
qualifying cluster spanning the listed files. The manifests describe verified
line ranges only in prose. If the verified 137-line clone disappears but an
unrelated small clone remains between the same files, recall stays green.

The Type-1 implementation has no direct unit coverage: the tests under
`crates/deslop-test-support/src/corpus_confidence/tests/curated.rs` exercise the
Type-2 path. The cheap contract in
`crates/deslop/tests/corpus_manifest_contract.rs:58-167` protects only
`must_find_type2`; deleting Flutter's three Type-1 entries is not rejected.

Required assertion:

- Give each curated entry machine-readable start/end lines or a normalized
  content digest and expected occurrence count. Match that exact clone, not
  merely its files.
- Add negative unit cases for an unrelated clone in the same file pair, wrong
  range/digest, hidden occurrence, wrong bucket, rank beyond the ceiling,
  duplicate file names, and malformed entries.
- Add a non-vacuity/shape contract for Type-1 `must_find` ground truth.

### P1 — Missing signal fields are silently converted to passing zeroes

`crates/deslop-test-support/src/corpus_confidence.rs:96-134,436-443` defaults a
missing or non-numeric signal to `0.0`. Deleting `signals.fused` and the axes can
make the bounded-max invariant compare zero to zero and pass.

`crates/deslop/tests/common/mod.rs:238-245` has the same default. Assertions
whose expected value is zero, including parts of the Type-1 signal contract,
therefore accept a missing field.

Required assertion:

- Parse every required signal as a present, numeric, finite value in `[0, 1]`
  before checking relationships or expected values.
- Add missing, `null`, string, `NaN`/non-finite, and out-of-range negative cases.

### P1 — The precision check can disable itself and inspects only one occurrence

`crates/deslop-test-support/src/corpus_precision.rs:43-64` does no work when
`top_n` is zero or the forbidden-supertype list is empty/malformed; non-string
entries are silently discarded. Lines 67-113 judge only the first occurrence,
so a forbidden declaration later in the same cluster passes depending on
ordering.

Required assertion:

- Reject zero `top_n`, an empty list, and every non-string/blank forbidden
  supertype as invalid test configuration.
- Inspect every shown occurrence. Add a case where only the second occurrence
  declares the forbidden supertype and require a failure.

### P1 — Six expensive corpus scans have no accuracy oracle

The manifests for Django, F#, Hugo, Jellyfin, Laravel, and React each have:

- zero `must_find` entries;
- zero `must_find_type2` entries; and
- no `must_not_rank_first` precision assertion.

Their current corpus tests can enforce resource ceilings, but cannot fail for a
recall or ranking regression—even a report containing zero useful findings.

Required assertion before spending on those scans:

- Curate at least one exact Type-1 or Type-2 clone per repository, with
  machine-readable location/digest evidence and an expected bucket/rank bound.
- Where the repository has known framework boilerplate, add a concrete
  forbidden-top-rank precision oracle.
- Until a repository has an accuracy oracle, its run must not be cited as
  regression coverage for detection quality.

## Action order

1. Fix the three mixed-cluster erasures and add the three-member subgroup pins.
2. Stop operator-only changes and majority verbatim subgroups from earning
   cluster-wide perfect confidence.
3. Reject incompatible live cache seeds and make delta equality cover the full
   visible payload.
4. Replace global/empty negative assertions with target-specific provenance and
   same-run positive controls.
5. Make corpus recall identify exact clones, make required signals fail closed,
   and add accuracy oracles for the six uncurated repositories before relying
   on their expensive runs.
