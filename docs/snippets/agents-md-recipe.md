# `AGENTS.md` / `CLAUDE.md` recipe — teach your AI agent that **prevention beats cure**

Drop the block below into your project's `AGENTS.md` (used by Codex / Continue / Cursor) **and** `CLAUDE.md` (used by Claude Code) so every coding agent in your loop is told to call Deslop's `find-similar` MCP tool **before** writing new code, not afterwards.

The whole point of Deslop is to keep duplicates out of the repo in the first place. If your agents only run `top-offenders` to scrub existing dupes, you've reduced Deslop to a static analyzer. The live MCP loop earns its keep when agents query **during authoring**.

---

## Step 0 — wire `deslop-mcp` into your client

The MCP rules below assume your client can actually call `deslop-mcp`. Configure it once per machine, per client.

**The binary you wire up is the one bundled inside the installed VSIX**, not a `cargo install` build. Building Deslop from source produces `target/release/deslop-mcp` for testing only — it is deliberately not installed onto `PATH`. Point your MCP client at the unpacked VSIX path so the agent talks to the exact binary the extension ships and stays version-locked to it.

After `code --install-extension deslop-live-X.Y.Z-<target>.vsix`, the binary lives at:

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/<platform>/deslop-mcp
```

`<platform>` is `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, or `win32-x64`. `<VERSION>` is the installed extension version — update it when you update the VSIX.

- **Claude Code:** `claude mcp add deslop -s user -- ~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/<platform>/deslop-mcp --root .`
- **Codex (`~/.codex/config.toml`):**
  ```toml
  [mcp_servers.deslop]
  command = "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp"
  args    = ["--root", "."]
  ```
- **Claude Desktop, Cursor, Continue:** same idea — set `command` to the absolute VSIX path, set `args` to `["--root", "<absolute repo path>"]`.

Homebrew/Scoop CLI users (`brew install nimblesite/tap/deslop` / `scoop install deslop`) may use the bare-name `"command": "deslop-mcp"` form because the package manager versions the binary lock-step with releases. Everyone else uses the absolute VSIX path.

The full snippet set lives in the [root README](https://github.com/Nimblesite/Deslop#use-deslop-from-an-ai-agent-mcp).

---

## Paste this into `AGENTS.md` and `CLAUDE.md`

```markdown
## Deslop — duplicate-code awareness

This project runs **Deslop Live** — an MCP server (`deslop-mcp`) that exposes the live duplicate-code analysis as MCP tools. Prevention beats cure: it is cheaper to *not write* a duplicate than to refactor one out later.

### LAW: call `find-similar` BEFORE you author new code

Before you write any new function, method, class, helper, fixture, test setup,
parser branch, error type, route handler, view model, or any other code unit
larger than a few lines, you MUST call the `find-similar` MCP tool with the
proposed code (or its byte range, if it already exists in a draft buffer) and
inspect the response.

- If the response shows a cluster with **`signals.fused ≥ 0.85`** *or* the
  bucket is `identical` / `nearly_identical`, do NOT write the new copy.
  Reuse the canonical occurrence the tool returns. Extract a helper if needed.
- If the response is empty or the closest match is structurally distant
  (`signals.fused < 0.6`), proceed with authoring.
- If the response is borderline (`0.6 ≤ fused < 0.85`), read the canonical
  occurrence and decide whether the new code is genuinely different. Bias
  toward reuse.

`find-similar` accepts either:
- a **byte range** in an existing file (`path`, `start_byte`, `end_byte`), or
- a **snippet** (`snippet`, `language`) — useful when you have a draft in your
  scratchpad that has not been written to disk yet.

### When to use the OTHER Deslop tools instead

- **Fixing existing duplicates** (refactor / dedup work) → start with
  `top-offenders`, then `cluster-by-id` for the cluster you'll merge. Don't
  use `find-similar` for this — it answers a different question.
- **Investigating a specific file** → `report-for-file`.
- **Investigating a specific block before refactor** → `report-for-range`.
- **Schema reference for the JSON shapes** → call `schema-doc` *once* per
  session. Don't bundle it into every response.

### Hard rules

- Do NOT skip `find-similar` because "this looks new." Most clones are
  introduced by people who genuinely thought the code was new. The tool exists
  to override that intuition with evidence.
- Do NOT silence Deslop warnings by widening thresholds, marking clusters
  hidden, or splitting the code into trivially different shapes. If the tool
  flags something, treat it as a real signal until you've shown otherwise.
- Do NOT call `find-similar` after writing a duplicate "just to confirm." That
  is cure, not prevention. Call it BEFORE the keystroke that creates the
  duplicate.
```

---

## Why this matters

Deslop's value comes from being **in the agent's inner loop**, not from being a CI gate. Every static analyzer in the world can tell you, after the fact, that you have duplicates. Only a live MCP server can tell the agent **mid-generation**, before the keystroke that creates the duplicate, that an equivalent already exists.

If your agents don't call `find-similar` during authoring, you're paying for a live server and using it like a batch scanner. The recipe above closes that gap.

## Where this recipe lives

- This file: `docs/snippets/agents-md-recipe.md` in the Deslop repo.
- Linked from the project root [`CLAUDE.md`](../../CLAUDE.md) and [`README.md`](../../README.md).
- The MCP tool description for `find-similar` (`crates/deslop-mcp/src/tools/mod.rs`) leads with the same prevention-first framing so agents see it via `tools/list` even without reading this doc.
