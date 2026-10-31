# Release review — `bdcd58b8` → `HEAD`

Scope: `git diff bdcd58b8..HEAD`, `git log bdcd58b8..HEAD` (14 commits). Reviewed with
AGENTS.md accuracy-audit rules in force: the ranking is by risk of a false positive,
a false negative, or an inaccurate reported figure. No source files were modified.

Commits reviewed:

```
c53641f4 Keep only the highest-stakes rules as AGENTS.md invariants
d83727c0 Trim AGENTS.md version comparison
70169294 Merge AGENTS.md accuracy rules into one quarantine process
10258897 Tighten AGENTS.md invariants
f5d5a344 Merge AGENTS.md hard and universal rules into invariants and recommendations
6c391634 Draw the AGENTS.md pipeline as a Mermaid flowchart
739a8c2c Trim AGENTS.md documentation rules
29267188 Trim AGENTS.md universal rules
6a29d7f6 Tighten AGENTS.md hard rules, drop decorative emojis and normalise casing
e651b675 Add Osprey language support, fail the corpus gate on unaccepted wrong judgments, and bound MCP frames and long-path IPC
e7b9bdf2 Fail corpus runs only on regressions, tie every run to its commit, and write judged pairs as pass or fail (#594)
828d5129 Hold every judged corpus pair to its recorded outcome (#592)
6a5bdf29 Preserve complete renamed dispatcher copies (#593)
a93476b6 Judge clone types (Type I–IV), shorten judging passes, and register second repositories (#590)
```

## Findings (most severe first)

### 1. 🔴 File discovery is now fail-hard: one bad manifest / marker conflict / unreadable file aborts the whole scan

- **Where:** `crates/deslop-core/src/lang/osprey_selection.rs:63` (`OspreyProjects::load`),
  `:77-96` (`language`), `:147` (`flavor_marker`), `:186` (`extension_flavor`);
  `crates/deslop-core/src/discover.rs:350-368` (`eligible_file`);
  `crates/deslop-core/src/pipeline/session/initialise.rs:70`.
- **Defect:** `discover_files` was previously infallible and never read file contents.
  It now returns `Result<DiscoveryResult, CoreError>` and, before registering a
  file, (a) walks the entire tree for `osprey.toml` manifests and propagates any
  `toml::from_str` failure, and (b) reads + parses every `.py` / `.osp` / `.ospml`
  file to detect a leading `osprey: flavor=` marker, propagating `read_source` IO
  errors, `parse_source` failures, and marker-vs-extension conflicts. `initialise`
  propagates the first such error with `?`, so the error is fatal to the run.
- **Concrete failure scenario (whole-run false negative):**
  1. Any repository (even a non-Osprey one) that happens to contain a file named
     `osprey.toml` whose content is not valid TOML — e.g. a config for an unrelated
     tool named Osprey, or a hand-edited manifest — makes `OspreyProjects::load`
     return `Err`, and `deslop` exits with an error **before reporting a single
     cluster**.
  2. A Python file whose first comment line is `# osprey: flavor=default` (a
     plausible documentation string), or a `.osp` file whose first comment is
     `// osprey: flavor=python`, trips `extension_flavor`'s
     "flavor marker contradicts the extension" `LanguageSelection` error and
     aborts discovery.
  3. A `.py`/`.osp`/`.ospml` file that becomes unreadable between the walk and the
     read (permissions, race) aborts discovery via `read_source(...)?`.
- **Why this is a regression:** the rest of the discovery path is deliberately
  tolerant — `discover.rs:208-223` documents that "one bad pattern must never"
  break an ignore file. The new manifest path is fail-closed over the *entire
  scan* rather than fail-soft over the *one file* (which is how unparseable source
  is already handled during corpus build). The tests only cover a malformed
  manifest placed under an *excluded* directory
  (`crates/deslop/tests/cli/detection/osprey_flavors.rs`,
  `excluded_project_manifests_never_block_or_steer_the_scan`); there is **no test**
  pinning the behaviour of a malformed manifest at the non-excluded root.
- **Recommended fix:** skip (with a `tracing::warn!`) manifests that fail to parse,
  mirroring the ignore-file policy, and/or only treat a manifest as an Osprey
  project when it is adjacent to Osprey sources; treat a marker/extension conflict
  as "analyse as the extension's default language" (or skip the one file) instead of
  failing the run.

### 2. 🟠 Discovery now parses every Python/Osprey file and walks the tree twice (perf + a scope gap)

- **Where:** `crates/deslop-core/src/discover.rs:363` (per-file `read_source` +
  `flavor_marker`), `crates/deslop-core/src/lang/osprey_selection.rs:63-75`
  (second full walk).
- **Defect:** every `.py`/`.osp`/`.ospml` file is fully read and tree-sitter-parsed
  during discovery *and again* during corpus build (2× I/O + 2× parse for the
  largest language family). `OspreyProjects::load` additionally runs
  `ignore::WalkBuilder::new(root).build()` with **no** `standard_filters(true)`,
  so it does not respect `.gitignore`/hidden rules the way `discover_files` does.
- **Failure scenario:** on a large Python monorepo, discovery time and memory
  approximately double; a `osprey.toml` under a gitignored or hidden vendor
  directory that is *not* in the built-in component list
  (`crates/deslop-core/src/config/builtin.rs:54-73`) is still loaded and can steer
  or fail selection even though `discover_files` would never have admitted that
  directory. Not a correctness defect in isolation, but it widens finding #1's
  blast radius.

### 3. 🟠 Code-action parser is derived from the cluster's first occurrence, not the edited file

