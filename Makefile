# agent-pmo:b636503
# =============================================================================
# Standard Makefile — Deslop
# Cross-platform: Linux, macOS, Windows (via GNU Make)
# Rust CLI. See docs/specs/SPEC.md and docs/repo-index.md.
#
# Targets prefixed with `_` are INTERNAL: hidden from the IDE make task list,
# invoked only by other targets or by CI. The public targets below are the
# human entry points and are the only ones `make help` lists.
# =============================================================================

.PHONY: build dup-gate test test-ollama lint fmt clean ci ci-ollama setup help deployment-verify coverage coverage-run coverage-report _ci-analyze _ci-contract-tests _ci-build _ci-gate _ci-test _ci-test-rust _ci-test-vsix vsix-package vsix-rebuild android-studio-rebuild android-studio-rebuild-reinstall typediagram-gen _delete-path-binaries _kill-deslop-processes _vsix-install _vsix-node-modules _vsix-build _vsix-test _vsix-test-ollama _vsix-coverage _vsix-coverage-check _vsix-webview-coverage _vsix-playwright-html _vsix-install-code _vsix-clean _vsix-stage-bundled-binaries _vsix-stage-and-package _jetbrains-build _jetbrains-verify _jetbrains-package _jetbrains-test _jetbrains-real-binary-test _android-studio-install _android-studio-uninstall

_JETBRAINS_DIR := clients/jetbrains

# ---------------------------------------------------------------------------
# OS Detection
#
# Every recipe in this file is POSIX shell — `case`, `for`, `[ -f ]`, `||`, and
# `$(RM)`/`$(MKDIR)` interpolated inside those constructs. Windows therefore
# runs them under Git Bash, and by absolute path: `bash.exe` resolved by name
# finds WSL's bash in System32 first, which sees a different filesystem and
# cannot build this checkout. Override GIT_BASH when Git for Windows lives
# somewhere other than its default location. `uname` under it reports
# MINGW*/MSYS*, which _vsix-stage-bundled-binaries maps to the win32-x64 target.
# ---------------------------------------------------------------------------
GIT_BASH ?= C:/Program Files/Git/usr/bin/bash.exe

RM = rm -rf
MKDIR = mkdir -p

ifeq ($(OS),Windows_NT)
  SHELL := $(GIT_BASH)
  .SHELLFLAGS := -c
  HOME ?= $(USERPROFILE)
  # The JetBrains wrapper is checked in and is the source of truth.
  # Override `GRADLE=...` only when deliberately testing another runtime.
  GRADLE ?= ./gradlew.bat
else
  GRADLE ?= ./gradlew
endif

# ---------------------------------------------------------------------------
# Coverage — single source of truth is coverage-thresholds.json
# See docs/specs/SPEC.md and REPO-STANDARDS-SPEC [COVERAGE-THRESHOLDS-JSON].
# ---------------------------------------------------------------------------
_COVERAGE_THRESHOLDS_FILE := coverage-thresholds.json

# ---------------------------------------------------------------------------
# [TEST-SELECTION] The cargo features every required test command enables.
# One definition, because a second copy is how a test stops being compiled
# without anyone noticing: `deslop-lsp/profiling` was off in every gate, so
# `profile_dir_writes_non_empty_firefox_profile_on_shutdown` was absent
# rather than skipped, and `--all-features` linting found two `missing_docs`
# violations in code no ordinary run had ever compiled.
# `deslop-lsp/tests/observability_heartbeat.rs` asserts each feature here is
# live, so dropping one fails the suite instead of silently deleting a test.
# ---------------------------------------------------------------------------
_TEST_FEATURES := deslop-core/live,deslop-lsp/profiling

# =============================================================================
# Standard Targets
# =============================================================================

## build: Compile/assemble all artifacts
build:
	@echo "==> Building..."
	cargo build --release --workspace

## typediagram-gen: Regenerate wire-format Rust IPC models from
##                  `docs/models/*.td` via the typediagram CLI. The
##                  generated file is gitignored; cargo's build.rs
##                  invokes this same script automatically, so manual
##                  invocation is only needed when iterating on the
##                  .td spec or the generator script itself.
typediagram-gen:
	@echo "==> typediagram-gen: regenerating IPC models from docs/models/live-ipc.td"
	node scripts/typediagram/generate.mjs

## test: Fail-fast tests + coverage + per-crate threshold enforcement.
##       See REPO-STANDARDS-SPEC [TEST-RULES] and [COVERAGE-THRESHOLDS-JSON].
##       [TEST-SELECTION] Runs every test in the workspace. Nothing is
##       selected by name: `cargo test --skip` matches a substring of the
##       *test name*, so `--skip ollama_ --skip corpus_` silently dropped
##       the corpus gate's own self-tests and the mock-Ollama suites
##       (gh #412). The Rust embedding tests are hermetic — they drive an
##       in-process mock server or a deliberately dead endpoint and need no
##       daemon. [TEST-SELECTION-SKIP] The suites that must not run here say
##       so at their own declaration, with `#[ignore = ".."]`: the reason is
##       printed on every run and `skip_policy_contract` holds it to the
##       policy. `#[ignore]` still compiles and lints the target — skipping
##       costs coverage of a test's execution, never of its compilation.
##       The `--ignore-filename-regex` list lives in
##       `coverage-thresholds.json` under `.rust.ignore_filename_regex`
##       (single source of truth). Per-crate thresholds live under
##       `.rust.crates.<crate>`; `_coverage_check` enforces each one
##       independently — no workspace roll-up masking.
##       [CI-RELEASE-BUILD] `--release` matches the profile every other
##       gate runs on. The duplication gate, the deployment gates and the
##       whole VSIX E2E suite all exercise `target/release`, so a
##       debug-profile test run gates release artifacts on code it never
##       executed.
##       [CI-RELEASE-BUILD] Coverage is **not** measured here, and that is
##       the point. `cargo llvm-cov` compiles into `target/llvm-cov-target`
##       because an instrumented artifact can never share a fingerprint
##       with an uninstrumented one, so a coverage run reuses nothing from
##       `target/release` and rebuilds the whole workspace. Measured on CI
##       run 32542178321: 21m24s compiling, 1m34s running. Coverage moved
##       to `make coverage`, which owns its own cache and runs in parallel,
##       so this target is a cache hit against the release build every
##       other gate already uses and the suite reports in minutes.
test: _delete-path-binaries typediagram-gen
	@echo "==> Testing (fail-fast, release profile)..."
	cargo test --release --workspace --all-targets --features $(_TEST_FEATURES)

