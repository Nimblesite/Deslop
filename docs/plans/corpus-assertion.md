# Corpus gate — stop lying, assert everything

The real-repository gate is specified in [`corpus.md`](../specs/corpus.md). This plan has one job:

> **A green corpus run must mean something, and it must never claim more than it earned.**

Fixture repos prove the pipeline runs. Only the corpus can prove it is *right* — and right now it barely tries. This is not fused-score work ([`fused-score-followups.md`](fused-score-followups.md) owns that); it is the instrument every accuracy claim in that plan depends on.

## Part 1 — Where the gate lies today

Each row is a place the suite reports more confidence than it has. **Every one is a green run that would survive a broken detector.**

### L1. Five of nine repositories assert nothing

#347 is fixed — the gate boots and ran green on 15, 16 and 17 Aug. That green is not evidence. The last run printed:

```
!! tokio: ACCURACY UNASSERTED — no curated duplicates and no ranking rule.
!! nest:  ACCURACY UNASSERTED — ...
```

Measured against the manifests, not the ledger:

| repo | language | `must_find` | `must_find_type2` | `must_not_rank_first` | determinism |
|---|---|---|---|---|---|
| flutter | dart | 3 | 0 | ✅ | — |
| nest | typescript | 0 | 2 † | — | ✅ |
| tokio | rust | 0 | 1 † | — | — |
| jellyfin | csharp | 0 | 0 | — | ✅ |
| django | python | 0 | 0 | — | — |
| react | javascript | 0 | 0 | — | — |
| laravel | php | 0 | 0 | — | — |
| hugo | go | 0 | 0 | — | — |
| fsharp | fsharp | 0 | 0 | — | — |

† on PR #392 only; `main` has zero.

Six of eight languages carry no curated ground truth of any kind. **A scan that returned an empty report would pass those five repositories.**

### L2. 🛑 Nothing asserts the scan found any files

