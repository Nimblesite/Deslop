# Root cause: the fused score carries no information

**One mechanism explains 8 of the 28 open bugs** — 2 showstoppers and 6 critical. It is the highest-leverage fix in the tracker.

## The defect

`crates/deslop-core/src/pair.rs:72`

```rust
let raw = self.structural + self.token_jaccard + self.embedding_cos;
raw.clamp(0.0, 1.0)
```

Three problems compound:

**1. The signals are not independent.** `token_jaccard` is a MinHash over "k-grams of **normalised node kinds**" (`lsh.rs:5`) — the same identifier- and literal-stripped representation that `structural` hashes (`lang/mod.rs:5`). One is a k-gram view of exactly what the other hashes. `token_jaccard` cannot disconfirm `structural`; it restates it.

**2. The third signal is off by default.** `--embeddings` defaults to `off` (`deslop/src/main.rs:90`), so `embedding_cos = 0.0` on every default run. The only signal that could separate *what the code means* from *what shape it has* never participates.

**3. Sum-then-clamp destroys the remaining discrimination.** `structural = 1.0` forces `fused = 1.0` no matter what else is true. Every false positive in the tracker reports `structural=1.00, token_jaccard=1.00, fused=1.00` — not because confidence is high, but because that is the only value the function can return once the shapes match.

The docstring cites an ensemble paper: *"averaging hurts; sum and max help."* That holds for **independent** ensemble members. Summing two correlated signals and clamping is the failure mode that literature warns about.

Net effect: **`fused` is a re-encoding of "the normalised shapes matched."** It is not a confidence score.

## What it explains

Normalisation maps every numeric, boolean, null and string literal to `__literal__` (`lang/dart.rs:13`) and flattens identifiers. Any two fragments sharing syntax therefore saturate all live signals:

| # | Sev | What collapses |
|---|---|---|
| [#331](https://github.com/Nimblesite/Deslop/issues/331) | showstopper | Flutter `StatefulWidget` declarations — identifiers stripped, so 453 distinct widgets become one cluster |
| [#336](https://github.com/Nimblesite/Deslop/issues/336) | showstopper | F# numeric arrays — every literal becomes `__literal__`, so all integer tables are identical |
| [#283](https://github.com/Nimblesite/Deslop/issues/283) | critical | Unrelated object-literal tables |
| [#284](https://github.com/Nimblesite/Deslop/issues/284) | critical | Unrelated test scenarios sharing scaffolding |
| [#285](https://github.com/Nimblesite/Deslop/issues/285) | critical | Tests sharing only an assertion idiom |
| [#103](https://github.com/Nimblesite/Deslop/issues/103) | critical | pytest idioms — `monkeypatch.setenv` chains, dict assertions |
| [#79](https://github.com/Nimblesite/Deslop/issues/79) | critical | Helper call sites differing only in literal arguments |
| [#71](https://github.com/Nimblesite/Deslop/issues/71) | critical | Same HTTP verb + status across different endpoints |

Every one is *shape matched, meaning differs*. That is precisely the distinction the current signal set cannot make.

## It also breaks rule zero

`CLAUDE.md` instructs every agent to branch on the fused score:

- `fused ≥ 0.85` → do not write the copy
- `0.6 ≤ fused < 0.85` → read the canonical occurrence, bias toward reuse
- `fused < 0.6` → author it

Any structural match returns exactly `1.0`. The middle band is unreachable except through LSH-only or embedding paths, and the top band fires on mandatory boilerplate. **The documented agent workflow cannot behave as written on the current engine.**

## Why fix this cluster first

- **Leverage**: 8 bugs, 2 of them showstoppers, one change site.
- **It is upstream of ranking.** Weight is a product of clone size, count and spanned LOC — all computed on clusters this function admits. Every precision fix downstream is compensating for it.
- **It unblocks measurement.** With `fused` pinned at 1.0, no threshold tuning, ranking policy or filter can be evaluated, because the input carries no signal to tune against.

## What a fix has to establish

Not prescribing an implementation, but any candidate must answer:

1. **Give the ensemble an independent member.** Two views of one normalised tree is one signal. Either a semantic signal participates by default, or the ensemble is not an ensemble.
2. **Stop clamping away the top of the range.** Saturation at the decision boundary is what makes every cluster look maximally confident.
3. **Preserve some literal information.** Collapsing all literals to `__literal__` is what makes distinct data tables identical (#336) and distinct call sites identical (#79). Type-2 detection needs identifier normalisation; it does not obviously need total literal erasure.

## How to verify a fix

`make test-corpus` already fails on #331 and #336 against real pinned repositories. Both must go green **without** the recall assertions regressing — `corpus_flutter_dart` asserts three hand-verified byte-identical duplicates are still found. That pairing is the point: precision fixes that silence false positives by suppressing real clones will fail the recall half.

Fix #301 (nondeterminism) first or in parallel. While identical runs disagree, no before/after measurement of this work is trustworthy.
