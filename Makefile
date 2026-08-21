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

.PHONY: build dup-gate test test-ollama lint fmt clean ci ci-ollama setup help deployment-verify vsix-package vsix-rebuild android-studio-rebuild android-studio-rebuild-reinstall typediagram-gen _delete-path-binaries _kill-deslop-processes _vsix-install _vsix-build _vsix-test _vsix-test-ollama _vsix-coverage _vsix-webview-coverage _vsix-playwright-html _vsix-install-code _vsix-clean _vsix-stage-bundled-binaries _vsix-stage-and-package _jetbrains-build _jetbrains-verify _jetbrains-package _jetbrains-test _jetbrains-real-binary-test _android-studio-install _android-studio-uninstall

_JETBRAINS_DIR := clients/jetbrains

# ---------------------------------------------------------------------------
# OS Detection
# ---------------------------------------------------------------------------
ifeq ($(OS),Windows_NT)
  SHELL := powershell.exe
  .SHELLFLAGS := -NoProfile -Command
  RM = Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
  MKDIR = New-Item -ItemType Directory -Force
  HOME ?= $(USERPROFILE)
  # The JetBrains wrapper is checked in and is the source of truth.
  # Override `GRADLE=...` only when deliberately testing another runtime.
  GRADLE ?= ./gradlew.bat
else
  RM = rm -rf
  MKDIR = mkdir -p
  GRADLE ?= ./gradlew
endif