_coverage_check:
	@_lcov="$${RUST_LCOV:-lcov.info}"; \
	 if [ ! -f "$$_lcov" ]; then echo "FAIL: $$_lcov not found"; exit 1; fi; \
	 if [ ! -f "$(_COVERAGE_THRESHOLDS_FILE)" ]; then echo "FAIL: $(_COVERAGE_THRESHOLDS_FILE) not found"; exit 1; fi; \
	 _default=$$(jq -r '.rust.default_threshold' "$(_COVERAGE_THRESHOLDS_FILE)"); \
	 if [ "$$_default" = "null" ] || [ -z "$$_default" ]; then \
	   echo "FAIL: $(_COVERAGE_THRESHOLDS_FILE) missing .rust.default_threshold"; exit 1; \
	 fi; \
	 _failed=0; \
	 for _crate in deslop-core deslop deslop-lsp deslop-mcp; do \
	   _threshold=$$(jq -r ".rust.crates.\"$$_crate\" // .rust.default_threshold" "$(_COVERAGE_THRESHOLDS_FILE)"); \
	   if [ "$$_threshold" = "null" ] || [ -z "$$_threshold" ]; then \
	     echo "FAIL: no threshold for crate $$_crate in $(_COVERAGE_THRESHOLDS_FILE)"; \
	     _failed=1; \
	     continue; \
	   fi; \
	   _counts=$$(awk -v crate="crates/$$_crate/src/" '\
	     /^SF:/ { in_crate = (index($$0, crate) > 0) ? 1 : 0; next } \
	     /^LH:/ { if (in_crate) lh += substr($$0, 4) + 0 } \
	     /^LF:/ { if (in_crate) lf += substr($$0, 4) + 0 } \
	     /^end_of_record/ { in_crate = 0 } \
	     END { printf "%d %d", lh, lf } \
	   ' "$$_lcov"); \
	   _lh=$$(echo "$$_counts" | awk '{print $$1}'); \
	   _lf=$$(echo "$$_counts" | awk '{print $$2}'); \
	   if [ "$$_lf" -eq 0 ]; then \
	     echo "FAIL: crate $$_crate has no covered lines in $$_lcov (all files filtered or crate has no tested source)"; \
	     _failed=1; \
	     continue; \
	   fi; \
	   _pct=$$(awk -v lh="$$_lh" -v lf="$$_lf" 'BEGIN { printf "%.1f", lh / lf * 100 }'); \
	   _pass=$$(awk -v lh="$$_lh" -v lf="$$_lf" -v t="$$_threshold" 'BEGIN { print (lh / lf * 100 + 1.0 >= t) ? 1 : 0 }'); \
	   if [ "$$_pass" -eq 1 ]; then \
	     printf "  %-14s %s%% (threshold %s%% + 1%% slack) OK\n" "$$_crate" "$$_pct" "$$_threshold"; \
	   else \
	     printf "  %-14s %s%% (threshold %s%% + 1%% slack) FAIL\n" "$$_crate" "$$_pct" "$$_threshold"; \
	     _failed=1; \
	   fi; \
	 done; \
	 _total_counts=$$(awk '\
	   /^LH:/ { lh += substr($$0, 4) + 0 } \
	   /^LF:/ { lf += substr($$0, 4) + 0 } \
	   END { printf "%d %d", lh, lf } \
	 ' "$$_lcov"); \
	 _total_lh=$$(echo "$$_total_counts" | awk '{print $$1}'); \
	 _total_lf=$$(echo "$$_total_counts" | awk '{print $$2}'); \
	 if [ "$$_total_lf" -eq 0 ]; then \
	   echo "FAIL: no covered lines in $$_lcov"; \
	   _failed=1; \
	 else \
	   _total_pct=$$(awk -v lh="$$_total_lh" -v lf="$$_total_lf" 'BEGIN { printf "%.1f", lh / lf * 100 }'); \
	   echo "Workspace total: $$_total_pct% ($$_total_lh/$$_total_lf lines)"; \
	 fi; \
	 exit "$$_failed"

## lint: Run all linters/analyzers (read-only). Does NOT format.
##       Also enforces the taxonomy content gate
##       ([CLONE-BUCKETS-DUAL-LABEL]): every product-facing `Type-N`
##       mention in site/src and examples must co-locate a canonical
##       bucket label. [TEST-SELECTION]: the release gate may not select
##       tests by name substring, and [TEST-SELECTION-SKIP]: every `#[ignore]`
##       must carry a category, an issue, a spec id and a plan. It also proves
##       the composite-action step scanner still finds every `run` body, since
##       the shell-injection gate in `deployment-verify` reads through it.
##       clippy runs `--all-targets`, which now covers the corpus suite too: `#[ignore]`
##       keeps it compiled and linted where `required-features` had removed
##       it from the build entirely. Commit 77bcbaed5 left it uncompilable
##       for exactly that reason.
##       Depends on typediagram-gen so the wire-generated module exists
##       before clippy parses the workspace on a fresh checkout.
lint: _ci-analyze _ci-contract-tests

# _ci-analyze: Read-only analyzers only. CI runs this before the build; test
#   harness self-tests live in _ci-contract-tests and run in the test phase.
_ci-analyze: typediagram-gen
	@echo "==> Linting..."
	cargo clippy --release --all-targets --workspace --features $(_TEST_FEATURES) -- -D warnings
	@bash scripts/repository/taxonomy-gate.sh
	@node scripts/actions/verify-env-path-writes.mjs