- **Where:** `crates/deslop-lsp/src/code_action.rs:56-62`.
- **Defect:** `build_for_range` now selects the parser with
  `refactor::parser_for_occurrence(cluster.occurrences.first(), path)`. For a
  cross-language cluster (signature/embedding clusters do cross languages), the
  first occurrence can be a different file and a different language than the edited
  `path`. `refactor::compute_plan` then parses the edited file's bytes with the
  wrong grammar.
- **Failure scenario:** editing file `b.py` (Python) in a cluster whose first
  occurrence is `a.rs` (Rust) parses `b.py` with the Rust grammar.
  `preconditions::occurrence_scopes` will normally find no matching scope and
  `compute_plan` returns `Ok(None)`, so the extract/merge code action is silently
  **not offered** (a false negative for the refactor surface); in pathological
  cases a mismatched tree could yield a wrong plan. The parser should be selected
  from the occurrence whose `path` equals the edited `path`.
- **Related minor defect:** `crates/deslop-core/src/report_render.rs:154,168`
  stores `language: Some(language)` with `language = file_languages.get(...)
  .copied().unwrap_or("")`. A missing entry becomes `Some("")` rather than `None`,
  which defeats the `unwrap_or_else(|| language_for_path(...))` fallback in
  `refactor::parser_for_occurrence` (`refactor/mod.rs:148`) and in
  `canonical_language` (`render/html.rs`) / `cluster_language`
  (`deslop-mcp/src/page.rs`). Low likelihood today (all analysed files carry a
  language), but it silently nullifies the documented "legacy reports fall back to
  the path" behaviour.

### 4. 🟡 Config/`.deslop.toml`/`osprey.toml` reload re-reads the whole workspace

- **Where:** `crates/deslop-core/src/pipeline/session/change.rs:298-312`.
- **Defect:** `apply_newly_included` used to apply only freshly-admitted paths; it
  now iterates and applies **every** discovered path. The unchanged-source early
  return sits *after* `parse_one_file` (`change.rs:146-154`), so each live config
  reload re-reads (and, when the incremental cache is cold, re-parses) the entire
  workspace.
- **Failure scenario:** editing `.deslop.toml` or `osprey.toml` on a large repo
  stalls the live session. Correctness is preserved (the early return prevents
  re-splicing), so this is a performance/latency finding, not an accuracy one.

### 5. Corpus gate semantics changed materially (verified, not a defect)

- **Where:** `crates/deslop-test-support/src/corpus_score/verdict.rs`,
  `outcomes.rs`, `history.rs`, `gate.rs`, `scripts/corpus/score-gate.sh`.
- **What:** the register-threshold "breach" gate was replaced by per-pair recorded
  outcomes (`corpus/register/outcomes.json`) plus append-only run history backslide
  detection (`corpus/history/runs/`). A new false positive/negative (`Unrecorded`)
  or a previously-passing pair now failing (`Regressed` / `Backslide`) fails the
  run; only a pair explicitly on record as failing is tolerated.
- **Assessment:** this is a strictness *increase* and is heavily unit-tested
  (`tests/verdict/gate.rs`, `tests/verdict.rs`, `history.rs`, `outcomes.rs`). One
  residual risk worth a deliberate decision rather than a code fix: the gate now
  trusts `outcomes.json` and `history/runs/` as the source of truth, so a stale or
  hand-edited record can mask a regression (`Standing::StillFailing` tolerates it
  indefinitely). The record is fail-closed on malformed content (`load` errors),
  which is correct.

## AGENTS.md rule checks

- **Files over 500 lines:** none among changed Rust source files (the `session.rs`
  split is a net reduction; `history.rs` 385, `outcomes.rs` 360). Data files
  `site/src/assets/data/issues.json` (5,247) and
  `site/tests/fixtures/github-issues.json` (5,347) are generated data, not code.
- **`unwrap()`/`expect()` in production:** none introduced (verified across all
  changed `.rs` files).
- **Regex on source:** none; Osprey marker detection is tree-sitter + `strip_prefix`.
- **Calculations outside Rust:** percentage/duplication figures remain in
  `deslop-core`; the JS `merge-verdicts.mjs` and `score-gate.sh` are orchestration/
  judging-input scripts (exempt).
- **Weakened/deleted tests:** the `tests/verdict.rs` removals are a rewrite of the
  old threshold-breach tests into stricter per-pair-outcome tests — not a weakening.
  No `#[ignore]`, `skip`, or deleted assertion was found.
- **Linter suppressions / duplicate code paths:** none found; the
  `canonicalise_path` duplicate was consolidated into `open_buffers`
  (`pipeline/mod.rs`).
- **Test coverage gaps (tied to findings):** no test pins (1) a malformed
  `osprey.toml` at a non-excluded root, (2) a marker/extension conflict in a real
  `.py`/`.osp` file, or (3) a cross-language cluster driving the code-action path.

## GO / NO-GO

**NO-GO (conditional).** The corpus-gate and MCP-bounding work is solid and
stricter than what it replaces, and the Osprey detection logic itself is well
tested. But finding #1 is a concrete, untested, whole-run false-negative path
introduced in this range: a single stray or malformed `osprey.toml`, a
`# osprey: flavor=default` comment in a `.py` file, or a marker/extension
disagreement makes `deslop` fail before reporting any cluster. In accuracy-audit
mode that is the worst failure class. Recommend: make manifest/marker resolution
fail-soft (skip/log the one file or manifest) and add a fixture pinning a
malformed root manifest before releasing HEAD.