# ---------------------------------------------------------------------------
# Coverage — single source of truth is coverage-thresholds.json
# See docs/specs/SPEC.md and REPO-STANDARDS-SPEC [COVERAGE-THRESHOLDS-JSON].
# ---------------------------------------------------------------------------
_COVERAGE_THRESHOLDS_FILE := coverage-thresholds.json

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
##       Does NOT require Ollama — tests whose names contain `ollama_`
##       are filtered out via `--skip ollama_`. `make ci-ollama` runs
##       the Ollama-gated tests explicitly against a live daemon.
##       The `--ignore-filename-regex` list lives in
##       `coverage-thresholds.json` under `.rust.ignore_filename_regex`
##       (single source of truth). Per-crate thresholds live under
##       `.rust.crates.<crate>`; `_coverage_check` enforces each one
##       independently — no workspace roll-up masking.
test: _delete-path-binaries typediagram-gen
	@echo "==> Testing (fail-fast + coverage + per-crate threshold)..."
	rustup component add llvm-tools-preview 2>/dev/null || true
	@_rust_ignore=$$(jq -r '.rust.ignore_filename_regex' "$(_COVERAGE_THRESHOLDS_FILE)"); \
	 cargo llvm-cov --workspace --all-targets --features deslop-core/live \
	    --ignore-filename-regex "$$_rust_ignore" \
	    --lcov --output-path lcov.info -- --skip ollama_ --skip corpus_
	@$(MAKE) _coverage_check RUST_LCOV=lcov.info

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
##       bucket label.
##       Depends on typediagram-gen so the wire-generated module exists
##       before clippy parses the workspace on a fresh checkout.
lint: typediagram-gen
	@echo "==> Linting..."
	cargo clippy --release --all-targets --workspace -- -D warnings
	@bash scripts/repository/taxonomy-gate.sh
	@echo "==> VSIX harness + packaging script gates (unit)..."
	@node --test clients/vscode/scripts/*.test.mjs
	@echo "==> PATH/env injection gate ([ACTION-ENVPATH])..."
	@node --test scripts/actions/verify-env-path-writes.test.mjs
	@node scripts/actions/verify-env-path-writes.mjs
	@echo "==> Docs installer snippet fail-closed gate ([DEPLOY-DOCS-INSTALLER-FAILCLOSED])..."
	@node --test scripts/deployment/installer-snippet.test.mjs
	@echo "==> Duplication-gate provenance gate ([CI-DESLOP])..."
	@node --test scripts/repository/dup-gate-source.test.mjs

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

## ci: fmt + lint + Rust test + build + deployment-verify + VSIX coverage +
##     VSIX E2E + webview coverage + HTML-report CSS (Playwright). Full CI
##     simulation mirroring the .github/workflows/ci.yml vsix job. Runs every
##     non-Ollama test suite, Rust and VSIX, and enforces per-crate + VSIX +
##     webview coverage thresholds. `_vsix-coverage` runs the unit + E2E suite
##     under c8 ([VSIX-TESTING-COVERAGE]); `_vsix-test` re-runs the same E2E
##     against the packaged bundle without coverage. Ollama-gated suites run
##     via `make ci-ollama`.
ci:
	@$(MAKE) fmt CHECK=1
	@$(MAKE) lint
	@$(MAKE) test
	@$(MAKE) build
	@$(MAKE) dup-gate
	@$(MAKE) deployment-verify
	@$(MAKE) _vsix-coverage
	@$(MAKE) _vsix-test
	@$(MAKE) _vsix-webview-coverage
	@$(MAKE) _vsix-playwright-html

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

## test-ollama: Run every Ollama-gated test — Rust `ollama_*` tests and
##              the VSIX `.vscode-test-ollama.mjs` suite — that
##              `make test`/`make ci` filter out. Requires a local Ollama
##              daemon on 127.0.0.1:11434 with `nomic-embed-text` pulled.
test-ollama: _vsix-test-ollama
	cargo test --release --workspace ollama_

## ci-ollama: `make ci` plus `make test-ollama`.
ci-ollama: ci test-ollama

## test-corpus: [CORPUS-*] Accuracy + resource suite against real public repos
##              pinned by `corpus/*.json`. Clones into git-ignored `.corpus/`
##              first (re-runs are free once cloned). Excluded from
##              `make test`/`make ci` via `--skip corpus_` because it needs
##              the network and measures wall time and peak memory, which are
##              runner-dependent. Run it when touching the pipeline.
test-corpus:
	node scripts/corpus/fetch-corpus.mjs
	cargo build --release --bin deslop
	cargo test --release -p deslop --test corpus_repos -- --nocapture --test-threads=1

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
	@fail=0; for t in $(CORPUS_TESTS); do \
	   cargo test --release -p deslop --test corpus_repos $$t -- --nocapture --test-threads=1 || fail=1; \
	 done; \
	 if [ $$fail -ne 0 ]; then echo "==> corpus: NEW failures (see [NEW] lines above)"; fi; \
	 exit $$fail

# Scheduled CI runs a deliberately small slice: clone + scan inside ~1 minute.
# `tokio` is the fastest corpus and the only one that has ever been stable
# across runs, so it is the control; `nest` is the cheapest repository that
# still reproduces the determinism defect (#301).
#
# Precision defects (#331 Dart, #336 F#) are NOT covered here — those repos
# peak above 13 GB (#166) and take minutes to scan. Run the full suite with
# `make test-corpus` locally, or dispatch the workflow with `full`.
CORPUS_REPOS ?= tokio nest
CORPUS_TESTS ?= corpus_tokio_rust corpus_nest_typescript corpus_determinism_nest_typescript

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
	node scripts/deployment/test-deployment-docs-contract.mjs
	node scripts/release/test-release-version-stamping.mjs
	node scripts/deployment/test-verifiers.mjs
	node scripts/actions/test-action-contract.mjs
	node scripts/actions/test-action-diff-gate.mjs

# _kill-deslop-processes: SIGTERM (then SIGKILL on holdouts) every running
#   `deslop-lsp` and `deslop-mcp` process so a stale child from a previous
#   VSCode/Cursor session can't shadow the freshly-installed VSIX bundle, and so
#   socket-bound integration tests don't get starved by a runaway analyser on
#   another workspace. Matches by process name (not full cmdline) so it will not
#   accidentally kill `cargo build -p deslop-lsp` or similar parent commands.
#   Idempotent — exits 0 when no matching process exists. Invoked by the rebuild
#   targets; `test`/`vsix-*` scrub via `_delete-path-binaries`.
_kill-deslop-processes:
	@echo "==> Killing any running deslop-lsp / deslop-mcp processes..."
	@_initial_lsp=$$(pgrep -x deslop-lsp 2>/dev/null || true); \
	 _initial_mcp=$$(pgrep -x deslop-mcp 2>/dev/null || true); \
	 _initial="$$_initial_lsp $$_initial_mcp"; \
	 _initial=$$(echo "$$_initial" | tr ' ' '\n' | sort -u | grep -v '^$$' || true); \
	 if [ -z "$$_initial" ]; then echo "    (none running)"; exit 0; fi; \
	 echo "    initial PIDs: $$(echo $$_initial | tr '\n' ' ')"; \
	 pkill -x deslop-lsp 2>/dev/null || true; \
	 pkill -x deslop-mcp 2>/dev/null || true; \
	 sleep 1; \
	 _survivors=""; \
	 for _pid in $$_initial; do \
	   if kill -0 "$$_pid" 2>/dev/null; then _survivors="$$_survivors $$_pid"; fi; \
	 done; \
	 if [ -n "$$_survivors" ]; then \
	   echo "    SIGKILL on holdouts:$$_survivors"; \
	   for _pid in $$_survivors; do kill -9 "$$_pid" 2>/dev/null || true; done; \
	   sleep 1; \
	   _final=""; \
	   for _pid in $$_survivors; do \
	     if kill -0 "$$_pid" 2>/dev/null; then _final="$$_final $$_pid"; fi; \
	   done; \
	   if [ -n "$$_final" ]; then echo "FAIL: PIDs alive after SIGKILL:$$_final"; exit 1; fi; \
	 fi; \
	 echo "    all targeted processes are dead (VSCode may auto-respawn — that is fine)"

# [DEPLOY-EXTERNAL-MCP-CONSUMER] No install-binary target by design; this scrub
#   keeps external MCP clients on the VSIX-bundled binary by absolute path.
# _delete-path-binaries: Remove any Deslop binaries that have leaked onto the
#   user's PATH (e.g. from `cargo install` or a package-manager install). The
#   VSIX is the only legitimate distribution surface. The VS Code extension,
#   Claude Code MCP, Codex MCP, and any other host MUST resolve `deslop`,
#   `deslop-lsp`, and `deslop-mcp` from the unpacked VSIX `bin/<platform>/`
#   directory by absolute path. PATH resolution would let a locally-built binary
#   shadow the Shipwright-versioned bundle. Invoked by every `_vsix-*` and
#   `test` target so a developer machine that previously installed Deslop is
#   auto-scrubbed.
_delete-path-binaries:
	@echo "==> Removing Deslop binaries from PATH..."
	@if command -v brew >/dev/null 2>&1; then \
	  brew uninstall --force deslop >/dev/null 2>&1 || true; \
	fi
	@for _bin in deslop deslop-lsp deslop-mcp; do \
	  cargo uninstall $$_bin >/dev/null 2>&1 || true; \
	  $(RM) "$(HOME)/.cargo/bin/$$_bin" "$(HOME)/.cargo/bin/$$_bin.exe"; \
	  hash -r 2>/dev/null || true; \
	  _attempts=0; \
	  while _found=$$(command -v $$_bin 2>/dev/null || true); [ -n "$$_found" ]; do \
	    if [ "$$_attempts" -ge 10 ]; then \
	      echo "FAIL: $$_bin still resolves on PATH at $$_found"; \
	      echo "Remove it before running tests; extension tests must use bundled binaries by absolute path."; \
	      exit 1; \
	    fi; \
	    case "$$_found" in \
	      */*) echo "    deleting $$_bin at $$_found"; $(RM) "$$_found" ;; \
	      *) echo "FAIL: $$_bin resolved to non-file command $$_found"; exit 1 ;; \
	    esac; \
	    hash -r 2>/dev/null || true; \
	    _attempts=$$(( _attempts + 1 )); \
	  done; \
	done

