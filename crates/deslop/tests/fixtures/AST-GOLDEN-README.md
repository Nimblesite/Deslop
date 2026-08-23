# AST goldens — read before you regenerate one

Each `ast-golden-<language>/` holds a `Sample.<ext>` source and a
`Sample.expected.ast` dump of the normalised tree. `debug_ast_dump_matches_committed_golden*`
in `crates/deslop/tests/cli/cache_and_debug.rs` compares `--debug-ast` output against the
committed dump byte-for-byte ([PIPELINE-NORMALIZE-AST]).

## Regenerating is not a fix

Byte-for-byte equality only proves the tool still agrees with a file the tool wrote. It cannot
tell you the expectation is *right*, so a wrong golden certifies itself forever.

That is not hypothetical. Every one of these goldens recorded the synthetic `__file__` root
spanning trivia the normaliser had already dropped:

| fixture | committed | correct | dropped trivia claimed as code |
| --- | --- | --- | --- |
| `ast-golden-go` | `[0..3573]` | `[759..3572]` | 759 bytes of leading comments |
| `ast-golden-fsharp` | `[0..317]` | `[52..316]` | 52 bytes of leading comments |
| the other nine | `[0..N]` | `[0..N-1]` | the trailing newline |

The Go root claimed 759 bytes of package comment as part of a duplicate. Every whole-file
cluster reported a range starting at byte 0 no matter how much trivia sat above the code, so a
user opening the occurrence landed on a licence header, and the offsets stopped tracking edits
that moved the code (`crates/deslop-mcp/tests/issue_153_rescan_freshness.rs`). The goldens were
wrong for as long as the fixtures existed and nothing caught it, because the only check compared
the tool to itself.

## What guards them now

`assert_dump_is_correct` checks the committed dump against the contract rather than against the
tool, so a dump regenerated from a broken build fails even when it matches the committed bytes
exactly:

- `__file__` spans exactly `[min(child.start) .. max(child.end)]` — never leading or trailing
  trivia. Real nodes keep their own span; a declaration's braces belong to the duplication even
  when a comment sits between them.
- every node nests inside its nearest shallower ancestor
- every range is non-empty and within the source
- no `comment` node survives normalisation
- every operator leaf is named by the token it stands for: its kind is `__op__` followed by
  exactly the bytes it spans ([PIPELINE-NORMALIZE-AST-OPERATOR]). A dump full of a shared
  `__op__` placeholder is byte-for-byte stable and completely wrong — it records a tree in
  which `alpha + beta` and `alpha - beta` are the same subtree, and regenerating would promote
  that to "expected" exactly as it once promoted the dropped Go comments. The name is read back
  out of the fixture, so this invariant is proved against the source and not against the tool.

This was verified by reverting the fix, regenerating all eleven goldens from the broken build,
and confirming the byte-for-byte check passed while all eleven tests still failed on the contract.

## So if a golden fails

Work out which side is wrong before touching the file.

1. **The dump changed and the contract still holds** — a grammar bump or a `normalise_kind` edit.
   Confirm the diff is only what you intended, then regenerate. Note that this also changes every
   user's fingerprints and invalidates their cache.
2. **The contract assertion fired** — the build is wrong, not the golden. Fix the source. A
   regenerated golden will not go green, which is the point.
