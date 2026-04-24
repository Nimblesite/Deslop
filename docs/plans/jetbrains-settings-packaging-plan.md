# JetBrains Settings And Packaging Plan

## Scope

Finish the JetBrains plugin's configuration and release packaging layer.

The current scaffold launches `deslop-lsp <workspace-root> --min-nodes 30
--embeddings off` and resolves the binary from `DESLOP_BINARY_DIR`, `PATH`,
bundled `bin/<platform>/`, then bare `deslop-lsp`. This plan replaces hard-coded
launch choices with user settings and makes the plugin package self-contained.

## Implementation Notes

- Mirror the VSIX settings contract where possible so workspace state is
  portable between editors.
- Fresh installs must keep embeddings off until the user explicitly selects a
  model.
- Version checks are required before Marketplace publication. Development
  builds may keep the current resolver fallback until `deslop-lsp --version`
  exists.
- Release packaging should stage binaries; activation must not download them.

## TODO

- [ ] Add persistent JetBrains settings for `minNodes`, embedding
      provider/model/endpoint/mode, and incremental analysis.
- [ ] Update `DeslopLspServerDescriptor` to pass settings-derived LSP launch
      arguments instead of hard-coded values.
- [ ] Add validation for invalid `minNodes`, provider ids, and endpoint values.
- [ ] Add `deslop-lsp --version` support if it is still missing when version
      checks are implemented.
- [ ] Enforce exact binary/plugin version matching for `PATH` binaries before
      Marketplace publication.
- [ ] Stage platform binaries into `clients/jetbrains/bin/<platform>/` during
      release packaging.
- [ ] Add release workflow packaging for `deslop-jetbrains-<version>.zip`.
- [ ] Document local development and packaged install paths in
      `clients/jetbrains/README.md`.
