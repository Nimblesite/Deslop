# CLI invocation & terminal UX

The `deslop` binary (`crates/deslop`) is the one-shot, cold-cache surface — a thin
shell over `deslop-core` used for CI gates and audits. This file specifies its
invocation contract and its terminal UX: the human-readable preamble and summary
on stderr, colour and logging controls, and the derived output formats. Exit
codes and the JSON schema are owned by [pipeline.md §EXIT-CODES](pipeline.md) and
[pipeline.md §OUTPUT-SCHEMA-JSON](pipeline.md); this file governs everything the
operator sees and types.

## Invocation

### [CLI-INVOCATION-PATH] Scan-root argument
`deslop [PATH]` takes the directory to analyse as a single positional argument,
defaulting to the current working directory. A valid but empty scan root is not
an error: the run completes with a zero-cluster report and still emits every
enabled output format, exiting `0`. The CLI must never panic on a well-formed
path; only a non-existent path (or a path that is actually an MCP tool name,
[CLI-SUBCOMMAND-LOOKALIKE]) is rejected, with a named error.

### [CLI-SUBCOMMAND-LOOKALIKE] MCP-tool-name guard
`deslop top-offenders` must not be parsed as a positional `PATH=top-offenders`
and silently scanned as a non-existent directory — that produced a confident
false negative ("no duplication detected") against zero files for an agent that
mistook an MCP tool name for a CLI subcommand. When the positional path matches a
known MCP tool name or UI label, the CLI exits non-zero with a dedicated message
that names `deslop .` (or `deslop <path>`) as the correct form and explains the
MCP tool form is exposed via the `deslop-mcp` server, not the CLI.

### [CLI-INVOCATION-HELP] Help surface
`deslop --help` advertises the complete configurable surface so agents and humans
can discover the tuning knobs without reading source. The help text exits `0`,
writes to stdout, and must list at minimum the analysis knob (`--min-nodes`), the
output-format suppressors (`--nojson`, `--notext`, `--nohtml`), the re-render path
(`--from-report`), config discovery (`--config`), the cache opt-out
(`--no-incremental`), the diff scope (`--diff`, `--only-changed`), the
embedding flags
(`--embeddings`, `--embedding-provider`, `--embedding-model`,
`--embedding-endpoint`), and the terminal-UX flags (`--log-to-console`,
`--log-level`, `--no-color`, `--technical`).

### [CLI-ARG-NO-INCREMENTAL] `--no-incremental` cache opt-out
The fingerprint cache is **on by default**
([pipeline.md §PIPELINE-INCREMENTAL](pipeline.md)): a bare `deslop .` populates
`<scan-root>/.deslop/cache/fingerprints/` and a second run over an unchanged tree
skips tree-sitter entirely. Cache invalidation is content-addressed, so a file edited while nothing was
watching is re-parsed automatically and a warm run can never disagree with a cold
one ([PIPELINE-INCREMENTAL-INVALIDATION]).

`--no-incremental` turns the *fingerprint* cache off for one run: nothing is read,
nothing is written, and `cache_stats` reports `{ hits: 0, misses: 0 }`. It does
**not** disable the embedding cache ([fusion.md §FUSION-EMBED-PROVIDER](fusion.md)),
a separate layer keyed on provider/model identity — pass `--embeddings off` (the
default) for a run that writes nothing at all. A read-only checkout needs no flag:
an unwritable cache directory degrades to a full parse with a `warn!` and a
complete report. `--output` cannot redirect the cache; it stays at the scan root
([pipeline.md §OUTPUT-DIR](pipeline.md)).

### [CLI-ARG-DIFF] `--diff` unified-diff scope

> **Status: shipped.** Pinned by `crates/deslop/tests/diff_scoped_reporting.rs`, `crates/deslop/tests/diff_scoped_ingest.rs` (stale, malformed and missing diffs, and the `--diff -` stdin form) and
> `crates/deslop/tests/diff_ingest_refusals.rs`.

`--diff <FILE|->` supplies a unified diff (`-` reads stdin) whose new-side added
lines scope the report. The scan itself is unchanged — the whole tree is analysed
so changed code still matches untouched helpers — but every occurrence and cluster
is tagged against the diff ([pipeline.md §OUTPUT-SCHEMA-DIFF-TAGS](pipeline.md)).
Ingestion, path resolution, and the working-tree verification that rejects a stale
diff are owned by [pipeline.md §PIPELINE-DIFF-INGEST](pipeline.md). Conflicts with
`--from-report` (exit `2`): a re-render has no tree to verify the diff against.

