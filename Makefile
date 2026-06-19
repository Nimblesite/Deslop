# agent-pmo:b636503
# =============================================================================
# Standard Makefile — Deslop
# Cross-platform: Linux, macOS, Windows (via GNU Make)
# Rust CLI. See docs/specs/SPEC.md and docs/plans/PLAN.md.
# =============================================================================

.PHONY: build test test-ollama lint fmt clean ci ci-ollama setup help build-release delete-path-binaries kill-deslop-processes deployment-verify vsix-install vsix-build vsix-test vsix-test-ollama vsix-coverage vsix-playwright-html vsix-package vsix-rebuild _vsix-stage-bundled-binaries _vsix-stage-and-package jetbrains-build jetbrains-verify jetbrains-package jetbrains-test jetbrains-real-binary-test typediagram-gen

JETBRAINS_DIR := clients/jetbrains

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
COVERAGE_THRESHOLDS_FILE := coverage-thresholds.json

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
	node scripts/typediagram-gen.mjs

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
test: delete-path-binaries typediagram-gen
	@echo "==> Testing (fail-fast + coverage + per-crate threshold)..."
	rustup component add llvm-tools-preview 2>/dev/null || true
	@_rust_ignore=$$(jq -r '.rust.ignore_filename_regex' "$(COVERAGE_THRESHOLDS_FILE)"); \
	 cargo llvm-cov --workspace --all-targets --features deslop-core/live \
	    --ignore-filename-regex "$$_rust_ignore" \
	    --lcov --output-path lcov.info -- --skip ollama_
	@$(MAKE) _coverage_check RUST_LCOV=lcov.info

_coverage_check:
	@_lcov="$${RUST_LCOV:-lcov.info}"; \
	 if [ ! -f "$$_lcov" ]; then echo "FAIL: $$_lcov not found"; exit 1; fi; \
	 if [ ! -f "$(COVERAGE_THRESHOLDS_FILE)" ]; then echo "FAIL: $(COVERAGE_THRESHOLDS_FILE) not found"; exit 1; fi; \
	 _default=$$(jq -r '.rust.default_threshold' "$(COVERAGE_THRESHOLDS_FILE)"); \
	 if [ "$$_default" = "null" ] || [ -z "$$_default" ]; then \
	   echo "FAIL: $(COVERAGE_THRESHOLDS_FILE) missing .rust.default_threshold"; exit 1; \
	 fi; \
	 _failed=0; \
	 for _crate in deslop-core deslop deslop-lsp deslop-mcp; do \
	   _threshold=$$(jq -r ".rust.crates.\"$$_crate\" // .rust.default_threshold" "$(COVERAGE_THRESHOLDS_FILE)"); \
	   if [ "$$_threshold" = "null" ] || [ -z "$$_threshold" ]; then \
	     echo "FAIL: no threshold for crate $$_crate in $(COVERAGE_THRESHOLDS_FILE)"; \
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
	@bash scripts/taxonomy-gate.sh
	@echo "==> VSIX stub-provider packaging gate (unit)..."
	@node --test clients/vscode/scripts/stub-gate.test.mjs

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
##     HTML-report CSS (Playwright). Full CI simulation. Runs every non-Ollama
##     test suite, Rust and VSIX, and enforces per-crate + VSIX coverage
##     thresholds. Ollama-gated suites run via `make ci-ollama`.
ci:
	@$(MAKE) fmt CHECK=1
	@$(MAKE) lint
	@$(MAKE) test
	@$(MAKE) build
	@$(MAKE) deployment-verify
	@$(MAKE) vsix-coverage
	@$(MAKE) vsix-playwright-html

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
##              `make test`/`make vsix-test`/`make ci` filter out.
##              Requires a local Ollama daemon on 127.0.0.1:11434 with
##              `nomic-embed-text` already pulled.
test-ollama: vsix-test-ollama
	cargo test --release --workspace ollama_

## ci-ollama: `make ci` plus `make test-ollama`.
ci-ollama: ci test-ollama

## build-release: Build the release binary for the deslop CLI
build-release:
	@echo "==> Building release binary..."
	cargo build --release --package deslop

## kill-deslop-processes: SIGTERM (then SIGKILL on holdouts) every running
##                        `deslop-lsp` and `deslop-mcp` process so a stale
##                        child from a previous VSCode/Cursor session can't
##                        shadow the freshly-installed VSIX bundle, and so
##                        socket-bound integration tests don't get starved
##                        by a runaway analyser on another workspace.
##                        Matches by process name (not full cmdline) so it
##                        will not accidentally kill `cargo build -p deslop-lsp`
##                        or similar parent commands. Idempotent — exits 0
##                        when no matching process exists. Invoked by every
##                        `vsix-*` and `test` target via `delete-path-binaries`.
kill-deslop-processes:
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

