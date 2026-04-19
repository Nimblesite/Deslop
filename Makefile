# agent-pmo:424c8f8
# =============================================================================
# Standard Makefile — CodeDedup
# Cross-platform: Linux, macOS, Windows (via GNU Make)
# Rust CLI. See docs/specs/SPEC.md and docs/plans/PLAN.md.
# =============================================================================

.PHONY: build test lint fmt clean ci setup help

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
COVERAGE_THRESHOLDS_FILE := coverage-thresholds.json

# =============================================================================
# Standard Targets
# =============================================================================

## build: Compile/assemble all artifacts
build:
	@echo "==> Building..."
	cargo build --release --workspace

## test: Fail-fast tests + coverage + threshold enforcement.
##       See REPO-STANDARDS-SPEC [TEST-RULES] and [COVERAGE-THRESHOLDS-JSON].
test:
	@echo "==> Testing (fail-fast + coverage + threshold)..."
	rustup component add llvm-tools-preview 2>/dev/null || true
	cargo llvm-cov --workspace --all-targets --lcov --output-path lcov.info -- --fail-fast
	$(MAKE) _coverage_check

## lint: Run all linters/analyzers (read-only). Does NOT format.
lint:
	@echo "==> Linting..."
	cargo clippy --release --all-targets --workspace -- -D warnings

## fmt: Format all code in-place. Pass CHECK=1 for read-only check (CI use).
fmt:
	@echo "==> Formatting$(if $(CHECK), (check mode),)..."
	cargo fmt --all$(if $(CHECK), --check,)

## clean: Remove all build artifacts
clean:
	@echo "==> Cleaning..."
	cargo clean
	$(RM) lcov.info
	$(RM) .codededup-cache

## ci: lint + test + build (full CI simulation)
ci: lint test build

## setup: Post-create dev environment setup (used by devcontainer)
setup:
	@echo "==> Setting up development environment..."
	rustup component add llvm-tools-preview clippy rustfmt
	cargo install --locked cargo-llvm-cov
	@echo "==> Setup complete. Run 'make ci' to validate."

# ---------------------------------------------------------------------------
# Private: coverage enforcement (called from `test`)
# ---------------------------------------------------------------------------
_coverage_check:
	@if [ ! -f "$(COVERAGE_THRESHOLDS_FILE)" ]; then echo "FAIL: $(COVERAGE_THRESHOLDS_FILE) not found"; exit 1; fi; \
	THRESHOLD=$$(jq -r '.default_threshold' "$(COVERAGE_THRESHOLDS_FILE)"); \
	LH=$$(grep '^LH:' lcov.info | awk -F: '{sum+=$$2} END{print sum+0}'); \
	LF=$$(grep '^LF:' lcov.info | awk -F: '{sum+=$$2} END{print sum+0}'); \
	if [ "$$LF" -eq 0 ]; then echo "FAIL: No lines in lcov.info"; exit 1; fi; \
	PCT=$$(awk "BEGIN{printf \"%.1f\", $$LH/$$LF*100}"); \
	PCT_INT=$$(awk "BEGIN{printf \"%d\", $$LH/$$LF*100}"); \
	echo "Line coverage: $${PCT}% (threshold: $${THRESHOLD}%)"; \
	if [ "$$PCT_INT" -lt "$${THRESHOLD}" ]; then \
	  echo "FAIL: $${PCT}% < $${THRESHOLD}%"; exit 1; \
	else \
	  echo "OK: $${PCT}% >= $${THRESHOLD}%"; \
	fi

# =============================================================================
# Repo-Specific Targets
# =============================================================================

## help: List all available targets
help:
	@echo "Standard targets:"
	@echo "  build  - Compile/assemble all artifacts"
	@echo "  test   - Fail-fast tests + coverage + threshold enforcement"
	@echo "  lint   - All linters/analyzers (read-only, no formatting)"
	@echo "  fmt    - Format all code in-place (CHECK=1 for read-only CI check)"
	@echo "  clean  - Remove build artifacts"
	@echo "  ci     - lint + test + build (full CI simulation)"
	@echo "  setup  - Post-create dev environment setup"
