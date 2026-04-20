Fake binary directory used by the E2E harness. The real `codededup-lsp`
binary is resolved from the extension bundle at install time; for E2E we
plant a stub script here (copied by `test/install-fake-lsp.mjs`) so the
extension boots without shelling out to the Rust workspace.