`files_analysed` is *printed* by `report_measurements` ([`corpus_repos.rs:244`](../../crates/deslop/tests/corpus_repos.rs#L244)) and **asserted nowhere**. A repository that analyses as **zero files** produces a clean report, exit code 0, and a green corpus gate on all nine repositories.

That is not hypothetical — it is exactly #342, which shipped: a repo under any folder named `dist`/`build`/`target` analysed as zero files. The corpus gate, the one instrument built to catch a total false negative, could not see it.

### L3. 🛑 #401 — the only curated precision check matches raw source text

[`corpus_repos.rs:320`](../../crates/deslop/tests/corpus_repos.rs#L320) is `text.contains(shape)` against `forbidden_top_shapes` — text pattern matching on source code, which `CLAUDE.md` prohibits outright. Unsound in both directions:

- **False positive in the gate** — the string matches inside a comment, doc comment, or string literal, so a legitimate cluster that merely *mentions* `extends StatefulWidget` is reported as boilerplate.
- **False negative in the gate** — a declaration written `extends  StatefulWidget`, or wrapped across a line, is not matched, so the boilerplate it exists to catch walks straight past.

This is the check that carries #331 and #336. **#331 was closed against it.**

### L4. Precision has no curated surface at all

Seven open false-positive issues (#71 #79 #103 #283 #284 #285 #336) all say *"these are not duplicates and Deslop clustered them"*. **No manifest field can express that.** `must_find` is recall-only; `must_not_rank_first` guards only the head of the report, only by shape string, only on flutter. Not one of those seven can be pinned on the repository it was reported against.

### L5. `must_find` asserts less than `must_find_type2`

`check_recall` asserts only that *some* cluster spans the curated paths. A 137-line byte-identical clone that renders `loosely_similar`, hides half its occurrences, and ranks #900 passes today. The Type-2 check already demands span **plus** bucket, saturating evidence and visibility — the byte-identical case is the easier proof and holds the weaker contract.

### L6. Determinism is asserted on 2 of 9 repositories

`corpus_determinism_nest_typescript` and `corpus_determinism_jellyfin_csharp`. Seven languages have no determinism assertion at all, and [PIPELINE-DETERMINISM] is what makes every other number in the report quotable.

### L7. A check id proves a check ran, not that it judged

`GATE_CHECKS` lists eight ids. A repository with empty curated lists still "runs" `recall`, `type2_recall` and `precision` — they iterate nothing and pass. The baseline ratchet in `known-failures.json` then reads that pass as evidence the defect is absent.

### L8. A green scheduled run is not full coverage

[CORPUS-CI] sizes the scheduled slice to finish in about a minute. The summary is supposed to name what was skipped. Nothing asserts it does, so a three-repo run reads as a nine-repo pass.

## Part 2 — Assert a fuckload

The target state: **every repository, every run, asserts every row below.** Ids are rank-independent per [CORPUS-BASELINE].

| check id | asserts | input | today |
|---|---|---|---|
| `files_analysed` | the scan parsed a plausible number of files, never zero | `expect_files_min` | ❌ missing (L2) |
| `recall` | curated byte-identical clones are reported | `must_find` | 🟡 span only (L5) |
| `recall_quality` | …in an act-now bucket, every curated occurrence **shown**, within `max_rank` | `must_find` | ❌ missing |
| `type2_recall` | curated renames reported, gate-vouched, shown | `must_find_type2` | ✅ |
| `precision` | curated non-duplicates never share a cluster | `must_not_cluster` | ❌ missing (L4) |
| `boilerplate_rank` | framework-mandated shapes never rank first | `must_not_rank_first` | 🛑 unsound (L3) |
| `data_table_rank` | digit-dominated clusters carry `category: data` | none | ✅ |
| `fused_bounded_max` | rendered confidence never exceeds the strongest axis | none | ✅ |
| `type2_gate_liveness` | the content gate produced *some* vouched evidence | none | ✅ |
| `determinism` | two runs on an unchanged tree agree exactly | none | 🟡 2 of 9 (L6) |
| `metrics_stable` | `duplication_percent` and cluster count reproduce exactly | none | 🟡 inside determinism |
| `cluster_count_band` | cluster count sits inside a curated band | `expect_clusters` | ❌ missing |
| `wall` / `memory` | resource ceilings | `ceilings` | ✅ |

### A. Curated precision — `[CORPUS-PRECISION-CURATED]` — LANDED

- [x] `must_not_cluster` — pairs a human confirmed are **not** duplicates, each carrying `why`, `verified` and `files`. Fails when any single **shown** cluster spans every listed path.
- [x] `check_curated_precision` in `corpus_precision.rs`, wired into `gate()` and `GATE_CHECKS` as `precision`, documented in `known-failures.json` `_checks`.
- [x] Visibility mirrors [CORPUS-RECALL] through one shared predicate — `corpus::cluster_shows_span`, read forwards by recall and backwards by precision. A hidden occurrence clears neither.
- [x] An entry naming fewer than two files fails rather than passing vacuously.
- [x] Four unit tests in `corpus_precision/tests.rs::curated_precision`: unclustered passes, shown-spanning fails, hidden does not breach, under-two-files fails.
- [ ] **Curated entries themselves** — no manifest carries `must_not_cluster` yet. That is item F.

### B. Strengthen recall to the Type-2 bar — `[CORPUS-RECALL]` — LANDED

- [x] `check_curated_recall` replaces the span-only `check_recall` and splits the verdict in two: `recall` (some cluster spans the paths) and `recall_quality` (it is labelled `identical`, every curated occurrence is shown, and it is within `max_rank`).
- [x] `identical` is the *only* admissible bucket, not merely an act-now one — [CORPUS-RECALL] defines `must_find` as byte-for-byte verified, so anything else is the engine contradicting a verified fact about the source. No prose is parsed to decide it.
- [x] Curated occurrences must be shown, through the same `cluster_shows_span` predicate as precision and `type2_recall`.
- [x] Optional `max_rank` per entry, inclusive.
- [x] Six unit tests in `corpus_confidence/tests/recall.rs`, including all four demotion buckets and the half-hidden pair — each of them a report the old span-only check passed.
- [ ] **`max_rank` values** — no entry curates one yet. That is item F.

### C. 🛑 #401 — replace the text-matching ranking rule — LANDED

- [x] Failing tests pinning both directions, watched red against the shipped `text.contains` arm: a comment / doc comment / string-literal mention must not fire, a clause wrapped across three lines must. Four of five went red for the right reason; the fifth (the flat spelling) stayed green, which is what proves the instrument was not simply blind.
- [x] The text-matching arm is deleted, not worked around. `crates/deslop-test-support/src/corpus_precision.rs` records what it did and which tests pin the replacement.
- [x] Replaced with a tree-sitter predicate: the declaration overlapping the ranked occurrence, its heritage clause, and a type-name leaf inside it. Both the declaration containing the occurrence and any declaration it contains count, because the ranked occurrence is usually the mandated *member* — Flutter's `createState` — not the class header.
- [x] `forbidden_top_shapes` is now `forbidden_top_supertypes`, a list of base-type names; `[CORPUS-PRECISION]` specifies the new form.
- [x] The heritage grammar is curated per language from each grammar's own `node-types.json` and asserted per language — dart, csharp, typescript, tsx, javascript, python, php. A language with no curated grammar **fails the gate** rather than passing it.
- [x] Type arguments are not base types: `extends State<LedgerView>` names `State`. Dart is the one grammar that flattens type arguments into sibling `type:` fields rather than nesting them, and `BaseTypes::FirstChildOnly` says so explicitly.
- [ ] Re-verify #331 against the replacement, and reopen it if the evidence does not survive. **Blocked on a flutter scan** — in progress.

### D. Assert the scan happened at all — LANDED, PENDING CURATION

- [x] `corpus_scope.rs` — `files_analysed` and `cluster_count_band`, wired first in `gate()` because every check after it iterates a set an empty report leaves empty.
- [x] An absent bound **fails**, so a manifest cannot switch the check off by omission, and `corpus_manifest_contract.rs::every_manifest_curates_a_non_vacuous_scan_scope` refuses it before any scan runs.
- [x] A missing `files_analysed` field fails rather than defaulting to zero — a defaulted zero and a measured zero are different defects and both are fatal.
- [x] Five unit tests in `corpus_scope/tests.rs`, both directions on both bounds plus the two uncurated shapes.
- [ ] **The curated numbers** — measuring all nine repositories against the pinned shas. django is measured (2,835 files / 9,268 clusters / 27.8%); the rest are in flight.

### E. Determinism everywhere

- [ ] Extend the determinism assertion from 2 repositories to all 9 — same tree, two runs, exact agreement on cluster count and `duplication_percent`.
- [ ] Assert the *values*, not just their equality, so a determinism pass cannot be bought by both runs being equally broken.

### F. Curate every repository, every language

Hand-verified ground truth at the bar the nest/tokio entries set — a real diff, quoted in `verified`, against the pinned `sha`. Per repo: **≥2 `must_find`, ≥1 `must_find_type2`, ≥2 `must_not_cluster`.**

- [ ] **django** (python) — #103 is a Python FP; its pin belongs here
- [ ] **react** (javascript) — #79 and #283 land here
- [ ] **jellyfin** (csharp)
- [ ] **laravel** (php)
- [ ] **hugo** (go)
- [ ] **fsharp** — #336 and #339 land here; curate **after** C and after #339, since both change what F# measures
- [ ] **flutter** — has recall and a ranking rule; add renames and precision
- [ ] **nest**, **tokio** — have renames only; add recall and precision
- [ ] #71, #284, #285 — place each against whichever repository reproduces it, and say so in the issue

### G. Make emptiness red, not chatty

`warn_when_accuracy_unasserted` prints `ACCURACY UNASSERTED` and passes. That warning has been printing into green runs the whole time.

- [ ] Extend `corpus_manifest_contract.rs`: every manifest must carry curated ground truth in **all three** categories, and every shipped language must have at least one curated repository.
- [ ] Land it as a ratchet — assert the counts that exist today and raise them as F lands, so the gate can never regress to vacuous.
- [ ] Assert the scheduled job summary names the skipped repositories (L8), so a slice is never misread as coverage.
- [ ] A check id whose curated input is empty must report **unasserted**, distinct from **passed** (L7) — the baseline ratchet must never read the first as the second.

## Part 3 — Smooth

- [ ] One command runs one repository end to end with legible failure text; a failure names the check, the curated entry, and the reported cluster it disagreed with.
- [ ] Clone/scan caching so re-running a single repository during curation is cheap — curation is iterative by nature and currently pays a full clone.
- [ ] Slice scheduling that rotates, so all nine repositories are covered across a week and the summary says which day covered what.
- [ ] `make test-corpus` stays strict locally (ignores `known-failures.json`) — it is the honest run and must remain the default for humans.

## Close-outs

- [ ] **#347** corpus gate never boots — three consecutive green corpus runs. Close naming the runs, not the colour.
- [ ] ⚠️ **#331 is already closed on synthetic evidence.** Its real-repository confirmation is the check L3 shows is unsound. Re-verify after C; reopen if it does not survive.

## Order

```
C #401 ──► D (scan happened) ──► A (precision surface) ──► B, E ──► F (curate all) ──► G (ratchet)
```

C and D first: C decides whether the precision evidence already on the board can be trusted, and D is the cheapest assertion on the list guarding the most severe failure. F is last — curation is only worth doing once the surfaces it writes into are sound, and G locks the door behind it.
