# Test suite structure

## [TEST-ONE-BINARY] Link each crate's integration suite once

The `deslop`, `deslop-core`, `deslop-lsp`, and `deslop-mcp` crates disable Cargo's automatic integration-test discovery and explicitly register their `tests/suite.rs` entry point. That entry point declares the component suites as modules. Every suite remains compiled and selectable; collecting modules must not drop tests or create a second automatically discovered executable for each source file.

The four crate manifests and suite entry points implement this build contract. `crates/deslop/tests/skip_policy_contract.rs` checks that every corpus Makefile command names a test target the `deslop` manifest actually declares. This is a test-build rule; it adds no production runtime behavior. Release profile and CI consumers follow [CI-RELEASE-BUILD].
