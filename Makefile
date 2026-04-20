# agent-pmo:9a71cbf
# =============================================================================
# Standard Makefile — Deslop
# Cross-platform: Linux, macOS, Windows (via GNU Make)
# Rust CLI. See docs/specs/SPEC.md and docs/plans/PLAN.md.
# =============================================================================

.PHONY: build test test-ollama lint fmt clean ci ci-ollama setup help build-release install-binary vsix-install vsix-build vsix-test vsix-test-ollama vsix-coverage vsix-package vsix-rebuild _vsix-stage-and-package


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
test:
	@echo "==> Testing (fail-fast + coverage + per-crate threshold)..."
	rustup component add llvm-tools-preview 2>/dev/null || true
	@_rust_ignore=$$(jq -r '.rust.ignore_filename_regex' "$(_COVERAGE_THRESHOLDS_FILE)"); \
	 cargo llvm-cov --workspace --all-targets --features deslop-core/live \
	    --ignore-filename-regex "$$_rust_ignore" \
	    --lcov --output-path lcov.info -- --skip ollama_
	@bash scripts/coverage-check.sh lcov.info "$(_COVERAGE_THRESHOLDS_FILE)"

## lint: Run all linters/analyzers (read-only). Does NOT format.
lint:
	@echo "==> Linting..."
	cargo clippy --release --all-targets --workspace -- -D warnings

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

## install-binary: Clean, build release, and install the binary onto the user's PATH.
##                 Deletes the installed binary and runs `cargo clean` first so a
##                 stale build artifact can never shadow the source on disk.
install-binary:
	@echo "==> Removing previously installed deslop binary..."
	cargo uninstall deslop 2>/dev/null || true
	$(RM) "$(HOME)/.cargo/bin/deslop"
	@echo "==> Cleaning build artifacts..."
	cargo clean --release --package deslop
	@echo "==> Building release binary from clean state..."
	cargo build --release --package deslop
	@echo "==> Installing deslop binary..."
	cargo install --locked --path crates/deslop --force

## vsix-install: Install Node deps for clients/vscode + webview-ui
vsix-install:
	cd clients/vscode && npm install --no-audit --no-fund
	cd clients/vscode/webview-ui && npm install --no-audit --no-fund

## vsix-build: Build deslop-lsp + deslop-mcp + VSIX bundle + webview UI.
vsix-build:
	cargo build --release -p deslop-lsp -p deslop-mcp -p deslop
	cd clients/vscode/webview-ui && npm run build
	cd clients/vscode && npm run build

## vsix-test: Run VS Code E2E tests against the REAL deslop-lsp binary.
vsix-test: vsix-install vsix-build
	cd clients/vscode && npm test

## vsix-test-ollama: Run the Ollama-gated VSIX e2e suite (csharp-type4
##                   fixture, provider=ollama, model=nomic-embed-text).
##                   NEVER runs in `make ci` / `make vsix-test`. Requires
##                   a local Ollama daemon and the model pulled.
vsix-test-ollama: vsix-install vsix-build
	cd clients/vscode && npm run test:ollama

## vsix-coverage: Run VS Code E2E + enforce the VSIX coverage threshold.
##                Threshold lives in clients/vscode/coverage-thresholds.json.
vsix-coverage: vsix-install vsix-build
	cd clients/vscode && npm run coverage

## vsix-package: Build the .vsix artifact (does not publish).
##               Stages the host-platform deslop-lsp + deslop-mcp + deslop
##               binaries into clients/vscode/bin/<platform>/ so the installed
##               extension can resolve them via the bundled path
##               ([VSIX-BINARY-VERSIONING]). CI stages every supported platform;
##               locally we only have the host toolchain so we only stage that one.
vsix-package: vsix-install vsix-build _vsix-stage-and-package

_vsix-stage-and-package:
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
	cd clients/vscode && npm run package

## vsix-rebuild: Nuke every build artifact (cargo target, staged bin/, node_modules,
##               dist/, out/, old .vsix), rebuild the workspace + webview + extension
##               from scratch, repackage the .vsix, and install it into the local
##               `code` CLI. Use when "why isn't my change showing up" strikes.
vsix-rebuild:
	@echo "==> [1/6] Removing cargo target/, staged bin/, node_modules, out/, dist/, old VSIX..."
	cargo clean
	$(RM) clients/vscode/bin
	$(RM) clients/vscode/node_modules
	$(RM) clients/vscode/webview-ui/node_modules
	$(RM) clients/vscode/out
	$(RM) clients/vscode/dist
	$(RM) clients/vscode/deslop-vscode.vsix
	$(RM) clients/vscode/coverage
	@echo "==> [2/6] Reinstalling npm deps (extension + webview-ui)..."
	cd clients/vscode && npm install --no-audit --no-fund
	cd clients/vscode/webview-ui && npm install --no-audit --no-fund
	@echo "==> [3/6] Rebuilding rust binaries (deslop, deslop-lsp, deslop-mcp)..."
	cargo build --release -p deslop -p deslop-lsp -p deslop-mcp
	@echo "==> [4/6] Rebuilding webview UI + extension bundle..."
	cd clients/vscode/webview-ui && npm run build
	cd clients/vscode && npm run build
	@echo "==> [5/6] Staging binaries + packaging .vsix..."
	$(MAKE) _vsix-stage-and-package
	@echo "==> [6/6] Installing the fresh .vsix into the VS Code CLI..."
	@if command -v code >/dev/null 2>&1; then \
	  code --install-extension clients/vscode/deslop-vscode.vsix --force; \
	else \
	  echo "WARN: 'code' CLI not on PATH — skipping install. VSIX is at clients/vscode/deslop-vscode.vsix"; \
	fi
	@echo "==> vsix-rebuild done. Reload the VS Code window to pick up the new extension."

## help: List all available targets
help:
	@echo "Standard targets:"
	@echo "  build          - Compile/assemble all artifacts"
	@echo "  test           - Fail-fast Rust tests + per-crate coverage threshold"
	@echo "  lint           - All linters/analyzers (read-only, no formatting)"
	@echo "  fmt            - Format all code in-place (CHECK=1 for read-only CI check)"
	@echo "  clean          - Remove build artifacts"
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