# _ci-contract-tests: Test the repository's Node-based gates exactly once.
#
#                     Carries `_vsix-node-modules` because the first group
#                     below runs the extension's own scripts, and one of them
#                     ([VSIX-TESTING-COVERAGE-RESTORE]) drives
#                     `extension-coverage.mjs` as a real process — which
#                     imports `istanbul-lib-coverage`. The CI job that runs
#                     this target builds no VSIX, so without the dependencies
#                     the gate dies on `ERR_MODULE_NOT_FOUND` and the contract
#                     it exists to prove is never exercised. Declaring the
#                     prerequisite here rather than in the workflow keeps the
#                     target runnable from a clean checkout.
_ci-contract-tests: _vsix-node-modules
	@echo "==> VSIX harness + packaging script gates (unit)..."
	@node --test clients/vscode/scripts/*.test.mjs
	@echo "==> PATH/env injection gate ([ACTION-ENVPATH])..."
	@node --test scripts/actions/verify-env-path-writes.test.mjs
	@echo "==> Composite-action step scanner proof ([ACTION-TESTS])..."
	@node --test scripts/actions/action-yaml.test.mjs
	@echo "==> Docs installer snippet fail-closed gate ([DEPLOY-DOCS-INSTALLER-FAILCLOSED])..."
	@node --test scripts/deployment/installer-snippet.test.mjs
	@echo "==> Archive reader/writer gate ([DEPLOY-VSIX-PACKAGE])..."
	@node --test scripts/deployment/zip.test.mjs
	@echo "==> PATH-scrub gate ([DEPLOY-EXTERNAL-MCP-CONSUMER])..."
	@node --test scripts/repository/scrub-path-binaries.test.mjs
	@echo "==> Process-scrub + host-shell gate ([DEPLOY-EXTENSION-BUNDLED-TESTS])..."
	@node --test scripts/repository/kill-deslop-processes.test.mjs
	@node --test scripts/repository/posix-shell.test.mjs
	@echo "==> Duplication-gate provenance gate ([CI-DESLOP])..."
	@node --test scripts/repository/dup-gate-source.test.mjs
	@echo "==> Accuracy-gate wiring gate ([CORPUS-SCORE])..."
	@node --test scripts/repository/score-gate-source.test.mjs
	@echo "==> Blinded judging folder gate ([CORPUS-REGISTER-WORKSPACE])..."
	@node --test scripts/repository/judging-workspace.test.mjs
	@echo "==> Verdict merge gate ([CORPUS-REGISTER-MERGE])..."
	@node --test scripts/repository/verdict-merge.test.mjs
	@echo "==> Test-selection gate ([TEST-SELECTION])..."
	@node --test scripts/repository/test-selection.test.mjs
	@echo "==> Coverage-isolation gate ([CI-COVERAGE-ISOLATION])..."
	@node --test scripts/repository/coverage-isolation.test.mjs

## fmt: Format all code in-place. Pass CHECK=1 for read-only check (CI use).
##      Depends on typediagram-gen because rustfmt walks the module tree
##      and refuses to run when `mod wire_generated;` cannot resolve. The
##      generated file is gitignored per CLAUDE.md, so on a clean CI
##      checkout it must be produced before fmt walks the sources.
fmt: typediagram-gen
	@echo "==> Formatting$(if $(CHECK), (check mode),)..."
	@_fmt_out=$$(cargo fmt --all$(if $(CHECK), --check,) 2>&1); _fmt_rc=$$?; \
	 echo "$$_fmt_out" | grep -v "unstable features are only available in nightly channel" || true; \
	 exit $$_fmt_rc

## clean: Remove all build artifacts
clean:
	@echo "==> Cleaning..."
	cargo clean
	$(RM) lcov.info
	$(RM) .deslop-cache

## ci: [CI-RELEASE-BUILD] The three phases `.github/workflows/ci.yml` runs,
##     in the same order and over the same artifacts, so a green `make ci`
##     locally means exactly what a green pipeline means:
##
##       1. `_ci-build` — every release artifact, compiled once: the
##          workspace, every release test binary, and the VSIX bundle with
##          those same binaries staged into it. This is the only phase that
##          compiles anything.
##       2. `_ci-gate` — the gates that read those artifacts (duplication,
##          deployment manifest). Cheap, and they fail before a suite runs.
##       3. `_ci-test` — the Rust suite, the VSIX suites and coverage, in
##          parallel, executing what phase 1 built.
##
##     Ollama-gated suites are excluded; they run via `make ci-ollama`.
ci:
	@$(MAKE) _ci-build
	@$(MAKE) _ci-gate
	@$(MAKE) _ci-test

## setup: Post-create dev environment setup (used by devcontainer).
##        Version pin for `typediagram` must match `.github/workflows/ci.yml`
##        per CLAUDE.md dependency-pinning rules. The CLI is required by
##        `make typediagram-gen` and the deslop-core `build.rs` on every
##        cargo build, so a fresh devcontainer needs it on PATH.
setup:
	@echo "==> Setting up development environment..."
	rustup component add llvm-tools-preview clippy rustfmt
	cargo install --locked cargo-llvm-cov
	npm install -g typediagram@0.11.0
	@echo "==> Setup complete. Run 'make ci' to validate."

# =============================================================================
# Repo-Specific Targets
# =============================================================================

# _ci-build: [CI-RELEASE-BUILD] Phase 1 of `make ci` — every release artifact
#   the later phases consume, produced exactly once. Mirrors the CI `lint` and
#   `build` jobs step for step.
#
#   It does NOT compile the release test binaries. Phase 3 runs the suite under
#   `cargo llvm-cov`, which instruments into its own target directory and
#   reuses none of them, so `cargo test --no-run` here only linked artifacts
#   nothing executes — 282s of every CI pipeline and of every local `make ci`.
#   `make lint` already type-checks every test target via `--all-targets`, and
#   phase 3 links them for real.
_ci-build:
	@$(MAKE) fmt CHECK=1
	@$(MAKE) lint
	@$(MAKE) build
	@$(MAKE) _vsix-build
	@$(MAKE) _vsix-stage-bundled-binaries

# _ci-gate: Phase 2 — the gates that only read phase 1's artifacts. They are
#   seconds of work against a report, so they run before any suite: a
#   duplication or deployment regression should not wait behind the tests.
_ci-gate:
	@$(MAKE) dup-gate
	@$(MAKE) deployment-verify

# _ci-test: Phase 3 — run every suite exactly once, instrumented for coverage.
#   `coverage-report` is deliberately excluded: it is phase 4 and runs only
#   after all collectors complete.
_ci-test:
	@$(MAKE) -j2 _ci-test-rust _ci-test-vsix

# _ci-test-rust: The Rust half of phase 3.
_ci-test-rust:
	@$(MAKE) coverage-run

# _ci-test-vsix: The VSIX half of phase 3 — extension-host and webview
#   coverage collection plus the standalone HTML-report Playwright check.
_ci-test-vsix:
	@$(MAKE) _vsix-coverage
	@$(MAKE) _vsix-webview-coverage
	@$(MAKE) _vsix-playwright-html

## test-shard: [TEST-SELECTION] One optional slice of the release suite for
##             local diagnosis. `make test` remains the whole
##             suite and is what a developer runs. The split is over test
##             *binaries*, never test names — `cargo test --skip` matches a
##             substring of the name and silently dropped whole suites that
##             way (gh #412) — and `test-shards.test.mjs` proves the union of
##             the shards is the whole set for every shard count CI uses.
##             [TEST-ONE-BINARY] Each crate's suites are modules of one
##             binary, so the partition is over 13 binaries rather than the
##             200 that preceded it, and the bulk of the runtime sits in
##             `deslop`'s. libtest already runs that binary's tests across
##             every core, so a shard is worth having for the crates it can
##             actually separate, not for balance.
##             Usage: `make test-shard SHARD=1 SHARDS=4`.
test-shard: _delete-path-binaries typediagram-gen
	@echo "==> Testing shard $(SHARD)/$(SHARDS) (fail-fast, release profile)..."
	@node scripts/repository/test-shards.mjs --shard $(SHARD) --of $(SHARDS) --features $(_TEST_FEATURES)

## coverage: [CI-RELEASE-BUILD] [COVERAGE-THRESHOLDS-JSON] Instrumented
##           release run + per-crate threshold enforcement. Split out of
##           `make test` because llvm-cov's instrumented target directory
##           shares nothing with `target/release`: bundling them made every
##           test run pay a 21-minute cold compile of the whole workspace
##           for a suite that executes in 94 seconds. Thresholds live in
##           `coverage-thresholds.json` and `_coverage_check` enforces each
##           crate independently — no workspace roll-up masking. The
##           `--ignore-filename-regex` list has the same single source.
##
##           [CI-COVERAGE-ISOLATION] The explicit `clean --workspace` is not
##           tidiness. `--no-report`
##           leaves both the raw profiles and the previous build's objects in
##           place, and `report` maps the merged profile against every object
##           it finds. An object from an earlier build carries an older
##           coverage mapping of the same file, so its line table is unioned
##           with the current one and the file is credited with lines it no
##           longer has — all of them unexecuted. Measured on this tree:
##           `app.rs` is 193 lines and 99.5% covered, and a single stale
##           object made it 362 lines and 53.0%, dragging `deslop-lsp` from
##           94.0% to 85.1% and the workspace from 94.5% to 92.6%. Cleaning
##           profiles alone (`--profraw-only`) does not fix it — the stale
##           object survives. `--workspace` drops only this repository's
##           artifacts, so third-party dependencies stay cached and the
##           honest number costs about a minute.
##           Carries `_delete-path-binaries` for the same reason `test`
##           does: this target runs the suite, and a Deslop binary leaked
##           onto PATH would shadow the built one.
coverage: coverage-run coverage-report

coverage-run: _delete-path-binaries typediagram-gen
	@echo "==> Coverage test collection (instrumented release)..."
	rustup component add llvm-tools-preview 2>/dev/null || true
	cargo llvm-cov clean --workspace
	cargo llvm-cov --release --workspace --all-targets --features $(_TEST_FEATURES) --no-report

coverage-report:
	@echo "==> Coverage calculation and threshold enforcement..."
	@_rust_ignore=$$(jq -r '.rust.ignore_filename_regex' "$(_COVERAGE_THRESHOLDS_FILE)"); \
	 cargo llvm-cov --release --features $(_TEST_FEATURES) report \
	    --ignore-filename-regex "$$_rust_ignore" \
	    --lcov --output-path lcov.info
	@$(MAKE) _coverage_check RUST_LCOV=lcov.info

## test-ollama: [TEST-SELECTION] The VSIX `.vscode-test-ollama.mjs` suite —
##              the only tests that need a real daemon on 127.0.0.1:11434
##              with `nomic-embed-text` pulled. The Rust embedding suites are
##              hermetic (mock server / dead endpoint) and run in `make test`;
##              they were never daemon-gated, only name-filtered (gh #412).
test-ollama: _vsix-test-ollama

## ci-ollama: `make ci` plus `make test-ollama`.
ci-ollama: ci test-ollama

## test-corpus: [CORPUS-*] Accuracy + resource suite against real public repos
##              pinned by `corpus/*.json`. Clones into git-ignored `.corpus/`
##              first (re-runs are free once cloned). [TEST-SELECTION-SKIP]
##              Every test in the suite is `#[ignore]`d as
##              [SKIP-TOO-LARGE-FOR-CI] (gh #422) — it needs the network and
##              measures wall time and peak memory, which are
##              runner-dependent — so `--ignored` is what selects it here,
##              scoped to Cargo's dedicated `corpus_repos` test target.
##              `make test`/`make ci` still compile and lint the target. Run
##              this when touching the pipeline.
test-corpus:
	node scripts/corpus/fetch-corpus.mjs
	cargo build --release --bin deslop
	cargo test --release -p deslop --test corpus_repos -- --ignored --nocapture --test-threads=1

## test-corpus-ci: `make test-corpus` in baseline mode — failures already
##                 recorded in `corpus/known-failures.json` are reported but
##                 do not fail the run; anything new does. Used by the
##                 scheduled corpus workflow so tracked defects stay visible
##                 without blocking. Local `make test-corpus` ignores the
##                 baseline and stays strictly red.
test-corpus-ci: export DESLOP_CORPUS_BASELINE = 1
test-corpus-ci:
	node scripts/corpus/fetch-corpus.mjs $(CORPUS_REPOS)
	cargo build --release --bin deslop
	@cargo test --release -p deslop --test corpus_repos -- --ignored --list > $(CORPUS_NAMES)
	@for t in $(CORPUS_TESTS); do \
	   grep -qFx "$$t: test" $(CORPUS_NAMES) || { \
	     echo "==> corpus: CORPUS_TESTS names \`$$t\`, which no test in corpus_repos answers to."; \
	     echo "==> corpus: --exact would select nothing and libtest would exit 0 (gh #412)."; \
	     exit 1; }; \
	 done
	@mkdir -p $(CORPUS_LOGS)
	@fail=0; for t in $(CORPUS_TESTS); do \
	   log=$(CORPUS_LOGS)/$$t.log; \
	   cargo test --release -p deslop --test corpus_repos -- --ignored --exact --nocapture --test-threads=1 $$t > $$log 2>&1 || fail=1; \
	   cat $$log; \
	   ran=$$(awk '$$1 == "running" && ($$3 == "test" || $$3 == "tests") { total += $$2 } END { print total + 0 }' $$log); \
	   if [ $$ran -lt $(CORPUS_MIN_TESTS) ]; then \
	     echo "==> corpus: \`$$t\` executed $$ran tests, not $(CORPUS_MIN_TESTS) (gh #412)."; \
	     echo "==> corpus: libtest exits 0 when a filter selects nothing, so this would have"; \
	     echo "==> corpus: reported a green run over zero repositories."; \
	     grep -F 'test result:' $$log || echo "==> corpus: libtest printed no summary at all."; \
	     fail=1; \
	   fi; \
	 done; \
	 if [ $$fail -ne 0 ]; then echo "==> corpus: NEW failures (see [NEW] lines above)"; fi; \
	 exit $$fail

## test-corpus-ci-full: `make test-corpus-ci` over the WHOLE corpus — every
##                      pinned repository and every test in the suite. Needs
##                      >16 GB (#166) and about 20 minutes. This is what the
##                      corpus workflow's `full` dispatch runs, so the names
##                      live here and the workflow names none of its own.
test-corpus-ci-full: CORPUS_REPOS = $(CORPUS_REPOS_FULL)
test-corpus-ci-full: CORPUS_TESTS = $(CORPUS_TESTS_FULL)
test-corpus-ci-full: test-corpus-ci

# Where the recipe above parks libtest's own list of the names it will answer
# to, so a selector that resolves to nothing fails loudly instead of passing
# green over zero repositories.
CORPUS_NAMES = target/corpus-test-names.txt

# One log per selected test, kept so the recipe can count what libtest actually
# ran. The name check above is a pre-flight — it catches the spelling that was
# wrong, and nothing else. `--exact` selecting nothing is only one way to run
# zero tests; libtest exits 0 for every one of them. This is the check that
# holds regardless of cause: a corpus run must execute at least one test, or it
# has proved nothing and must not report green (gh #412).
CORPUS_LOGS = target/corpus-logs
CORPUS_MIN_TESTS = 1

# Scheduled CI runs a deliberately small slice: clone + scan inside ~1 minute.
# `tokio` is the fastest corpus and the only one that has ever been stable
# across runs, so it is the control; `nest` is the cheapest repository that
# still reproduces the determinism defect (#301).
#
# Precision defects (#331 Dart, #336 F#) are NOT covered here — those repos
# peak above 13 GB (#166) and take minutes to scan. Run the full suite with
# `make test-corpus` locally, or dispatch the workflow with `full`.
#
# [CORPUS-CI] These are the names libtest answers to, matched with `--exact`:
# bare, because `crates/deslop/Cargo.toml` gives the corpus suite its own
# `[[test]]` target, which makes that file a crate root rather than a module.
# They were written `corpus_repos::<test>` — a module path from a layout that
# no longer exists — and every scheduled run selected nothing and reported
# green (gh #412 again). `crates/deslop/tests/corpus_selection_contract.rs`
# is what now holds these names to tests that exist.
CORPUS_REPOS ?= tokio nest
CORPUS_TESTS ?= corpus_tokio_rust corpus_nest_typescript corpus_determinism_nest_typescript

# The whole corpus, for `test-corpus-ci-full`. A second copy of these lists in
# the workflow YAML is how the `full` dispatch came to pass the substring
# `corpus_` into an `--exact` loop and scan nothing at all.
CORPUS_REPOS_FULL = flutter jellyfin tokio django react nest laravel hugo fsharp tornado
CORPUS_TESTS_FULL = corpus_flutter_dart corpus_jellyfin_csharp corpus_tokio_rust \
                    corpus_django_python corpus_react_javascript corpus_nest_typescript \
                    corpus_laravel_php corpus_hugo_go corpus_fsharp corpus_tornado_python \
                    corpus_determinism_nest_typescript corpus_determinism_jellyfin_csharp

# [CI-DESLOP] Self-hosted duplication gate. Runs the release binary built by
#   `build` against this repo, so the gate is always the CURRENT detector, never
#   a released or PATH-installed one. Reads `[threshold] max_duplication_percent`
#   from `.deslop.toml` — the single source of truth — and exits 3 when repo-wide
#   duplication climbs past it. `make ci` runs this, so a green local run means a
#   green gate in CI; the workflow calls this same target rather than repeating
#   the command, so the two can never drift.
## dup-gate: Fail when this repo's own duplication exceeds .deslop.toml.
dup-gate: build
	@echo "==> Duplication gate (.deslop.toml [threshold])..."
	./target/release/deslop . --no-color

# [CORPUS-SCORE] Accuracy gate. Scans the register-backed target repositories
#   with the CURRENT build and scores every judged pair against the independent
#   clone registers in `corpus/register/`, failing when a CLEARLY IN goes
#   unreported or a CLEARLY OUT gets reported past the thresholds in
#   `corpus/register/score-thresholds.json` — the single source of truth, never
#   hardcoded here or in CI. Cluster totals and duplication percentages are
#   printed as description and gate nothing. The workflow calls the same script,
#   so a green local run means a green gate in CI.
## score-gate: Fail when accuracy drops against the judged clone registers.
##             This is the corpus run CI performs, one command, no arguments.
score-gate:
	./scripts/corpus/score-gate.sh

# [CORPUS-SCORE] The two-engine comparison, one command and no arguments: the
#   last released commit against the current HEAD, each engine rebuilt from a
#   clean tree, both scored by one scorer built from the working tree. Scans
#   every judged register rather than the CI slice — nobody waits on this one,
#   and the whole point is the widest comparable measurement available.
## compare: Compare the last release against HEAD across the whole register corpus.
compare:
	./scripts/compare-versions.sh

# [CORPUS-REGISTER-WORKSPACE] The folder a clone judge is handed: repositories,
#   reports and the judging skill, one workspace per repository the last
#   comparison scanned. It is built OUTSIDE this repository on purpose — a judge
#   who can read this source is contaminated and every verdict from that pass is
#   void. Override the location with JUDGING_DIR.
JUDGING_DIR ?= $(HOME)/clone-judging
## judging-folder: Build the blinded folder a clone judge works through.
judging-folder:
	./scripts/corpus/prepare-judging.sh $(JUDGING_DIR)

## merge-verdicts: Fold judged verdicts back into the clone registers. Takes
##                 two or more judging folders. Imports ONLY the pairs every
##                 source agrees on — the registers included; everything else
##                 is left out and listed in docs/reports/verdict-merge.md.
merge-verdicts:
	@test -n "$(JUDGED_DIRS)" || { echo "set JUDGED_DIRS to two or more judging folders"; exit 1; }
	@node scripts/corpus/merge-verdicts.mjs $(JUDGED_DIRS)

# [DEPLOY-CI-GATES] CI/release deployment-drift gate: manifest schema, binary
#   version contracts, release-workflow gates, and the verifier proof suite.
## deployment-verify: Validate deployment manifest and built binary contracts.
##                    Also runs the verifier proof suite which builds fake
##                    binaries and plugin zips violating each Shipwright
##                    contract rule and asserts every verifier rejects them.
##                    Without this, a silently-broken verifier could let a
##                    drifted binary ship. The action diff-gate proof runs the
##                    action's own step body against the freshly built CLI in
##                    both gate directions — the self-test's runner leg cannot,
##                    since it installs a published release ([ACTION-GATE]).
deployment-verify: build
	node scripts/deployment/verify-deployment-manifest.mjs shipwright.json
	node scripts/deployment/verify-deployment-binaries.mjs shipwright.json target/release
	node scripts/release/verify-release-workflow-gates.mjs .github/workflows/release.yml
	node scripts/release/test-release-workflow-contract.mjs
	node scripts/release/test-release-publish-contract.mjs
	node scripts/deployment/test-deployment-docs-contract.mjs
	node scripts/release/test-release-version-stamping.mjs
	node scripts/deployment/test-verifiers.mjs
	node scripts/actions/test-action-contract.mjs
	node scripts/actions/test-action-diff-gate.mjs

# _kill-deslop-processes: Terminate every running `deslop`, `deslop-lsp`, and
#   `deslop-mcp` so a stale child from a previous VSCode/Cursor session can't
#   shadow the freshly-installed VSIX bundle, socket-bound integration tests
#   don't get starved by a runaway analyser on another workspace, and — on
#   Windows, where a running image cannot be deleted — `cargo clean` can empty
#   `target/`. The matching, the two-phase kill, and the fail-closed re-check
#   live in the script so they can be tested without this target's destructive
#   side effect. Idempotent. Invoked by the rebuild targets; `test`/`vsix-*`
#   scrub via `_delete-path-binaries`.
_kill-deslop-processes:
	@bash scripts/repository/kill-deslop-processes.sh

# [DEPLOY-EXTERNAL-MCP-CONSUMER] No install-binary target by design; this scrub
#   keeps external MCP clients on the VSIX-bundled binary by absolute path.
# _delete-path-binaries: Remove any Deslop binaries that have leaked onto the
#   user's PATH (e.g. from `cargo install` or a package-manager install), and
#   fail if one survives. Invoked by every `_vsix-*` and `test` target so a
#   developer machine that previously installed Deslop is auto-scrubbed. The
#   detection, the deletion, and the fail-closed re-check live in the script so
#   they can be tested against a fixture PATH — running this target is
#   destructive by design, so it is never what the test drives (#474).
_delete-path-binaries:
	@bash scripts/repository/scrub-path-binaries.sh

# _vsix-install: Install Node deps for clients/vscode + webview-ui.
_vsix-install:
	cd clients/vscode && npm install --no-audit --no-fund
	cd clients/vscode/webview-ui && npm install --no-audit --no-fund

# _vsix-node-modules: The extension's dependencies, materialised only when
#                     they are missing. `lint` reaches this through
#                     `_ci-contract-tests`, so it must not change anything a
#                     reader would have to review: `npm ci` installs exactly
#                     what `package-lock.json` names and never writes it back,
#                     where `npm install` may resolve a newer tree and commit
#                     that decision to the lockfile. A gate that can silently
#                     move a dependency version is not a gate. Present means
#                     present — a deliberate refresh is `_vsix-install`, and
#                     the webview's own dependencies belong to the VSIX build,
#                     not to these script gates.
_vsix-node-modules:
	@test -d clients/vscode/node_modules \
	  || (cd clients/vscode && npm ci --no-audit --no-fund)

# _vsix-build: Build deslop-lsp + deslop-mcp + VSIX bundle + webview UI.
#   Depends on `_vsix-install` so a cold CI checkout has the webview-ui +
#   extension Node deps needed for esbuild bundling, and on `typediagram-gen`
#   so the gitignored wire-generated.ts exists before tsc runs.
_vsix-build: _vsix-install typediagram-gen
	cargo build --release -p deslop-lsp -p deslop-mcp -p deslop
	cd clients/vscode/webview-ui && npm run typecheck && npm run build
	cd clients/vscode && npm run build

# _vsix-test: Run VS Code E2E tests against bundled extension binaries only.
_vsix-test: _delete-path-binaries _vsix-install _vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm test

# _vsix-test-ollama: Run the Ollama-gated VSIX e2e suite (csharp-type4 fixture,
#   provider=ollama, model=nomic-embed-text). NEVER runs in `make ci` /
#   `make _vsix-test`. Requires a local Ollama daemon and the model pulled.
#   Reached through the public `make test-ollama` umbrella.
_vsix-test-ollama: _delete-path-binaries _vsix-install _vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm run test:ollama

# _vsix-coverage: Run the VS Code suite (extension host) and measure it. The
#   desktop host writes no V8 profile for extension code (gh #440), so the
#   counters are compiled into the modules and dumped from inside the host —
#   see clients/vscode/scripts/extension-coverage.mjs. One suite execution
#   yields both the pass/fail result and the coverage summary. The webview leg
#   is measured by _vsix-webview-coverage below.
_vsix-coverage: _delete-path-binaries _vsix-install _vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm run coverage:extension

# _vsix-playwright-html: Render the standalone HTML report from a fixture repo
#   with the real deslop CLI, then assert in a headless browser (Playwright)
#   that the design-system CSS actually applies — dark theme, layout,
#   cluster-card accent, and syntax colours ([OUTPUT-HUMAN-HTML]). Builds only
#   the deslop CLI the renderer needs and fetches the Chromium headless shell
#   with its OS libraries (--with-deps is idempotent, a no-op once cached, and a
#   no-op for system libs on macOS) so `make ci` reproduces CI exactly.
_vsix-playwright-html: _vsix-install
	cargo build --release -p deslop
	cd clients/vscode && npx playwright install --with-deps chromium && npm run test:playwright:html

# _vsix-webview-coverage: Drive the webview bundle in a real browser (Playwright)
#   with V8 coverage on and map executed ranges back to webview-ui/src. Threshold
#   calculation is deferred to _vsix-coverage-check.
#   The webview is invisible to the vscode-test c8 pass (extension host only);
#   this closes that blind spot (#254). The script rebuilds the production
#   bundle in a finally, so a coverage build is never left staged for packaging
#   — even if the Playwright run or mapping fails.
_vsix-webview-coverage: _vsix-install
	cd clients/vscode && npx playwright install --with-deps chromium && npm run coverage:webview

_vsix-coverage-check:
	cd clients/vscode && npm run coverage:extension:check
	cd clients/vscode && npm run coverage:webview:check

## vsix-package: Build the .vsix artifact (does not publish).
##               Stages the host-platform deslop-lsp + deslop-mcp + deslop
##               binaries into clients/vscode/bin/<platform>/ and produces a
##               platform-specific VSIX via `vsce package --target`
##               ([VSIX-BINARY-VERSIONING]). CI stages every supported platform;
##               locally we only have the host toolchain so we only stage that one.
vsix-package: _delete-path-binaries _vsix-install _vsix-build _vsix-stage-and-package

# [DEPLOY-EXTENSION-BUNDLED-TESTS] Stage binaries into the extension bundle so
#   VSIX tests run against bundled binaries, never a PATH-visible build.
_vsix-stage-bundled-binaries:
	@_uname_s=$$(uname -s); _uname_m=$$(uname -m); \
	 case "$$_uname_s-$$_uname_m" in \
	   Darwin-arm64)   _platform=darwin-arm64 ;; \
	   Darwin-x86_64)  _platform=darwin-x64 ;; \
	   Linux-x86_64)   _platform=linux-x64 ;; \
	   Linux-aarch64)  _platform=linux-arm64 ;; \
	   MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) _platform=win32-x64 ;; \
	   *) echo "FAIL: unsupported host $$_uname_s-$$_uname_m for vsix-package"; exit 1 ;; \
	 esac; \
	 case "$$_platform" in win32-*) _ext=.exe ;; *) _ext= ;; esac; \
	 _dest=clients/vscode/bin/$$_platform; \
	 echo "==> Staging bundled binaries into $$_dest"; \
	 $(RM) clients/vscode/bin; $(MKDIR) "$$_dest"; \
	 for _bin in deslop-lsp deslop-mcp deslop; do \
	   _src=target/release/$$_bin$$_ext; \
	   if [ ! -f "$$_src" ]; then echo "FAIL: $$_src missing (_vsix-build should have produced it)"; exit 1; fi; \
	   cp "$$_src" "$$_dest/$$_bin$$_ext"; \
	   chmod +x "$$_dest/$$_bin$$_ext"; \
	 done
	cp shipwright.json clients/vscode/shipwright.json

_vsix-stage-and-package: _vsix-stage-bundled-binaries
	cd clients/vscode && npm run package

# _vsix-clean: Remove VSIX-specific build artifacts (staged bin/, node_modules,
#   dist/, out/, packaged .vsix, coverage). Does NOT touch cargo's target/ —
#   chain `make clean` for that.
_vsix-clean:
	@echo "==> Cleaning VSIX build artifacts..."
	$(RM) clients/vscode/bin
	$(RM) clients/vscode/node_modules
	$(RM) clients/vscode/webview-ui/node_modules
	$(RM) clients/vscode/out
	$(RM) clients/vscode/dist
	$(RM) clients/vscode/deslop-live.vsix
	$(RM) clients/vscode/deslop-live-*.vsix
	$(RM) clients/vscode/shipwright.json
	$(RM) clients/vscode/coverage

# _vsix-install-code: Replace the installed Deslop.live extension with the
#   freshly packaged clients/vscode/deslop-live.vsix. Stale Marketplace folders
#   must be removed first; VS Code otherwise keeps loading the higher released
#   version even after `code --install-extension --force` reports success.
#   Skips with a warning if `code` isn't on PATH.
_vsix-install-code:
	@if command -v code >/dev/null 2>&1; then \
	  _vsix=$$(ls clients/vscode/deslop-live-*.vsix 2>/dev/null | head -n1); \
	  if [ -z "$$_vsix" ]; then echo "FAIL: no clients/vscode/deslop-live-*.vsix found"; exit 1; fi; \
	  echo "==> Removing installed Deslop.live VS Code extension copies..."; \
	  code --uninstall-extension nimblesite.deslop-live --force >/dev/null 2>&1 || true; \
	  for _extensions in "$(HOME)/.vscode/extensions" "$(HOME)/.vscode-insiders/extensions"; do \
	    if [ -d "$$_extensions" ]; then \
	      find "$$_extensions" -maxdepth 1 -type d -name 'nimblesite.deslop-live*' -print -exec rm -rf {} +; \
	    fi; \
	  done; \
	  echo "==> Installing $$_vsix into the VS Code CLI..."; \
	  code --install-extension "$$_vsix" --force; \
	else \
	  echo "WARN: 'code' CLI not on PATH — skipping install. VSIX is at clients/vscode/deslop-live-<target>.vsix"; \
	fi

## vsix-rebuild: Nuke every build artifact (cargo target/, staged bin/, node_modules,
##               dist/, out/, old .vsix), rebuild the workspace + webview + extension
##               from scratch, repackage the .vsix, install it into the local `code`
##               CLI, and scrub any cargo-installed PATH copies so the installed VSIX
##               bundle is the only source of truth. Use when "why isn't my change
##               showing up" strikes. Composes existing targets — does not duplicate
##               their logic.
vsix-rebuild:
	@$(MAKE) _kill-deslop-processes
	@$(MAKE) clean
	@$(MAKE) _vsix-clean
	@$(MAKE) vsix-package
	@$(MAKE) _vsix-install-code
	@$(MAKE) _delete-path-binaries
	@echo "==> vsix-rebuild done. Reload the VS Code window to pick up the new extension."
	@echo "    PATH copies removed — the VSIX bundle is now the only source of truth."

# _jetbrains-build: Build the JetBrains plugin zip (single LSP4IJ artifact, all IDE
#   families). LSP4IJ reaches Android Studio, IntelliJ Community, and Rider/Ultimate
#   from one build, so there is no separate native-LSP artifact.
_jetbrains-build:
	$(RM) $(_JETBRAINS_DIR)/deslop-lsp4ij/build/distributions/*.zip
	cargo build --release -p deslop-lsp
	cd $(_JETBRAINS_DIR) && $(GRADLE) :deslop-lsp4ij:buildPlugin

# _jetbrains-verify: Verify JetBrains plugin project and archive structure.
_jetbrains-verify:
	cd $(_JETBRAINS_DIR) && $(GRADLE) verifyPluginProjectConfiguration verifyPluginStructure

# _jetbrains-package: CI/release packaging gate — build the JetBrains plugin zip
#   (single LSP4IJ artifact), verify project/structure, and assert the packaged
#   artifact via scripts/deployment/verify-jetbrains-package.mjs. Headless (no IDE install);
#   invoked by .github/workflows/ci.yml. Local devs use android-studio-rebuild or
#   android-studio-rebuild-reinstall to actually load the plugin into the IDE.
_jetbrains-package: _jetbrains-build
	@$(MAKE) _jetbrains-verify
	node scripts/deployment/verify-jetbrains-package.mjs

# _jetbrains-test: Run the JetBrains tests via the wrapper — the shared-module
#   resolver/descriptor/panel tests plus the LSP4IJ surface's reactive-wiring tests.
_jetbrains-test:
	cd $(_JETBRAINS_DIR) && $(GRADLE) :deslop-shared:test :deslop-lsp4ij:test --no-daemon

# _jetbrains-real-binary-test: Run the resolver tests AND the real-binary
#   contract test, which copies target/release/deslop-lsp into a synthetic
#   plugin root and proves the resolver accepts it AND rejects manifest drift.
#   Requires a release build of deslop-lsp.
_jetbrains-real-binary-test:
	cargo build --release -p deslop-lsp
	cd $(_JETBRAINS_DIR) && DESLOP_LSP_REAL_BINARY="$(CURDIR)/target/release/deslop-lsp" \
	  $(GRADLE) :deslop-shared:test --no-daemon --rerun-tasks

## android-studio-rebuild: Rebuild the Android Studio (LSP4IJ) plugin and install
##                         it (macOS). Kills stale Deslop processes, builds the
##                         plugin zip via _jetbrains-build (host deslop-lsp
##                         bundled), and installs it into Android Studio with its
##                         required LSP4IJ dependency. For a full clean + uninstall
##                         first, use android-studio-rebuild-reinstall.
android-studio-rebuild:
	@$(MAKE) _kill-deslop-processes
	@$(MAKE) _jetbrains-build
	@$(MAKE) _android-studio-install
	@echo "==> Restart Android Studio to load the rebuilt plugin."

## android-studio-rebuild-reinstall: Full clean + uninstall + rebuild + reinstall
##                         of the Android Studio (LSP4IJ) plugin — the JetBrains
##                         analogue of vsix-rebuild (macOS). Kills stale Deslop
##                         processes, cleans every build artifact, uninstalls the
##                         currently installed plugin from Android Studio, then
##                         rebuilds and reinstalls it via android-studio-rebuild.
##                         Use when "why isn't my change showing up" strikes.
android-studio-rebuild-reinstall:
	@$(MAKE) _kill-deslop-processes
	@$(MAKE) clean
	@$(MAKE) _android-studio-uninstall
	@$(MAKE) android-studio-rebuild

# _android-studio-install: Install the freshly built plugin into the newest
#   Android Studio config on this Mac, plus its LSP4IJ dependency (pinned to the
#   version deslop-lsp4ij/build.gradle.kts builds against — keep the two in
#   sync). Without LSP4IJ, Android Studio disables Deslop. Warns (does not fail)
#   when Android Studio has never been launched here.
_android-studio-install:
	@_zip=$$(ls $(_JETBRAINS_DIR)/deslop-lsp4ij/build/distributions/deslop-lsp4ij-*.zip 2>/dev/null | head -n1); \
	 if [ -z "$$_zip" ]; then echo "FAIL: no deslop-lsp4ij-*.zip found (the build step failed)"; exit 1; fi; \
	 _cfg=$$(ls -d "$(HOME)/Library/Application Support/Google/AndroidStudio"* 2>/dev/null | sort | tail -n1); \
	 if [ -z "$$_cfg" ]; then \
	   echo "WARN: no Android Studio config dir under ~/Library/Application Support/Google/."; \
	   echo "      Launch Android Studio once, or install from disk:"; \
	   echo "      Settings -> Plugins -> gear -> Install Plugin from Disk -> $$_zip"; \
	   exit 0; \
	 fi; \
	 _plugins="$$_cfg/plugins"; $(MKDIR) "$$_plugins"; \
	 if [ ! -d "$$_plugins/lsp4ij" ]; then \
	   echo "==> Installing the LSP4IJ 0.20.1 dependency from the Marketplace"; \
	   _tmp=$$(mktemp -d); \
	   if curl -fsSL -o "$$_tmp/lsp4ij.zip" "https://plugins.jetbrains.com/plugin/download?pluginId=com.redhat.devtools.lsp4ij&version=0.20.1"; then \
	     unzip -q -o "$$_tmp/lsp4ij.zip" -d "$$_plugins"; \
	   else \
	     echo "    WARN: LSP4IJ download failed — install it from the Marketplace manually."; \
	   fi; \
	   $(RM) "$$_tmp"; \
	 fi; \
	 echo "==> Installing $$(basename "$$_zip") into $$_plugins"; \
	 $(RM) "$$_plugins/deslop-lsp4ij"; \
	 unzip -q -o "$$_zip" -d "$$_plugins"; \
	 echo "    Installed into $$(basename "$$_cfg") with its LSP4IJ dependency."

# _android-studio-uninstall: Remove the installed deslop-lsp4ij plugin from the
#   newest Android Studio config on this Mac. Warns (does not fail) when Android
#   Studio has never been launched here or the plugin isn't installed. Leaves the
#   shared LSP4IJ dependency in place — it is a Marketplace plugin, not ours.
_android-studio-uninstall:
	@_cfg=$$(ls -d "$(HOME)/Library/Application Support/Google/AndroidStudio"* 2>/dev/null | sort | tail -n1); \
	 if [ -z "$$_cfg" ]; then \
	   echo "WARN: no Android Studio config dir - nothing to uninstall."; exit 0; \
	 fi; \
	 _plugin="$$_cfg/plugins/deslop-lsp4ij"; \
	 if [ -d "$$_plugin" ]; then \
	   echo "==> Uninstalling deslop-lsp4ij from $$(basename "$$_cfg")"; \
	   $(RM) "$$_plugin"; \
	 else \
	   echo "    (deslop-lsp4ij not installed in $$(basename "$$_cfg") - nothing to remove)"; \
	 fi

## help: List all available targets
help:
	@echo "Standard targets:"
	@echo "  build          - Compile/assemble all artifacts"
	@echo "  test           - Fail-fast Rust tests + per-crate coverage threshold"
	@echo "  lint           - All linters/analyzers (read-only, no formatting)"
	@echo "  fmt            - Format all code in-place (CHECK=1 for read-only CI check)"
	@echo "  clean          - Remove build artifacts"
	@echo "  ci             - fmt + lint + rust test + build + deployment-verify + VSIX coverage + HTML-report CSS"
	@echo "  setup          - Post-create dev environment setup"
	@echo ""
	@echo "Repo-specific targets:"
	@echo "  typediagram-gen        - Regenerate wire-format IPC models from docs/models/*.td"
	@echo "  deployment-verify      - Validate deployment manifest and built binary contracts"
	@echo "  test-ollama            - Ollama-gated Rust + VSIX tests (never in CI)"
	@echo "  score-gate             - Score this build against the judged clone registers (what CI runs)"
	@echo "  compare                - Compare the last release against HEAD across every register"
	@echo "  judging-folder         - Build the blinded repos+reports+skill folder for a clone judge"
	@echo "  merge-verdicts         - Import verdicts every source agrees on (JUDGED_DIRS=\"a b\")"
	@echo "  test-corpus            - Accuracy + resource gate against pinned real repositories"
	@echo "  test-corpus-ci         - test-corpus in baseline mode (reports tracked defects)"
	@echo "  ci-ollama              - make ci plus make test-ollama"
	@echo "  vsix-package           - Build the platform-specific .vsix artifact + deployment gate"
	@echo "  vsix-rebuild           - Nuke + rebuild + repackage + install the VSIX from scratch"
	@echo "  android-studio-rebuild - Rebuild + install the Android Studio (LSP4IJ) plugin (macOS)"
	@echo "  android-studio-rebuild-reinstall - Clean + uninstall + rebuild + reinstall the plugin (macOS)"
	@echo ""
	@echo "Internal '_'-prefixed targets (CI steps / plumbing) are hidden; read the Makefile for them."