## delete-path-binaries: Remove any Deslop binaries that have leaked onto the
##                       user's PATH (e.g. from a stray `cargo install`). The
##                       VSIX is the only legitimate distribution surface — the
##                       VS Code extension, Claude Code MCP, Codex MCP, and any
##                       other host MUST resolve `deslop`, `deslop-lsp`, and
##                       `deslop-mcp` from the unpacked VSIX `bin/<platform>/`
##                       directory by absolute path. PATH resolution would let
##                       a locally-built binary shadow the shipright-versioned
##                       bundle. This target is invoked by every `vsix-*` and
##                       `test` target so a developer machine that previously
##                       ran `cargo install` is automatically scrubbed.
delete-path-binaries:
	@echo "==> Removing cargo-installed Deslop binaries from PATH..."
	@for _bin in deslop deslop-lsp deslop-mcp; do \
	  cargo uninstall $$_bin 2>/dev/null || true; \
	  $(RM) "$(HOME)/.cargo/bin/$$_bin" "$(HOME)/.cargo/bin/$$_bin.exe"; \
	  _found=$$(command -v $$_bin 2>/dev/null || true); \
	  if [ -n "$$_found" ]; then \
	    echo "FAIL: $$_bin still resolves on PATH at $$_found"; \
	    echo "Remove it before running tests; extension tests must use bundled binaries by absolute path."; \
	    exit 1; \
	  fi; \
	done

## deployment-verify: Validate deployment manifest and built binary contracts.
##                    Also runs the verifier proof suite which builds fake
##                    binaries and plugin zips violating each Shipwright
##                    contract rule and asserts every verifier rejects them.
##                    Without this, a silently-broken verifier could let a
##                    drifted binary ship.
deployment-verify: build
	node scripts/verify-deployment-manifest.mjs shipwright.json
	node scripts/verify-deployment-binaries.mjs shipwright.json target/release
	node scripts/verify-release-workflow-gates.mjs .github/workflows/release.yml
	node scripts/test-release-workflow-contract.mjs
	node scripts/test-release-version-stamping.mjs
	node scripts/test-verifiers.mjs

## vsix-install: Install Node deps for clients/vscode + webview-ui
vsix-install:
	cd clients/vscode && npm install --no-audit --no-fund
	cd clients/vscode/webview-ui && npm install --no-audit --no-fund

## vsix-build: Build deslop-lsp + deslop-mcp + VSIX bundle + webview UI.
##             Depends on `vsix-install` so a cold CI checkout has the
##             webview-ui + extension Node deps needed for esbuild bundling,
##             and on `typediagram-gen` so the gitignored wire-generated.ts
##             exists before tsc runs.
vsix-build: vsix-install typediagram-gen
	cargo build --release -p deslop-lsp -p deslop-mcp -p deslop
	cd clients/vscode/webview-ui && npm run build
	cd clients/vscode && npm run build

## vsix-test: Run VS Code E2E tests against bundled extension binaries only.
vsix-test: delete-path-binaries vsix-install vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm test

## vsix-test-ollama: Run the Ollama-gated VSIX e2e suite (csharp-type4
##                   fixture, provider=ollama, model=nomic-embed-text).
##                   NEVER runs in `make ci` / `make vsix-test`. Requires
##                   a local Ollama daemon and the model pulled.
vsix-test-ollama: delete-path-binaries vsix-install vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm run test:ollama

## vsix-coverage: Run VS Code E2E + enforce the VSIX coverage threshold.
##                Threshold lives in the repo-root coverage-thresholds.json.
vsix-coverage: delete-path-binaries vsix-install vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm run coverage

## vsix-playwright-html: Render the standalone HTML report from a fixture repo
##                       with the real deslop CLI, then assert in a headless
##                       browser (Playwright) that the design-system CSS actually
##                       applies — dark theme, layout, cluster-card accent, and
##                       syntax colours ([OUTPUT-HUMAN-HTML]). Builds only the
##                       deslop CLI the renderer needs and fetches the Chromium
##                       headless shell with its OS libraries (--with-deps is
##                       idempotent, a no-op once cached, and a no-op for system
##                       libs on macOS) so `make ci` reproduces CI exactly.
vsix-playwright-html: vsix-install
	cargo build --release -p deslop
	cd clients/vscode && npx playwright install --with-deps chromium && npm run test:playwright:html

## vsix-package: Build the .vsix artifact (does not publish).
##               Stages the host-platform deslop-lsp + deslop-mcp + deslop
##               binaries into clients/vscode/bin/<platform>/ and produces a
##               platform-specific VSIX via `vsce package --target`
##               ([VSIX-BINARY-VERSIONING]). CI stages every supported platform;
##               locally we only have the host toolchain so we only stage that one.
vsix-package: delete-path-binaries vsix-install vsix-build _vsix-stage-and-package

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
	   if [ ! -f "$$_src" ]; then echo "FAIL: $$_src missing (vsix-build should have produced it)"; exit 1; fi; \
	   cp "$$_src" "$$_dest/$$_bin$$_ext"; \
	   chmod +x "$$_dest/$$_bin$$_ext"; \
	 done
	cp shipwright.json clients/vscode/shipwright.json

