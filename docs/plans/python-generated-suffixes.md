# Python generated-file suffixes — auto report_hide

> Filed by external user (NimblesiteAgenticPlatform). Reproduces against
> `deslop-mcp` running on the agent-backend repo.

## Problem

`[EXCLUSION-CONFIG]` documents conservative built-in `report_hide` defaults
covering C# (`*.g.cs`, `*.generated.cs`, `*.designer.cs`, `*.pb.cs`,
`*.openapi.cs`) plus the `generated` path component and `alembic/versions`.
Python's typical generated-file conventions are not in
`BUILTIN_REPORT_HIDE_SUFFIXES` ([config.rs:51](../../crates/deslop-core/src/config.rs#L51)).

Concrete miss: `src/agent_backend/api/schemas_generated.py` carries
`"""...DO NOT HAND-EDIT..."""` and an explicit `Regenerate with ...`
docstring, but deslop ranks its self-duplication (cluster
`37c47c92219ce15d`, weight ≈ 1126) above every hand-written cluster in
`src/agent_backend/`. Top-of-report noise that the user cannot fix at
the source — extraction means hand-editing generated code, which the
header forbids.

## Proposal

Extend `BUILTIN_REPORT_HIDE_SUFFIXES` for Python:

```rust
const BUILTIN_REPORT_HIDE_SUFFIXES: &[&str] = &[
    // C#
    ".g.cs", ".generated.cs", ".designer.cs", ".pb.cs", ".openapi.cs",
    // Python
    "_generated.py", "_pb2.py", "_pb2_grpc.py",
];
```

Match against the file basename (suffix), same as the existing C# rules.

## Open question — content-marker detection

Suffix lists won't catch every project's convention. A second pass that
scans the first ~2 KiB of each candidate for a marker like
`DO NOT HAND-EDIT` / `DO NOT EDIT` / `@generated` would generalise the
rule. Add only if the suffix list proves insufficient — keep the
detector cheap (read at most 2 KiB, no regex on source per repo rule).

## Acceptance

- New E2E fixture: a Python repo with `*_generated.py` and a
  hand-written file that duplicates it.
- Cluster where every occurrence is in `*_generated.py` rolls into
  `clusters_hidden`, not the rendered `clusters` list.
- Cluster mixing generated + hand-written stays visible (existing
  `[EXCLUSION-CONFIG]` semantics).
- `BUILTIN_REPORT_HIDE_SUFFIXES` and `[EXCLUSION-CONFIG]` doc
  paragraph updated together.

## TODO

- [ ] Add Python suffixes to `BUILTIN_REPORT_HIDE_SUFFIXES`
- [ ] Update `[EXCLUSION-CONFIG]` doc paragraph in `docs/specs/exclusion.md`
- [ ] Add E2E fixture covering generated-only and mixed clusters
- [ ] Decide on content-marker detection (defer or implement)