### [CLI-ARG-ONLY-CHANGED] `--only-changed` filter

> **Status: shipped.** Pinned by `crates/deslop/tests/diff_scoped_reporting.rs`.

`--only-changed` (requires `--diff`, exit `2` without it) omits clusters that do
not intersect the diff from every rendered format, counts them in
`clusters_outside_diff`, and reroutes the `--fail-over` gate to the diff-scoped
percentage ([pipeline.md §METRICS-DIFF-SCOPE](pipeline.md)) so legacy debt cannot
fail a pre-merge check. The stderr summary switches to the delta form: newly
introduced clones first, then cross-file matches into existing code, then the
omitted count — three figures that reconcile, since every surviving cluster
intersects the diff and is therefore one or the other. A filtered run whose body
came out empty reports "no diff-affected duplication" with the omitted count; it
must never claim the codebase is clean, which would contradict the legacy debt it
just omitted.

### [CLI-INVOCATION-VERSION] Version output
`deslop --version` prints the plain line `deslop <version>` followed by a newline
to stdout, leaves stderr empty, and exits `0`. Adding `--json` (or
`--format json`) emits the deployment version manifest instead — a single-line
JSON object carrying `manifestVersion`, `name`, `version`, `kind` (`"cli"`),
`language` (`"rust"`), and `product` (see [deployment.md §DEPLOY-VERSION-CONTRACT](deployment.md)).
The version request is resolved before normal argument parsing so it never
depends on a valid scan path.

### [CLI-ARG-EMBEDDINGS] `--embeddings` mode validation
`--embeddings <MODE>` selects the meaning-based detection policy and accepts
exactly three values: `off` (default — skip the embedding pass entirely), `auto`
(probe the provider and silently fall back to `off` with a warning when it is
unreachable), and `required` (hard-fail when the provider cannot be reached). Any
other value is a user error: the CLI exits non-zero before any analysis with
`invalid --embeddings value <given>` rather than guessing a mode.

### [OUTPUT-FORMAT-DERIVED] Derived output formats
The canonical JSON report ([pipeline.md §OUTPUT-SCHEMA-JSON](pipeline.md)) is the
single source of truth; the text and HTML reports are **derived views** rendered
from the same in-memory `Report`, so the three never drift. A default run writes
all three (`<base>.json`, `<base>.txt`, `<base>.html`), where `<base>` defaults to
`<scan-root>/.deslop/deslop-report` ([pipeline.md §OUTPUT-DIR](pipeline.md)) and
`--output <PATH_PREFIX>` overrides it. `--nojson`, `--notext`,
and `--nohtml` suppress individual formats; suppressing all three is rejected as
an error (a silent run is never useful). `--from-report <file>` skips analysis
entirely and re-renders the derived text and HTML straight from an existing
canonical JSON, which doubles as the round-trip test of the JSON schema's
deserialization path.

## Terminal UX

### [UX-PREAMBLE] Run preamble
Before any analysis begins, `deslop` prints a preamble to stderr stating what the
run will do: a `deslop scanning <path> for duplicated code...` headline, the
`report → <base>.{json,txt,html}` destination, and the log destination
(`log → <file>` or `log → stderr (--log-to-console)`). Under `--technical` it
inserts an extra dimmed line surfacing the active knobs —
`min-nodes=<n>, embeddings=<mode>, incremental=<bool>` — so an operator can
confirm tuning at a glance. The preamble honours the resolved [UX-NO-COLOR]
colour choice.

### [UX-PLAIN-SUMMARY] Plain summary (default)
The default stderr summary is plain English aimed at a human in a terminal (no
jargon, no signal letters): a `Found N groups…` headline, friendly cache/embedding
sentences when applicable, a per-bucket breakdown, a one-sentence "Worst offender"
callout, the worst-10 ranked rows each with an action sentence, and a `Next:`
pointer to the HTML report. A zero-cluster report instead prints a single
"no duplication detected" success line and omits the worst-offender callout
entirely; the renderer is total and never panics on an empty corpus.
`--technical` upgrades this to the researcher view ([UX-TECHNICAL-BREAKDOWN]); it
changes verbosity, not the bucket labels.

