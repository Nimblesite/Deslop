# Deslop for VS Code

**Clean up the duplicate code in your repo, and stop your coding agents from writing more.**

Deslop finds copied and near-copied code across your workspace and ranks it worst first, so you know what to remove. While you or your agent edit, it flags new code that duplicates something that already exists. It is an open-source alternative to jscpd that runs live in the editor and inside the agent loop.

[![Deslop demo: inspect a duplicate cluster and compare both copies in VS Code.](https://deslop.live/assets/img/deslop-demo.gif)](https://deslop.live/#demo)

[Watch the narrated demo](https://deslop.live/#demo) · [Supported languages](https://deslop.live/#languages)

## Clean up what's there

- **Top Offenders** lists the duplicate clusters worth removing, biggest first.
- Open a cluster to see every copy, then diff any copy against the canonical one before you refactor.
- The report view filters by language, severity and path. Walkthrough: [VS Code Cluster Panel](https://deslop.live/docs/vscode-cluster-panel/).

## Stop new duplicates

- **While you type**, a label at the end of the line tells you the code matches an existing cluster.
- **Your coding agent** gets the same check over MCP, so it can see that the code already exists before it writes another copy. Agents inside VS Code (such as Copilot Chat) pick it up automatically.
- Optional local embeddings through Ollama catch code that behaves the same but reads differently. Nothing leaves your machine.

### Agents outside VS Code

Claude Code, Claude Desktop, Codex, Cursor and Continue need the absolute path to the `deslop-mcp` binary bundled with this extension:

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp
```

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp --root .
```

Update `<VERSION>` whenever the extension updates. Setup for other clients: [AI integration](https://deslop.live/docs/ai-integration/).

## How it compares

Deslop parses code with tree-sitter and compares syntax trees, not token streams. That lets it report copies with renamed identifiers, changed literals or inserted statements as well as exact ones.

Deslop is tested against [judged clone registers](https://github.com/Nimblesite/Deslop/tree/main/corpus/register): code pairs from pinned open-source repositories that a reviewer has marked as real copies or not copies. A missed real copy counts as a false negative, and a reported non-copy counts as a false positive. [Accuracy Transparency](https://deslop.live/docs/accuracy-transparency/) explains the scoring and how the duplication percentage is calculated.

No head-to-head benchmark against jscpd or other duplicate detectors has been published yet, so we make no claim that Deslop is more accurate than they are. The register results cover only the judged examples.

## Install

- [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live), or the platform `.vsix` from the [latest release](https://github.com/Nimblesite/Deslop/releases/latest).
- The CLI is also available for CI: `brew install nimblesite/tap/deslop` or `scoop install deslop`.

The extension ships its own binaries, so nothing is downloaded after install. Settings live under **Deslop** in the Settings UI.

## License

MIT.
