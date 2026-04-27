# agent-pmo:9a71cbf
# =============================================================================
# Standard Makefile — Deslop
# Cross-platform: Linux, macOS, Windows (via GNU Make)
# Rust CLI. See docs/specs/SPEC.md and docs/plans/PLAN.md.
# =============================================================================

.PHONY: build test test-ollama lint fmt clean ci ci-ollama setup help build-release install-binary delete-path-binaries deployment-verify vsix-install vsix-build vsix-test vsix-test-ollama vsix-coverage vsix-package vsix-rebuild _vsix-stage-bundled-binaries _vsix-stage-and-package jetbrains-build jetbrains-verify jetbrains-package

GRADLE ?= gradle
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
else
  RM = rm -rf
  MKDIR = mkdir -p
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

## test: Fail-fast tests + coverage + per-crate threshold enforcement.
##       See REPO-STANDARDS-SPEC [TEST-RULES] and [COVERAGE-THRESHOLDS-JSON].
##       Does NOT require Ollama — tests whose names contain `ollama_`
##       are filtered out via `--skip ollama_`. `make ci-ollama` runs
##       the Ollama-gated tests explicitly against a live daemon.
##       The `--ignore-filename-regex` list lives in
##       `coverage-thresholds.json` under `.rust.ignore_filename_regex`
##       (single source of truth). Per-crate thresholds live under
##       `.rust.crates.<crate>`; `scripts/coverage-check.sh` enforces
##       each one independently — no workspace roll-up masking.
test: delete-path-binaries
	@echo "==> Testing (fail-fast + coverage + per-crate threshold)..."
	rustup component add llvm-tools-preview 2>/dev/null || true
	@_rust_ignore=$$(jq -r '.rust.ignore_filename_regex' "$(_COVERAGE_THRESHOLDS_FILE)"); \
	 cargo llvm-cov --workspace --all-targets --features deslop-core/live \
	    --ignore-filename-regex "$$_rust_ignore" \
	    --lcov --output-path lcov.info -- --skip ollama_
	@bash scripts/coverage-check.sh lcov.info "$(_COVERAGE_THRESHOLDS_FILE)"

## lint: Run all linters/analyzers (read-only). Does NOT format.
##       Also enforces the taxonomy content gate
##       ([CLONE-BUCKETS-DUAL-LABEL]): every product-facing `Type-N`
##       mention in site/src and examples must co-locate a canonical
##       bucket label.
lint:
	@echo "==> Linting..."
	cargo clippy --release --all-targets --workspace -- -D warnings
	@bash scripts/taxonomy-gate.sh

## fmt: Format all code in-place. Pass CHECK=1 for read-only check (CI use).
fmt:
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

## ci: fmt + lint + Rust test + build + VSIX e2e + VSIX coverage.
##     Full CI simulation. Runs every non-Ollama test suite, Rust and
##     VSIX, and enforces per-crate + VSIX coverage thresholds.
##     Ollama-gated suites run via `make ci-ollama`.
ci:
	@$(MAKE) fmt CHECK=1
	@$(MAKE) lint
	@$(MAKE) test
	@$(MAKE) build
	@$(MAKE) deployment-verify
	@$(MAKE) vsix-coverage

## setup: Post-create dev environment setup (used by devcontainer)
setup:
	@echo "==> Setting up development environment..."
	rustup component add llvm-tools-preview clippy rustfmt
	cargo install --locked cargo-llvm-cov
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