# _vsix-install: Install Node deps for clients/vscode + webview-ui.
_vsix-install:
	cd clients/vscode && npm install --no-audit --no-fund
	cd clients/vscode/webview-ui && npm install --no-audit --no-fund

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

# _vsix-coverage: Run VS Code E2E + enforce the VSIX coverage threshold.
#   Threshold lives in the repo-root coverage-thresholds.json.
_vsix-coverage: _delete-path-binaries _vsix-install _vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm run coverage

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
#   with V8 coverage on, map executed ranges back to webview-ui/src via inline
#   sourcemaps, and enforce .vsix.webview_threshold from coverage-thresholds.json.
#   The webview is invisible to the vscode-test c8 pass (extension host only);
#   this closes that blind spot (#254). The script rebuilds the production
#   bundle in a finally, so a coverage build is never left staged for packaging
#   — even if the Playwright run or mapping fails.
_vsix-webview-coverage: _vsix-install
	cd clients/vscode && npx playwright install --with-deps chromium && npm run coverage:webview

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
	@echo "  test-corpus            - Accuracy + resource gate against pinned real repositories"
	@echo "  test-corpus-ci         - test-corpus in baseline mode (reports tracked defects)"
	@echo "  ci-ollama              - make ci plus make test-ollama"
	@echo "  vsix-package           - Build the platform-specific .vsix artifact + deployment gate"
	@echo "  vsix-rebuild           - Nuke + rebuild + repackage + install the VSIX from scratch"
	@echo "  android-studio-rebuild - Rebuild + install the Android Studio (LSP4IJ) plugin (macOS)"
	@echo "  android-studio-rebuild-reinstall - Clean + uninstall + rebuild + reinstall the plugin (macOS)"
	@echo ""
	@echo "Internal '_'-prefixed targets (CI steps / plumbing) are hidden; read the Makefile for them."
