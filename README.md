# Deslop

Live duplicate and dead code analysis inside your IDE. Inline warnings as you
type, worst-offender view, and a live channel your AI agent can consult before
it copy-pastes.

[Install for VS Code](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live) ·
[Add to GitHub Actions](https://deslop.live/docs/github-action/) ·
[Documentation](https://deslop.live/docs/) ·
[Releases](https://deslop.live/releases/)

[![Deslop showing ranked duplicate-code clusters, an inline warning, and a side-by-side comparison in VS Code.](site/src/assets/img/screenshot.webp)](https://deslop.live/docs/vscode-cluster-panel/)

## Install in your IDE

### VS Code

Install [Deslop Live from the VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live),
then open a supported codebase. Deslop starts automatically and updates its
findings as the code changes.

You can also install a platform-specific VSIX from the
[releases page](https://deslop.live/releases/):

```bash
code --install-extension deslop-live-X.Y.Z-<target>.vsix
```

The extension bundles the LSP server, MCP server, and CLI. JetBrains support is
in development.

## Add it to your pipeline

### GitHub Actions

```yaml
- uses: actions/checkout@v4
- uses: Nimblesite/Deslop@vX.Y.Z
  with:
    fail-over: "5.0" # fail above 5% duplicated lines
```

Substitute the [newest release](https://github.com/Nimblesite/Deslop/releases/latest)
for `X.Y.Z` — the tag you pin *is* the CLI version the action installs, so this
file names no version rather than committing one that silently rots into
installing an older CLI for everyone who copies it. The [Action
reference](https://deslop.live/docs/github-action/) renders the current number,
because it is built after each release rather than committed.

The action analyses the workspace, uploads JSON, text, and HTML reports, and
fails the job if duplication exceeds the threshold. It needs no token and only
the default `contents: read` permission.

See the [GitHub Action reference](https://deslop.live/docs/github-action/) for
all inputs, outputs, supported runners, and threshold rules.

### Other CI systems

Install the CLI and run:

```bash
deslop . --fail-over 5.0
```

Exit code `3` means the duplication threshold was breached. Reports are still
written so the pipeline can expose the offenders.

```bash
# macOS / Linux
brew install nimblesite/tap/deslop

# Windows
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
scoop install deslop
```

On Linux without Homebrew, `curl` the latest release archive instead — the
[install docs](https://deslop.live/docs/#linux-curl) have a checksum-verified
snippet that always resolves the newest version.

## How it works

Deslop parses code with tree-sitter and compares structure instead of merely
matching lines. That lets it find exact copies, renamed copies, and similar
implementations across a repository.

- **In the editor:** the LSP shows live findings and comparisons.
- **For coding agents:** the MCP server lets an agent call `find-similar` before
  writing a function, helper, or test setup.
- **In CI:** the CLI produces ranked reports and enforces a duplication ceiling.

All three surfaces use the same analysis engine. The worst offenders come first
so the report starts with the cleanup that matters most.

Supported languages: C#, Rust, Python, Dart, JavaScript, TypeScript, TSX, PHP,
F#, and Go.

## Connect a coding agent

The VS Code extension includes `deslop-mcp`. Point your MCP-capable agent at the
bundled binary, then add the [Deslop agent instruction](docs/snippets/agents-md-recipe.md)
to the repository so it checks for similar code before writing.

See [AI integration](https://deslop.live/docs/ai-integration/) for Claude Code,
Cursor, Copilot, Continue, Codex, and other MCP clients.

## Run a local scan

```bash
deslop                         # scan the current directory
deslop ./my-repo               # scan another repository
deslop . --output ./report     # write report.json, report.txt, report.html
```

Configuration lives in `.deslop.toml`. See the
[configuration reference](https://deslop.live/docs/configuration/) and
[configuration and reports guide](https://deslop.live/docs/configuration/#report-output).

## Project direction — accuracy first

Deslop is proven useful across every language it parses. Accuracy is now the
highest-value aim, ahead of features, languages, UI, and performance: every
reported cluster must be a real duplicate, and every real duplicate must be
reported. Fixing code that can cause a false positive or false negative
outranks all other work.

Duplicate code is where Deslop starts, not where it ends — finding slop,
preventing more, and removing it safely is the longer arc. See
[the messaging guide](docs/messaging.md) and
[the design specification](docs/specs/SPEC.md).

## Contributing

Bug reports with a reproduction are the most useful thing you can send,
especially inaccurate results. Code contributions are restricted while we audit
for accuracy — see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE) © NIMBLESITE PTY LTD.
