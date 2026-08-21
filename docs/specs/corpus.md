# Real-repository corpus gate

Fixture repos prove the pipeline runs. They cannot prove it is *right*: a fixture is written by the same person who wrote the detector, and it never has 6,000 files. The corpus suite scans real public codebases and asserts what the fixtures cannot — that hand-verified duplicates are actually reported, that scaffolding does not outrank copy-paste, and that a scan of a real repository fits in a CI runner.

### [CORPUS-PIN] Pinned repositories

Each repository is one `corpus/<name>.json` manifest naming a `url`, a `tag`, **and** the commit `sha` that tag pointed at when the duplicates were curated. `scripts/corpus/fetch-corpus.mjs` clones through the tag (shallow, cheap) and then verifies `HEAD` against the pinned `sha`.

A moved or re-cut upstream tag is a **hard error that deletes the clone**, never a silent re-baseline. Curated line ranges and `must_find` paths are only meaningful against the commit they were verified on; accepting a drifted tag would silently convert a recall assertion into a coin flip.

Clones live in git-ignored `.corpus/`. A missing clone is a hard error naming `make test-corpus`, never a skipped test.

### [CORPUS-CEILINGS] Resource budget

Every manifest carries `ceilings.max_wall_seconds` and `ceilings.max_peak_rss_mb`, measured by running the release binary under `/usr/bin/time`.

The memory ceiling is **7168 MB — the RAM of a standard GitHub Actions runner**, not an invented number. Deslop ships a GitHub Action; a scan that exceeds the runner is a scan the documented product cannot perform. The wall ceiling is deliberately loose: it exists to catch a hang, not to police throughput.

### [CORPUS-RECALL] Curated duplicates

`must_find` lists duplicates a human confirmed byte-for-byte, each with the `diff` that proved it. A cluster must span every path in the entry; anything less is a false negative on code known to be duplicated.

**An empty `must_find` asserts nothing, and the suite says so out loud** — such a run prints `ACCURACY UNASSERTED` and has proven only that the scan fit its budget. A green corpus test is not evidence of accuracy unless the repository has curated entries. An entry that lists no files fails rather than passing vacuously.

`must_find_type2` is the same contract for **renames**: pairs a human confirmed are the same code with different identifiers, each carrying `verified` — how the rename was checked — alongside `why` and `files`. Byte-identical clones belong in `must_find`; a Type-2 pair is not byte-identical by definition, so it can never reach the `identical` bucket and must be earned from the content gate. The entry passes only when a cluster spans its files, that cluster is `nearly_identical`, its shown signals carry saturating shape evidence, and the curated occurrences are themselves visible. A hidden occurrence fails: recall is what the report *shows*, not what it contains.

`must_find_type2_status` states the curation position in words and must not contradict the list. Because an empty list silently asserts nothing, the manifests are themselves under test: curated Type-2 ground truth must exist in at least two languages, and every entry must name at least two distinct files with its human evidence attached.

### [CORPUS-PRECISION] Ranking rules

Ranking *is* the product, so the head of the report is where a false positive does the most damage. Two rules apply to the top-ranked clusters:

- `must_not_rank_first` names framework-mandated shapes for that language. Such code cannot be extracted or merged, so it must never outrank genuine copy-paste. Compare [CLONE-NOISE-*](noise.md).

  `forbidden_top_supertypes` is a list of **base-type names**, matched as an AST predicate: a ranked cluster fails when the type declaration its first occurrence overlaps names one of them in the language's heritage clause. Both the declaration containing the occurrence and any declaration the occurrence contains count, because the ranked occurrence is usually the mandated *member* — Flutter's `createState` — not the class header that makes it mandated.

  It is never matched against source text. The rule shipped as `text.contains("extends StatefulWidget")` and was wrong in both directions (gh #401): it fired on a comment or string literal that merely mentioned the supertype, and it missed a declaration whose clause was wrapped across lines. Type arguments are not base types — `extends State<LedgerView>` names `State` — and a language with no curated heritage grammar fails the gate rather than passing it, so a rule that cannot fire is never mistaken for a rule that found nothing.
- Language-agnostic: a top-ranked cluster that is overwhelmingly digits and separators is a data table, and must carry `category: data` rather than ranking at full logic weight.

### [CORPUS-BASELINE] The known-failures ratchet

`corpus/known-failures.json` records checks that already fail, each mapped to a tracked issue.

- **Locally, `make test-corpus` ignores the file entirely** and fails on everything. Local runs stay honest.
- **`make test-corpus-ci` reads it** and fails only on checks *not* recorded — that is the regression gate, and it is what the scheduled workflow runs.
- Adding an entry is admitting a defect shipped. Fixing it and deleting the entry is the only correct exit. Widening a ceiling or deleting an assertion to clear an entry is prohibited.
- Check ids are rank-independent by construction: ranks move between runs while [PIPELINE-DETERMINISM] is unmet, so a rank-bearing key would churn the baseline into uselessness.

### [CORPUS-CI] Scheduled, never blocking

The corpus workflow is scheduled and dispatchable, and has **no `pull_request` trigger**. It reports; it does not gate. Merges must not depend on a suite that is red on tracked defects, and a suite that is red on tracked defects must not be quietly weakened to make it green.

Scheduled runs scan a slice sized to finish in about a minute, not the whole corpus. The job summary states which repositories were skipped, so a green scheduled run is never misread as full coverage.
