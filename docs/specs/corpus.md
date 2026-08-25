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

**Neither bound is the measured number, and neither is a golden.** Nobody knows how many clusters Flutter *should* have. A measurement is only what this detector reports today, and today's detector has known defects — `[PIPELINE-CLUSTER-ELECT]` moved tokio's count from 2,155 to 2,568 in one commit. Pinning a measurement would convert whatever is currently wrong into the contract, and the corpus gate would then defend the bug.

So both bounds are deliberately loose rails, each sized to the failure it exists to catch. `expect_files_min` sits at roughly three quarters of the measured `files_analysed`, rounded down: enough to catch discovery losing a chunk of the repository — an exclusion pattern, an extension map, the whole tree (gh #342) — while tolerating the handful of files a legitimate new generated-file rule removes. `expect_clusters` runs from half the measured count to double it, because neither a collapse nor an explosion fits inside a factor of two and ordinary tuning does. What that band asserts is what is actually knowable: the number is not zero, not half, and not double. Re-curate when a deliberate change moves a repository outside its rails, and say in the commit which change moved it.

An absent bound is not a repository with no opinion about its own size — it is a check that cannot fire, so a manifest that omits one fails the gate rather than passing it, and `corpus_manifest_contract.rs` refuses it before any scan runs.

### [CORPUS-RECALL] Curated duplicates

`must_find` lists duplicates a human confirmed byte-for-byte, each with the `diff` that proved it. Two check ids judge it:

- `recall` — some cluster spans every path in the entry. Anything less is a false negative on code known to be duplicated.
- `recall_quality` — that duplication is reported **as what it is**: a cluster labelled `identical`, with every curated occurrence *shown*, and within the entry's optional `max_rank` ceiling.

`recall` alone used to be the whole assertion. A 137-line byte-identical clone that rendered `loosely_similar`, hid one of its two occurrences and ranked #900 satisfied it completely — while `must_find_type2` next door already demanded span *plus* bucket *plus* visibility for the strictly harder case. The byte-identical case is the easier proof and must not hold the weaker contract. `identical` is the only bucket a byte-identical pair may reach: anything else is the engine contradicting a verified fact about the source. `max_rank` is optional per entry, because only the entries a human ranked get a rank assertion — ranking is the product, and a finding a user never scrolls to is a finding they do not get.

**An empty `must_find` asserts nothing, and the suite says so out loud** — such a run prints `ACCURACY UNASSERTED` and has proven only that the scan fit its budget. A green corpus test is not evidence of accuracy unless the repository has curated entries. An entry that lists no files fails rather than passing vacuously.

`must_find_type2` is the same contract for **renames**: pairs a human confirmed are the same code with different identifiers, each carrying `verified` — how the rename was checked — alongside `why` and `files`. Byte-identical clones belong in `must_find`; a Type-2 pair is not byte-identical by definition, so it can never reach the `identical` bucket and must be earned from the content gate. The entry passes only when a cluster spans its files, that cluster is `nearly_identical`, its shown signals carry saturating shape evidence, and the curated occurrences are themselves visible. A hidden occurrence fails: recall is what the report *shows*, not what it contains.

`must_find_type2_status` states the curation position in words and must not contradict the list. Because an empty list silently asserts nothing, the manifests are themselves under test: curated Type-2 ground truth must exist in at least two languages, and every entry must name at least two distinct files with its human evidence attached.

### [CORPUS-PRECISION] Ranking rules

Ranking *is* the product, so the head of the report is where a false positive does the most damage. Two rules apply to the top-ranked clusters:

- `must_not_rank_first` names framework-mandated shapes for that language. Such code cannot be extracted or merged, so it must never outrank genuine copy-paste. Compare [CLONE-NOISE-*](noise.md).

  `forbidden_top_supertypes` is a list of **base-type names**, matched as an AST predicate: a ranked cluster fails when the type declaration its first occurrence overlaps names one of them in the language's heritage clause. Both the declaration containing the occurrence and any declaration the occurrence contains count, because the ranked occurrence is usually the mandated *member* — Flutter's `createState` — not the class header that makes it mandated.

  It is never matched against source text. The rule shipped as `text.contains("extends StatefulWidget")` and was wrong in both directions (gh #401): it fired on a comment or string literal that merely mentioned the supertype, and it missed a declaration whose clause was wrapped across lines. Type arguments are not base types — `extends State<LedgerView>` names `State` — and a language with no curated heritage grammar fails the gate rather than passing it, so a rule that cannot fire is never mistaken for a rule that found nothing.
- Language-agnostic: a top-ranked cluster that is overwhelmingly digits and separators is a data table, and must carry `category: data` rather than ranking at full logic weight.

### [CORPUS-PRECISION-CURATED] Curated non-duplicates

`must_not_cluster` lists pairs a human confirmed are **not** duplication, each with `why`, `verified` (how it was checked) and `files`. The `precision` check fails when a single shown cluster spans every path in an entry.

This is [CORPUS-RECALL]'s predicate read backwards — the same "does a shown cluster span every curated path" question, opposite verdict — and visibility works the same way for the same reason: a false positive nobody is shown is not a false positive, so a cluster whose curated side is entirely hidden does not breach the entry. Suppressing it is the fix, not a loophole.

Without this field no manifest could express the thing seven open false-positive issues all say — *these are not duplicates and Deslop clustered them* — so none of them could be pinned on the repository it was reported against. An entry naming fewer than two files fails rather than passing vacuously: one path cannot describe a pair the engine wrongly joined.

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
