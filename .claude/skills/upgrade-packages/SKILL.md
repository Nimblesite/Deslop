---
name: upgrade-packages
description: Upgrade all dependencies/packages to their latest versions for the Deslop Rust workspace and the Node packages (VSIX extension, webview-ui, site). Use when the user says "upgrade packages", "update dependencies", "bump versions", "update packages", or "upgrade deps".
argument-hint: "[--check-only] [--major] [package-name]"
---
<!-- agent-pmo:b636503 -->

# Upgrade Packages

Upgrade all project dependencies to their latest compatible (or latest major, if `--major`) versions.

This repo has exactly two package managers:

- **cargo** (Rust workspace at repo root — `Cargo.toml`, members `crates/deslop-core`, `crates/deslop`, `crates/deslop-lsp`, `crates/deslop-mcp`).
- **npm** (three separate Node projects: [clients/vscode/package.json](clients/vscode/package.json), [clients/vscode/webview-ui/package.json](clients/vscode/webview-ui/package.json), [site/package.json](site/package.json)).

Process cargo first, then each npm project in turn.

## Arguments

- `--check-only` — List outdated packages without upgrading. Stop after Step 2.
- `--major` — Include major version bumps (breaking changes). Without this flag, stay within semver-compatible ranges.
- Any other argument is treated as a specific package name to upgrade (instead of all packages).

## Step 1 — Confirm the manifest files exist

Expected layout:

- `Cargo.toml` — Rust workspace manifest (exact pins — `=x.y.z` — are a CLAUDE.md invariant).
- `clients/vscode/package.json` — VS Code extension.
- `clients/vscode/webview-ui/package.json` — React webview bundled inside the extension.
- `site/package.json` — Eleventy static site.

If any of the above is missing but the directory exists, stop and tell the user — don't guess. If a manifest has been genuinely removed, that's out-of-scope for this skill.

## Step 2 — List outdated packages

Run the appropriate command for each manifest BEFORE upgrading anything. Show the user what will change.

### Rust (workspace root)

```bash
cargo outdated        # install: cargo install cargo-outdated
cargo update --dry-run
```

**Read the docs:** https://doc.rust-lang.org/cargo/commands/cargo-update.html

**Deslop-specific constraint:** the tree-sitter grammars (`tree-sitter`, `tree-sitter-c-sharp`, `tree-sitter-rust`, `tree-sitter-python`) are **exact-pinned (`=x.y.z`)** in the workspace `Cargo.toml`, and [.github/workflows/ci.yml](.github/workflows/ci.yml) has a `Verify grammar pins` step that fails the build if any pin drifts off `=x.y.z`. When upgrading grammars, keep the `=` prefix and update the pin to the new exact version — never let cargo widen it to a caret range.

### Node (each project — run in its own working directory)

```bash
# VS Code extension
cd clients/vscode && npm outdated
# React webview inside the extension
cd clients/vscode/webview-ui && npm outdated
# Eleventy site
cd site && npm outdated
```

**Read the docs:** https://docs.npmjs.com/cli/v10/commands/npm-update

If `--check-only` was passed, **stop here** and report the combined outdated list (cargo + each npm project).

## Step 3 — Read the official upgrade docs

**Before running any upgrade command, you MUST fetch and read the official documentation URL listed above for the detected package manager.** Use WebFetch to retrieve the page. This ensures you use the correct flags and understand the behavior. Do not guess at flags or options from memory.

## Step 4 — Upgrade packages

If a specific package name was given as an argument, upgrade only that package (in whichever manifest contains it). Otherwise run the full upgrade for each manifest.

### Rust (workspace root)

```bash
cargo update                          # semver-compatible updates
# --major flag:
cargo update --breaking               # major version bumps (cargo 1.84+)
```

Run from the workspace root so every workspace member is updated.

**After any grammar bump:** re-run the `Verify grammar pins` check locally by grepping for `tree-sitter` in `Cargo.toml` and confirming every line still matches the `"=X.Y.Z"` pattern. If cargo relaxed a pin, edit it back to `=X.Y.Z` before committing.

### Node (each project — run in its own working directory)

```bash
# VS Code extension
cd clients/vscode && npm update                 # semver-compatible
cd clients/vscode && npx npm-check-updates -u && npm install   # --major: bump package.json to latest majors

# React webview
cd clients/vscode/webview-ui && npm update
cd clients/vscode/webview-ui && npx npm-check-updates -u && npm install

# Eleventy site
cd site && npm update
cd site && npx npm-check-updates -u && npm install
```

## Step 5 — Verify the upgrade

After upgrading, run the full CI simulation:

```bash
make ci
```

This runs `fmt --check → lint → test → build → vsix-coverage` against the upgraded workspace. If you upgraded the site's deps, also rebuild the site:

```bash
cd site && npm run build
```

If tests fail:

1. Read the failure output carefully.
2. Check the changelog / migration guide for the upgraded packages (fetch the release notes URL if available).
3. Fix breaking changes in the code.
4. Re-run `make ci`.
5. If stuck after 3 attempts on the same failure, report it to the user with the error details and the package that caused it.

## Step 6 — Report

Provide a summary per manifest:

- Packages upgraded (old version → new version) — group by manifest (`Cargo.toml`, `clients/vscode`, `clients/vscode/webview-ui`, `site`).
- Packages skipped (and why, e.g., major version bump without `--major` flag).
- `make ci` result after upgrade (and `site` build result, if touched).
- Any breaking changes that were fixed.
- Any packages that could not be upgraded (with error details).
- Confirm tree-sitter grammar pins are still exact (`=X.Y.Z`) after the upgrade.

## Rules

- **Always list outdated packages first** before upgrading anything.
- **Always read the official docs** for the package manager before running upgrade commands.
- **Always run `make ci` after upgrading** to catch breakage immediately.
- **Never remove packages** unless they were explicitly deprecated and replaced.
- **Never downgrade packages** unless rolling back a broken upgrade.
- **Never modify lockfiles manually** (`Cargo.lock`, `package-lock.json`) — let the package manager regenerate them.
- **Never relax tree-sitter grammar pins.** They stay on `=X.Y.Z` — the CI grammar-pin check exists because a loose pin is a bug.
- **Commit nothing** — leave changes in the working tree for the user to review.

## Success criteria

- All outdated packages upgraded to latest compatible (or latest major if `--major`) across cargo + every npm project.
- `make ci` passes.
- Tree-sitter grammars still exact-pinned.
- User has a clear summary of what changed per manifest.
