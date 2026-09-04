# Real-repository corpus gate

Fixture repos prove the pipeline runs. They cannot prove it is *right*: a fixture is written by the same person who wrote the detector, and it never has 6,000 files. The corpus suite scans real public codebases and asserts what the fixtures cannot — that hand-verified duplicates are actually reported, that scaffolding does not outrank copy-paste, and that a scan of a real repository fits in a CI runner.

### [CORPUS-PIN] Pinned repositories

Each repository is one `corpus/<name>.json` manifest naming a `url`, a `tag`, **and** the commit `sha` that tag pointed at when the duplicates were curated. `scripts/corpus/fetch-corpus.mjs` clones through the tag (shallow, cheap) and then verifies `HEAD` against the pinned `sha`.

A moved or re-cut upstream tag is a **hard error that deletes the clone**, never a silent re-baseline. Curated line ranges and `must_find` paths are only meaningful against the commit they were verified on; accepting a drifted tag would silently convert a recall assertion into a coin flip.

Clones live in git-ignored `.corpus/`. A missing clone is a hard error naming `make test-corpus`, never a skipped test.

### [CORPUS-CEILINGS] Resource budget

Every manifest carries `ceilings.max_wall_seconds` and `ceilings.max_peak_rss_mb`, measured by running the release binary under the platform's peak-RSS measurement.

That measurement must report a **true** peak, never a sampled one. A sample taken on an interval is a lower bound, and a lower bound on a ceiling assertion produces false passes — sampling `WorkingSet64` during a flutter scan read 3,629 MB where the OS's own peak counter read 4,818 MB. Both supported platforms therefore read a counter the kernel maintains: POSIX wraps the scan in `/usr/bin/time` (`-v` GNU, `-l` BSD), and Windows, which has no such tool, spawns the scan directly and watches `PeakWorkingSet64` for its pid via `scripts/corpus/peak-working-set.ps1`. Both emit the same `Maximum resident set size (kbytes):` line, so one parser reads either.

A measurement that never arrived is an error, never a zero. A zero would parse as a real number and clear every memory ceiling in the corpus at once.

The memory ceiling is **sized per repository, in `corpus/*.json`** — a function of the corpus's own scale, never of the machine that happens to host the test, and never a shared standard number: every repo is different. A ceiling copied from a CI runner says nothing about Deslop: it is either so loose it never fires or so tight it fails for reasons that are not the product's. Each manifest's own `rationale` records why its number is what it is. The wall ceiling is deliberately loose: it exists to catch a hang, not to police throughput.

### [CORPUS-SCOPE] The scan happened

Every other check here reads the clusters a report contains, and none of them can see the report that contains *nothing*: a scan that analysed zero files renders cleanly, exits 0, and satisfies recall, precision, confidence and ceilings at once, because each iterates a set that is empty. gh #342 shipped exactly that — a repository under any folder named `dist`, `build` or `target` analysed as zero files.

Every manifest therefore carries two curated bounds, and both are **required**:

- `expect_files_min` — the floor `files_analysed` must clear. Under it, discovery lost part of the repository.
- `expect_clusters` — a `{ min, max }` band the rendered cluster count must sit inside, inclusive at both ends. Below it, detection stopped finding duplicates; above it, something started manufacturing them. Both are repository-wide swings that no per-cluster check can see, because each of those judges only the clusters that *are* there.

**Neither bound is the measured number, and neither is a golden.** Nobody knows how many clusters Flutter *should* have. A measurement is only what this detector reports today, and today's detector has known defects — `[PIPELINE-CLUSTER-CLOSURE]` moved tokio's count from 2,155 to 2,568 in one commit. Pinning a measurement would convert whatever is currently wrong into the contract, and the corpus gate would then defend the bug.