_vsix-stage-and-package: _vsix-stage-bundled-binaries
	cd clients/vscode && npm run package

## vsix-clean: Remove VSIX-specific build artifacts (staged bin/, node_modules,
##             dist/, out/, packaged .vsix, coverage). Does NOT touch cargo's
##             target/ — chain `make clean` for that.
vsix-clean:
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

## vsix-install-code: Install the packaged clients/vscode/deslop-live.vsix
##                    into the local `code` CLI. Skips with a warning if
##                    `code` isn't on PATH.
vsix-install-code:
	@if command -v code >/dev/null 2>&1; then \
	  _vsix=$$(ls clients/vscode/deslop-live-*.vsix 2>/dev/null | head -n1); \
	  if [ -z "$$_vsix" ]; then echo "FAIL: no clients/vscode/deslop-live-*.vsix found"; exit 1; fi; \
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
	@$(MAKE) kill-deslop-processes
	@$(MAKE) clean
	@$(MAKE) vsix-clean
	@$(MAKE) vsix-package
	@$(MAKE) vsix-install-code
	@$(MAKE) delete-path-binaries
	@echo "==> vsix-rebuild done. Reload the VS Code window to pick up the new extension."
	@echo "    PATH copies removed — the VSIX bundle is now the only source of truth."

## jetbrains-build: Build both JetBrains plugin zips (native-LSP + LSP4IJ).
jetbrains-build:
	$(RM) $(JETBRAINS_DIR)/deslop-ultimate/build/distributions/*.zip $(JETBRAINS_DIR)/deslop-lsp4ij/build/distributions/*.zip
	cargo build --release -p deslop-lsp
	cd $(JETBRAINS_DIR) && $(GRADLE) :deslop-ultimate:buildPlugin :deslop-lsp4ij:buildPlugin

## jetbrains-verify: Verify JetBrains plugin project and archive structure.
jetbrains-verify:
	cd $(JETBRAINS_DIR) && $(GRADLE) verifyPluginProjectConfiguration verifyPluginStructure

## jetbrains-package: Build and verify the JetBrains plugin package artifact.
jetbrains-package: jetbrains-build
	@$(MAKE) jetbrains-verify
	node scripts/verify-jetbrains-package.mjs

## jetbrains-test: Run the JetBrains shared-module tests via the wrapper.
jetbrains-test:
	cd $(JETBRAINS_DIR) && $(GRADLE) :deslop-shared:test --no-daemon

## jetbrains-real-binary-test: Run the resolver tests AND the real-binary
##                             contract test, which copies target/release/deslop-lsp
##                             into a synthetic plugin root and proves the
##                             resolver accepts it AND rejects manifest drift.
##                             Requires a release build of deslop-lsp.
jetbrains-real-binary-test:
	cargo build --release -p deslop-lsp
	cd $(JETBRAINS_DIR) && DESLOP_LSP_REAL_BINARY="$(CURDIR)/target/release/deslop-lsp" \
	  $(GRADLE) :deslop-shared:test --no-daemon --rerun-tasks

## help: List all available targets
help:
	@echo "Standard targets:"
	@echo "  build          - Compile/assemble all artifacts"
	@echo "  test           - Fail-fast Rust tests + per-crate coverage threshold"
	@echo "  lint           - All linters/analyzers (read-only, no formatting)"
	@echo "  fmt            - Format all code in-place (CHECK=1 for read-only CI check)"
	@echo "  clean          - Remove build artifacts"
	@echo "  deployment-verify - Validate deployment manifest and built binary contracts"
	@echo "  ci             - fmt + lint + rust test + build + VSIX coverage + HTML-report CSS (Playwright)"
	@echo "  setup          - Post-create dev environment setup"
	@echo ""
	@echo "Repo-specific targets:"
	@echo "  test-ollama    - Ollama-gated Rust + VSIX tests (never in CI)"
	@echo "  ci-ollama      - make ci plus make test-ollama"
	@echo "  build-release  - Build the release binary for the deslop CLI"
	@echo "  delete-path-binaries - Scrub Deslop binaries off PATH (VSIX bundle is canonical)"
	@echo "  vsix-install   - Install Node deps for clients/vscode + webview-ui"
	@echo "  vsix-build     - Build LSP + MCP + VSIX bundle + webview UI"
	@echo "  vsix-test      - Run VS Code E2E tests against the real LSP"
	@echo "  vsix-test-ollama - Ollama-gated VSIX e2e (type4 fixture, never in CI)"
	@echo "  vsix-coverage  - VS Code E2E + enforce coverage threshold"
	@echo "  vsix-playwright-html - Assert the standalone HTML report's CSS renders (Playwright)"
	@echo "  vsix-package   - Build the .vsix artifact"
	@echo "  vsix-rebuild   - Nuke + rebuild + repackage + install the VSIX from scratch"
	@echo "  jetbrains-build - Build the JetBrains plugin zip"
	@echo "  jetbrains-verify - Verify JetBrains plugin configuration and structure"
	@echo "  jetbrains-package - Build and verify the JetBrains plugin zip"
