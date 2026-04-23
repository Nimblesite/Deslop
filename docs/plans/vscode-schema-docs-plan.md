# VS Code Schema Docs Plan

## Scope

Add the optional build-time VSIX copy of `docs/specs/REPORTING-CONTEXT.md` as
`schema_doc.md`.

The live extension already fetches `schema_doc` through
`deslop/reportSchemaDoc` and falls back to the report field, so drift is already
controlled. This plan exists only to improve offline documentation ergonomics
inside the packaged extension.

## Implementation Notes

- The source of truth remains `docs/specs/REPORTING-CONTEXT.md`.
- The generated copy must not become editable source.
- The VSIX should prefer the LSP RPC when a server is running and use the
  packaged markdown only as a cold/offline fallback.
- Keep the build step deterministic and cheap.

## TODO

- [ ] Add a VSIX build script that copies
      `docs/specs/REPORTING-CONTEXT.md` into the extension package as
      `schema_doc.md`.
- [ ] Wire `deslop.showSchemaDoc` fallback order to RPC, live report field,
      packaged `schema_doc.md`, then a short hard-coded failure message.
- [ ] Ensure the generated copy is excluded from source-of-truth edits if it is
      checked into a generated output directory.
- [ ] Add a unit test proving `openSchemaDoc` reads the packaged fallback when
      no LSP client and no live report are available.
- [ ] Add a packaging test or script assertion proving the `.vsix` contains the
      packaged schema doc.