### [UX-TECHNICAL-BREAKDOWN] Technical summary (`--technical`)
`--technical` switches the stderr summary from plain English to the researcher
view without changing the bucket labels (which are always the shared-text
`hybrid_title`, e.g. `Same shape, different content [structural-only]`, so the
same text serves a human reader and an AI scraper). The technical view adds a
column legend (`rank, signal, id, copies, AST nodes, weight,
(s=structural j=token e=embedding), files`) and, per ranked cluster, the
truncated cluster id, AST node count, ranking weight, and the fused signal triple.
It is purely additive verbosity layered on the [UX-PLAIN-SUMMARY] structure.

### [UX-TECHNICAL-CACHE] Cache statistics line
When the incremental cache ([pipeline.md §PIPELINE-INCREMENTAL](pipeline.md)) is
active and recorded any activity, the summary surfaces cache statistics. Under
`--technical` it prints the raw `cache: <hits> hit / <misses> miss` line; in
plain mode it prints the friendly `skipped <hits> unchanged file(s) using the
cache` only when there were hits, and stays silent when the cache neither hit nor
missed. The counts come from `Report.cache_stats`, the same source the text
renderer uses, so the surfaces never disagree.

### [UX-TECHNICAL-EMBEDDINGS] Embedding provenance line
When an embedding pass produced provenance (`Report.embedding_provenance` is set),
the summary reports it. Under `--technical` it prints the full provenance line
`embeddings: <provider>/<model>@<version> (<dims>-d, indexed <indexed>/<attempted>,
failures <failed>)`; in plain mode it prints a single human sentence instead of
the triple. When no embedding pass ran the line is omitted in both modes.

### [UX-NO-COLOR] `--no-color` flag
`--no-color` unconditionally disables ANSI colour in the stderr preamble and
summary; it is the top of the colour-precedence chain and wins over `NO_COLOR`,
`DESLOP_FORCE_COLOR`, and TTY detection. Colour is also disabled automatically
when stderr is not a terminal. Disabling colour swaps in an empty escape-string
theme, so the textual layout is byte-identical to the coloured form minus the
escapes — safe for CI capture and pipes.

### [UX-COLOR-NO-COLOR-ENV] `NO_COLOR` environment variable
`NO_COLOR` (set to any value, per <https://no-color.org>) suppresses all ANSI
colour in the stderr output. It takes precedence over `DESLOP_FORCE_COLOR`, so
when both are set the output is plain. This makes the documented colour-precedence
order: `--no-color` → `NO_COLOR` → `DESLOP_FORCE_COLOR` → stderr-TTY
autodetection.

### [UX-COLOR-FORCE] `DESLOP_FORCE_COLOR` environment variable
`DESLOP_FORCE_COLOR` (set to any value) forces ANSI colour in the stderr preamble
and summary even when stderr is not a terminal — the intended use is CI logs that
render colour but report a non-TTY. It sits below `--no-color` and `NO_COLOR` in
precedence (both still win), and above the automatic TTY probe.

### [UX-LOG-CONSOLE] `--log-to-console`
By default `deslop` keeps stderr human-readable: tracing events go to a
timestamped `deslop-<unix-seconds>.log` file in the report directory's `logs/`
subdirectory — `<scan-root>/.deslop/logs/` for a default run
([pipeline.md §OUTPUT-DIR](pipeline.md)) — and stderr carries only the preamble
and summary. `--log-to-console` reverses this — tracing events stream to stderr
(interleaved with the summary) and no log file is written, and no `logs/`
directory is created. This is the diagnostic mode for piping logs straight into a
terminal or a parent process.

### [UX-LOG-LEVEL] `--log-level`
`--log-level <LEVEL>` sets the minimum tracing severity emitted to whichever sink
is active, accepting `error`, `warn`, `info` (default), `debug`, and `trace`.
Raising the level suppresses lower-severity events (e.g. `warn` drops the `info`
"deslop invoked" entry). An unparseable level is rejected with a named error.
`RUST_LOG`, when set, overrides this flag entirely ([UX-LOG-RUST-LOG]).

### [UX-LOG-RUST-LOG] `RUST_LOG` precedence
When the `RUST_LOG` environment variable is set, it takes full precedence over
`--log-level` and supplies the entire tracing filter (per
`tracing_subscriber::EnvFilter` syntax), matching Rust-ecosystem convention.
`--log-level` is only consulted as the fallback when `RUST_LOG` is unset, so users
who never touch `RUST_LOG` still get exactly the severity they request on the
flag.