So both bounds are deliberately loose rails, each sized to the failure it exists to catch. `expect_files_min` sits at roughly three quarters of the measured `files_analysed`, rounded down: enough to catch discovery losing a chunk of the repository — an exclusion pattern, an extension map, the whole tree (gh #342) — while tolerating the handful of files a legitimate new generated-file rule removes. `expect_clusters` runs from half the measured count to double it, because neither a collapse nor an explosion fits inside a factor of two and ordinary tuning does. What that band asserts is what is actually knowable: the number is not zero, not half, and not double. Re-curate when a deliberate change moves a repository outside its rails, and say in the commit which change moved it.

An absent bound is not a repository with no opinion about its own size — it is a check that cannot fire, so a manifest that omits one fails the gate rather than passing it, and `corpus_manifest_contract.rs` refuses it before any scan runs.

### [CORPUS-RECALL] Curated duplicates

`must_find` lists duplicates a human confirmed byte-for-byte, each with the `diff` that proved it. Two check ids judge it:

- `recall` — some cluster spans every path in the entry. Anything less is a false negative on code known to be duplicated.
- `recall_quality` — every curated byte-identical endpoint relation is classified `identical`, every curated occurrence is shown in the same component, and that component is within the entry's optional `max_rank` ceiling.

`recall` alone used to be the whole assertion. A 137-line byte-identical pair that compared as `loosely_similar`, hid one of its two occurrences and landed in a component ranked #900 satisfied it completely. The byte-identical case is the easier proof and must not hold the weaker contract. `identical` is the only classification a byte-identical pair may reach; that classification is asserted on the exact curated endpoints, never on their closure component. `max_rank` is optional per entry, because only the entries a human ranked get a rank assertion — ranking is the product, and a finding a user never scrolls to is a finding they do not get.

**An empty `must_find` asserts nothing, and the suite says so out loud** — such a run prints `ACCURACY UNASSERTED` and has proven only that the scan fit its budget. A green corpus test is not evidence of accuracy unless the repository has curated entries. An entry that lists no files fails rather than passing vacuously.

`must_find_type2` is the same contract for renames: each human-verified pair carries `verified`, `why`, `files`, and `min_nodes`. The entry passes only when that exact pair is admitted with the required structural and content evidence, both curated occurrences are visible in the same closure component, and the reported extent reaches the node floor. Assertions read the pair record for evidence and the cluster record for visibility, extent, and mass; they never read pair scores from a cluster.

`must_find_type2_status` states the curation position in words and must not contradict the list. Because an empty list silently asserts nothing, the manifests are themselves under test: curated Type-2 ground truth must exist in at least two languages, and every entry must name at least two distinct files with its human evidence and its `min_nodes` extent attached.

### [CORPUS-PRECISION] Ranking rules

Ranking *is* the product, so the head of the report is where a false positive does the most damage. Two rules apply to the top-ranked clusters:

- `must_not_rank_first` names framework-mandated shapes for that language. Such code cannot be extracted or merged, so it must never outrank genuine copy-paste. Compare [CLONE-NOISE-*](noise.md).

  `forbidden_top_supertypes` is a list of **base-type names**, matched as an AST predicate: a ranked cluster fails when the type declaration its first occurrence overlaps names one of them in the language's heritage clause. The language is that of the first occurrence's file, resolved the way the engine maps files to parsers; a cluster carries no language of its own, and an occurrence in a file no parser claims fails the gate rather than passing it. Both the declaration containing the occurrence and any declaration the occurrence contains count, because the ranked occurrence is usually the mandated *member* — Flutter's `createState` — not the class header that makes it mandated.

  It is never matched against source text. The rule shipped as `text.contains("extends StatefulWidget")` and was wrong in both directions (gh #401): it fired on a comment or string literal that merely mentioned the supertype, and it missed a declaration whose clause was wrapped across lines. Type arguments are not base types — `extends State<LedgerView>` names `State` — and a language with no curated heritage grammar fails the gate rather than passing it, so a rule that cannot fire is never mistaken for a rule that found nothing.
- Language-agnostic: a finding that is overwhelmingly digits and separators is recognized as a data table for detection-time visibility, but any visible closure component keeps its full mass and carries no data category.

### [CORPUS-PRECISION-CURATED] Curated non-duplicates

`must_not_cluster` lists pairs a human confirmed are **not** duplication, each with `why`, `verified` (how it was checked) and `files`. The `precision` check fails when a single shown cluster spans every path in an entry.

This is [CORPUS-RECALL]'s predicate read backwards — the same "does a shown cluster span every curated path" question, opposite verdict — and visibility works the same way for the same reason: a false positive nobody is shown is not a false positive, so a cluster whose curated side is entirely hidden does not breach the entry. Suppressing it is the fix, not a loophole.

Without this field no manifest could express the thing seven open false-positive issues all say — *these are not duplicates and Deslop clustered them* — so none of them could be pinned on the repository it was reported against. An entry naming fewer than two files fails rather than passing vacuously: one path cannot describe a pair the engine wrongly joined.

### [CORPUS-REGISTER] The clone register — independent ground truth

`must_find` and `must_not_cluster` are curated by whoever is working on Deslop, which is the problem: read the engine's code and you start writing down the engine's opinion of itself. A register is the same idea with the contamination removed.

Registers live in `corpus/register/<name>.json`, pinned to the commit they were judged at. Each entry names two or more ranges as `path:startLine-endLine`, with `why` (the judgement in plain English) and `verified` (the diff that was actually run, and what it returned).

**Three verdicts.** CLEARLY IN is an obvious clone — failing to report it is a false negative. CLEARLY OUT is a pairing that would be plainly wrong — reporting it is a false positive. NOT CLEAR is everything carrying a shred of doubt; it is recorded so nobody re-litigates it and it asserts nothing. Most pairs are NOT CLEAR, and that is the healthy outcome: the register is worth something because it is small and unarguable.

**The judge never sees this codebase.** Judging happens in a folder built outside this repository by `make judging-folder`, holding one directory per repository — the target source, two reports labelled A and B by a sealed coin flip, and candidate pairs rendered as source only — no rank, no mass, no band, no node count, no provenance. Loading any Deslop source or spec into the judge's context is contamination: the pass aborts and every verdict from it is discarded. The full protocol is `.agents/skills/clone-register-prepare` (preparer) and `.agents/skills/judge-clone-pairs` (judge).

**One predicate, both directions.** An entry is *matched* when some published cluster shows occurrences overlapping every listed range. A matched CLEARLY IN is correct and an unmatched one is a false negative; a matched CLEARLY OUT is a false positive and an unmatched one is correct. Overlap rather than exact line equality is what makes the assertion non-fragile — it survives extent drift, rank movement, band movement and occurrence-count changes, and breaks only when the engine stops pairing two regions it must pair, or starts pairing two it must not.

**It is the only evidence of an accuracy change.** `scripts/compare-versions.sh` scores both engines against the register on every run and states the verdict. A version degraded against another only if it introduces a false negative or a false positive against the judged pairs; cluster totals, duplication percentages and rank movement are description, never verdict. A CLEARLY IN neither version finds, or a CLEARLY OUT both report, is a standing defect — real, and not slippage.

An empty `clearly_out` list must carry `clearly_out_status` prose saying so, for the reason `must_find_status` exists: emptiness is not evidence that precision is good.

The protocol lives in two skills, deliberately split so neither role can drift into the other: `.agents/skills/clone-register-prepare` builds the workspace and files what comes back, and `.agents/skills/judge-clone-pairs` is the judging protocol, which never names this project and is installed at the root of the handed-over folder as a skill the judge can run by name, linked from each repository directory as `JUDGING.md`, so a judge never reaches back here to read it. The A/B key and the pinned checkouts are written beside that folder and never inside it.

### [CORPUS-REGISTER-COVERAGE] How far the register is meant to go

The register is built up over time, never in one pass. The shape it is growing towards:

- **Two to three repositories per language, of visibly different size.** A single repository teaches the register that language's house style, not the language. A small library and a large application disagree about what repetition is normal, and the register needs both.
- **Roughly 100 CLEARLY IN and 100 CLEARLY OUT entries per repository.** Reaching that takes several judging passes at different seeds; a pass that yields a dozen firm verdicts out of two hundred candidates is doing well, not badly.
- **About 200 candidates per repository per pass, as a ceiling.** Past that a repository stops paying for the time. `scripts/corpus/register-workspace.mjs` caps the draw there.

**Breadth beats depth.** When the choice is another pass on a repository already judged versus a first pass on a language with no register at all, the new language wins. A false positive that only appears in Kotlin is invisible until some Kotlin is judged, however deep the Python register goes.

Progress is measured against that shape, not against a percentage. A language with no register is a hole; a language with one small repository is a partial answer. `corpus/register/README.md` carries the current standing so the next pass can be chosen without re-deriving it.

Two rules hold at every size. **The register never grows by relaxing certainty** — a pass that produced four entries produced four entries, and topping it up with arguable ones destroys the only property that makes it worth having. And **an entry is never deleted or softened to make a run pass**; a contradicted CLEARLY IN means either the engine is wrong or the entry was never certain, and settling that needs a fresh workspace and a fresh reading of the source.

### [CORPUS-REGISTER-WORKSPACE] The folder a judge is handed

`make judging-folder` produces one folder, and that folder is the whole of what a judge gets. It holds three things and nothing else:

- **The repositories.** Each checked out at its pinned commit, source only, no git history.
- **The reports.** Two pair lists per repository, cut down to which regions were grouped together. Rank, band, mass, node and occurrence counts, thresholds, timings, versions and cache statistics are all the engine's opinion of itself, and every one of them is stripped. The two lists are labelled A and B by a coin flip, and neither letter means "newer".
- **The judging protocol**, installed at the root as `.agents/skills/judge-clone-pairs/SKILL.md` with `.claude/skills/judge-clone-pairs` symlinked to it — the same layout this repository uses for its own skills, so an agent opening the folder can run it by name and the two paths can never drift apart. Each repository directory reaches the same file as `JUDGING.md`. The folder's `AGENTS.md` (imported by `CLAUDE.md`) and each repository's `README.md` come across with it.

Four rules make the folder worth having, and each is enforced rather than trusted:

0. **Every word the judge reads is a copy.** The protocol and both guides are files in `.agents/skills/judge-clone-pairs/`, copied across byte for byte — the protocol from `SKILL.md`, the guides from `handover/`. Nothing is composed while the folder is built: prose written at run time is prose nobody reviewed, and two passes a month apart would hand two different folders to two judges.
1. **It never names this project.** The builder scans everything it wrote and refuses to finish if the name appears. A judge who learns what produced the reports is judging the tool, not the code.
2. **The key stays outside it.** Which engine got which letter is written to a sibling directory, never inside. Inside, it is the answer sheet.
3. **It lives outside this checkout.** A judge who can walk up into this source is contaminated, and every verdict from that pass is void.

Candidates are drawn up to a ceiling of 200 per repository, stratified across where a pair came from and how large it is, then shuffled — so one pass cannot end up all one engine's findings or all one size of block. Where a pair came from drives the sampling and is then discarded; the judge sees a number.

Built by `scripts/corpus/prepare-judging.sh` and `scripts/corpus/register-workspace.mjs`, asserted by `scripts/repository/judging-workspace.test.mjs`, and described for the two roles in `.agents/skills/clone-register-prepare` and `.agents/skills/judge-clone-pairs`.

### [CORPUS-REGISTER-QUEUE] How a new repository gets in

A repository cannot become a register until a judge has ruled on its pairs, and a judge cannot rule on pairs until a comparison has produced two reports for it. Left there, the corpus could never grow: the comparison would only ever scan repositories that already have a register.

`corpus/judging-queue.json` breaks that circle. It lists repositories waiting on a first pass — url, commit id, language, and why this one — and the comparison scans everything in it alongside the registers. Each queued repository then gets a workspace from `make judging-folder` exactly like a judged one.

A queued repository has **no register yet**, and a repository with a register is **not** in the queue; `crates/deslop/tests/corpus_commit_pins.rs` asserts both, so the queue drains rather than growing a second copy of the corpus. Entries are pinned to a full commit id for the same reason a register is: the judge and the scan have to read one identical tree.

The CI accuracy gate does **not** scan the queue. It runs a small slice of judged registers, because a repository with no verdicts can answer no questions and would only cost time on every push.

### [CORPUS-REGISTER-MERGE] Getting verdicts back in

Several judges are sent the same folder and rule independently. Their answers become ground truth only where they **agree**: two judges who file one pair under two verdicts have between them said something false, and taking either answer would write that falsehood into the register and score every engine against it from then on.

`scripts/corpus/merge-verdicts.mjs` is the only way a verdict reaches a register.

**A pair is imported only when every source agrees on it** — this repository's existing registers and all the judged folders together. Three sources agreeing with a verdict the register already holds leave it exactly as it is; two judged folders agreeing on a pair the register does not hold add it. Anything else is left out and written to the report. There is no majority, no tie-break and no preferred judge: a pair two readers see differently is exactly the pair a register must assert nothing about.

What counts as not agreeing, each named after the comparison that found it:

- `clearly_in/clearly_out` — one judge's CLEARLY IN against another's CLEARLY OUT. A claim about the source that cannot be half true.
- `clearly_in/not_clear`, `clearly_out/not_clear` — one judge committed where another would not.
- `occurrences_mismatch` — the ranges a judge filed against a candidate number are not the ranges `candidates/pairs.json` associates with it. A set comparison between two files that must agree; the report prints both range lists and infers nothing about why they differ.
- `register_conflict` — an earlier pass and this one read the same lines differently.
- `duplicate_pair` — the draw showed one pair of regions under two candidate numbers, and they were answered differently.

The rest of the rules:

- **At least two judges must have ruled.** One reader with a firm opinion is an opinion; two arriving at it separately is evidence.
- **Prose.** A scored entry states its judgement and its diff at length. NOT CLEAR is held to a note instead: it asserts nothing, so there is no assertion to state, and refusing a terse note would throw away the record that stops the next pass re-reading a pair somebody has already ruled on.
- **One tree.** Every judged folder and the register must name the same commit. A line number means nothing without the tree it was read in.
- **Judges must have been shown the same candidates.** Candidate numbers are the only handle a verdict has on a pair, so two workspaces with different pair lists would have their verdicts cross-matched into disagreements that never happened. The run refuses outright.
- A repository that gains a register leaves `corpus/judging-queue.json` in the same run, per [CORPUS-REGISTER-QUEUE].

`docs/reports/verdict-merge.md` holds what was left out. Every string in it is a column header, a label derived from the verdicts themselves, or a value read out of a judging folder — the only sentences are the judges' own, quoted. Rows are sorted by kind, repository and candidate, so two runs over the same verdicts produce the same document.

Asserted by `scripts/repository/verdict-merge.test.mjs`, which drives the script against throwaway judging folders that agree and that do not.

### [CORPUS-SCORE] The accuracy score

The register says which pairs are real. The score says how the engine is doing against them, in one number per repository and one for the corpus.

**The number.** `score = 100 × correct / judged`, where `judged` is every CLEARLY IN and CLEARLY OUT entry and `correct` is the ones the engine answered right. A CLEARLY IN it reports is correct; one it misses is a **false negative**. A CLEARLY OUT it stays silent on is correct; one it reports is a **false positive**. Both wrong answers are bugs, and each costs exactly one judged pair — no weighting, no partial credit.

A register with no entries scores **nothing**, never 100%. Being asked no questions is not the same as answering them all correctly, which is why the denominator is printed beside every figure.

**The corpus score is `correct / judged` over every judged pair**, not the mean of the per-repository scores. Averaging percentages would let a repository with two judged pairs outvote one with two hundred.

**Cost is measured and reported, never scored.** Every scan runs under the same peak-RSS measurement the ceiling suite uses, recording wall time, peak resident set and CPU seconds. They sit beside the score so two runs can be compared honestly; a slower engine that finds the same pairs has not become less accurate. Cluster and pair totals are description in exactly the same way.

**The gate.** `corpus/register/score-thresholds.json` holds a maximum false-positive and false-negative count per repository. Those counts are the whole gate, and there is deliberately no minimum-score percentage: a score is `100 * (judged - false negatives - false positives) / judged`, so a percentage threshold is really a defect allowance divided by the size of the register. Leave one in place and it widens on its own every time the register grows - the number on the page never changes while what it permits multiplies. Counts mean the same thing whether a register judges six pairs or six hundred, and "no false positives and no false negatives" already says "a perfect score" exactly. The defaults are strict — a judged repository must answer every judged pair correctly — and every entry under `repos` is an admission that a defect shipped, carrying the reason, exactly like [CORPUS-BASELINE]. Numbers move one way: tightened when the engine improves, never loosened to make a run pass. Deleting an entry by fixing the defect is the only correct exit.

**The report is mechanical, and it lives in the repository.** `SCORE.md` and `score.json` are written by the run that took the measurements, not by whoever is reading them. No figure in either document is transcribed, summarised or re-typed by hand or by an agent, and **neither is ever hosted, uploaded or published anywhere** — a report you cannot re-run, diff and reproduce from the tool is not evidence, it is somebody's recollection. Point at the path.

**Comparisons read side by side.** Every table in `SCORE.md` gives each engine its own column: the corpus standing is one measure per row, and each per-repository table is one repository per row. Two figures a reader is meant to compare never sit a row apart, because a comparison split across rows is one the reader has to reassemble by eye. The per-repository table also says whether each defect is **new** against the first engine or **standing** in both, so a regression is never confused with a bug that was already there.

**One implementation.** The scoring, the gate and the scorecard markdown are all `deslop-test-support`'s `corpus_score` module, run through the `corpus-score` binary. Nothing outside Rust computes a count, a percentage or a delta; the shell measures and orchestrates, and `scripts/compare-versions-summary.mjs` reads the scorecard rather than deriving anything from it, so the two documents cannot disagree. The scorer is deliberately built from the working tree and never from a compared engine's source: two engines in a comparison must be scored by one identical scorer.

**Two entry points, one code path.** `scripts/corpus/score-gate.sh` scans the register-backed targets with the current build and gates on the thresholds — this is what CI runs, on a small slice. `scripts/compare-versions.sh` scans the same targets with two engines and additionally reports whether the second lost ground against the first. Both write the same run manifest and call the same scorer.

**Degradation has a hard definition.** Version B degraded against version A only if B misses a CLEARLY IN that A found, or reports a CLEARLY OUT that A stayed silent on. A defect both engines share is a **standing defect** — real, and reported as such, but not slippage.

### [CORPUS-BASELINE] The known-failures ratchet

`corpus/known-failures.json` records checks that already fail, each mapped to a tracked issue.

- **Locally, `make test-corpus` ignores the file entirely** and fails on everything. Local runs stay honest.
- **`make test-corpus-ci` reads it** and fails only on checks *not* recorded — that is the regression gate, and it is what the scheduled workflow runs.
- Adding an entry is admitting a defect shipped. Fixing it and deleting the entry is the only correct exit. Widening a ceiling or deleting an assertion to clear an entry is prohibited.
- Check ids are rank-independent by construction: ranks move between runs while [PIPELINE-DETERMINISM] is unmet, so a rank-bearing key would churn the baseline into uselessness.

### [CORPUS-CI] Scheduled, never blocking

The corpus workflow is scheduled and dispatchable, and has **no `pull_request` trigger**. It reports; it does not gate. Merges must not depend on a suite that is red on tracked defects, and a suite that is red on tracked defects must not be quietly weakened to make it green.

Scheduled runs scan a slice sized to finish in about a minute, not the whole corpus. The job summary states which repositories were skipped, so a green scheduled run is never misread as full coverage. The slice is selected by `--exact`, never by substring: a filter that matches part of a test name selects nothing the day a test is renamed, and an empty run reports green (gh #412).

Every test in the suite is `#[ignore]`d under [SKIP-TOO-LARGE-FOR-CI] and cites **gh #422**, which records the measurement — `flutter/flutter` at roughly 9.5 GB peak and 9m44s (#166), `dotnet/fsharp` above 13 GB — and the memory work that would let these tests move into the PR gate. `--ignored` is what selects them; nothing is selected by name. See [release.md §TEST-SELECTION-SKIP](release.md).

`make test` and `make lint` still **compile and lint** the target. Skipping costs coverage of the suite's execution, never of its compilation: the earlier `required-features` gate removed it from `--all-targets` entirely, and commit `77bcbaed5` left it uncompilable, with nothing to notice until someone ran `make test-corpus`.