## install-binary: Clean, build release, and install all three binaries
##                 (deslop, deslop-lsp, deslop-mcp) onto the user's PATH.
##                 Deletes the installed binaries and runs `cargo clean` first
##                 so a stale build artifact can never shadow the source on disk.
install-binary:
	@for _bin in deslop deslop-lsp deslop-mcp; do \
	  echo "==> Removing previously installed $$_bin binary..."; \
	  cargo uninstall $$_bin 2>/dev/null || true; \
	  $(RM) "$(HOME)/.cargo/bin/$$_bin"; \
	done
	@echo "==> Cleaning build artifacts..."
	cargo clean --release --package deslop --package deslop-lsp --package deslop-mcp
	@echo "==> Building release binaries from clean state..."
	cargo build --release --package deslop --package deslop-lsp --package deslop-mcp
	@for _crate in deslop deslop-lsp deslop-mcp; do \
	  echo "==> Installing $$_crate binary..."; \
	  cargo install --locked --path crates/$$_crate --force; \
	done

## delete-path-binaries: Remove cargo-installed Deslop binaries before tests so
##                       extension tests cannot accidentally pass by resolving
##                       PATH instead of the extension bundle. VS Code extension
##                       directories in PATH are skipped — the resolver's bundled
##                       candidate (clients/vscode/bin/<platform>/) always wins
##                       because it is evaluated before the path candidate.
delete-path-binaries:
	@echo "==> Removing cargo-installed Deslop binaries from PATH..."
	@for _bin in deslop deslop-lsp deslop-mcp; do \
	  cargo uninstall $$_bin 2>/dev/null || true; \
	  $(RM) "$(HOME)/.cargo/bin/$$_bin" "$(HOME)/.cargo/bin/$$_bin.exe"; \
	  _found=$$(command -v $$_bin 2>/dev/null || true); \
	  if [ -n "$$_found" ]; then \
	    case "$$_found" in \
	      */.vscode/extensions/*|*/.vscode-server/extensions/*|*/.cursor/extensions/*) \
	        echo "SKIP: $$_bin at $$_found is a VS Code extension bundle — not a PATH install" ;; \
	      *) \
	        echo "FAIL: $$_bin still resolves on PATH at $$_found"; \
	        echo "Remove it before running tests; extension tests must use bundled binaries."; \
	        exit 1 ;; \
	    esac; \
	  fi; \
	done

## deployment-verify: Validate deployment manifest and built binary contracts.
deployment-verify: build
	node scripts/verify-deployment-manifest.mjs deployment-toolkit.json
	node scripts/verify-deployment-binaries.mjs deployment-toolkit.json target/release

## vsix-install: Install Node deps for clients/vscode + webview-ui
vsix-install:
	cd clients/vscode && npm install --no-audit --no-fund
	cd clients/vscode/webview-ui && npm install --no-audit --no-fund

## vsix-build: Build deslop-lsp + deslop-mcp + VSIX bundle + webview UI.
##             Depends on `vsix-install` so a cold CI checkout has the
##             webview-ui + extension Node deps needed for esbuild bundling.
vsix-build: vsix-install
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
##                Threshold lives in clients/vscode/coverage-thresholds.json.
vsix-coverage: delete-path-binaries vsix-install vsix-build _vsix-stage-bundled-binaries
	cd clients/vscode && npm run coverage

## vsix-package: Build the .vsix artifact (does not publish).
##               Stages the host-platform deslop-lsp + deslop-mcp + deslop
##               binaries into clients/vscode/bin/<platform>/ so the installed
##               extension can resolve them via the bundled path
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
	 $(RM) "$$_dest"; $(MKDIR) "$$_dest"; \
	 for _bin in deslop-lsp deslop-mcp deslop; do \
	   _src=target/release/$$_bin$$_ext; \
	   if [ ! -f "$$_src" ]; then echo "FAIL: $$_src missing (vsix-build should have produced it)"; exit 1; fi; \
	   cp "$$_src" "$$_dest/$$_bin$$_ext"; \
	   chmod +x "$$_dest/$$_bin$$_ext"; \
	 done
	cp deployment-toolkit.json clients/vscode/deployment-toolkit.json

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
	$(RM) clients/vscode/deslop-vscode.vsix
	$(RM) clients/vscode/deployment-toolkit.json
	$(RM) clients/vscode/coverage

## vsix-install-code: Install the packaged clients/vscode/deslop-vscode.vsix
##                    into the local `code` CLI. Skips with a warning if
##                    `code` isn't on PATH.
vsix-install-code:
	@if command -v code >/dev/null 2>&1; then \
	  echo "==> Installing clients/vscode/deslop-vscode.vsix into the VS Code CLI..."; \
	  code --install-extension clients/vscode/deslop-vscode.vsix --force; \
	else \
	  echo "WARN: 'code' CLI not on PATH — skipping install. VSIX is at clients/vscode/deslop-vscode.vsix"; \
	fi

## vsix-rebuild: Nuke every build artifact (cargo target/, staged bin/, node_modules,
##               dist/, out/, old .vsix), rebuild the workspace + webview + extension
##               from scratch, repackage the .vsix, install it into the local `code`
##               CLI, and scrub any cargo-installed PATH copies so the installed VSIX
##               bundle is the only source of truth. Use when "why isn't my change
##               showing up" strikes. Composes existing targets — does not duplicate
##               their logic.
vsix-rebuild:
	@$(MAKE) clean
	@$(MAKE) vsix-clean
	@$(MAKE) vsix-package
	@$(MAKE) vsix-install-code
	@$(MAKE) delete-path-binaries
	@echo "==> vsix-rebuild done. Reload the VS Code window to pick up the new extension."
	@echo "    PATH copies removed — the VSIX bundle is now the only source of truth."

## jetbrains-build: Build the JetBrains plugin zip.
##                 JetBrains archive verification is deferred to GitHub #55
##                 while the local Gradle validation path is tracked in #56.
jetbrains-build:
	cargo build --release -p deslop-lsp
	cd $(JETBRAINS_DIR) && $(GRADLE) buildPlugin

## jetbrains-verify: Verify JetBrains plugin project and archive structure.
jetbrains-verify:
	cd $(JETBRAINS_DIR) && $(GRADLE) verifyPluginProjectConfiguration verifyPluginStructure

## jetbrains-package: Alias for the JetBrains plugin package artifact.
jetbrains-package: jetbrains-build

## help: List all available targets
help:
	@echo "Standard targets:"
	@echo "  build          - Compile/assemble all artifacts"
	@echo "  test           - Fail-fast Rust tests + per-crate coverage threshold"
	@echo "  lint           - All linters/analyzers (read-only, no formatting)"
	@echo "  fmt            - Format all code in-place (CHECK=1 for read-only CI check)"
	@echo "  clean          - Remove build artifacts"
	@echo "  deployment-verify - Validate deployment manifest and built binary contracts"
	@echo "  ci             - fmt + lint + rust test + build + VSIX e2e + VSIX coverage"
	@echo "  setup          - Post-create dev environment setup"
	@echo ""
	@echo "Repo-specific targets:"
	@echo "  test-ollama    - Ollama-gated Rust + VSIX tests (never in CI)"
	@echo "  ci-ollama      - make ci plus make test-ollama"
	@echo "  build-release  - Build the release binary for the deslop CLI"
	@echo "  install-binary - Build release and install binary onto PATH"
	@echo "  vsix-install   - Install Node deps for clients/vscode + webview-ui"
	@echo "  vsix-build     - Build LSP + MCP + VSIX bundle + webview UI"
	@echo "  vsix-test      - Run VS Code E2E tests against the real LSP"
	@echo "  vsix-test-ollama - Ollama-gated VSIX e2e (type4 fixture, never in CI)"
	@echo "  vsix-coverage  - VS Code E2E + enforce coverage threshold"
	@echo "  vsix-package   - Build the .vsix artifact"
	@echo "  vsix-rebuild   - Nuke + rebuild + repackage + install the VSIX from scratch"
	@echo "  jetbrains-build - Build the JetBrains plugin zip"
	@echo "  jetbrains-verify - Verify JetBrains plugin configuration and structure"
	@echo "  jetbrains-package - Alias for jetbrains-build"
