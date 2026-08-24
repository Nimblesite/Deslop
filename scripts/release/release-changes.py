#!/usr/bin/env python3
"""Shipwright change-sensitive release classifier.

Given a `shipwright.json` manifest and a git range (prior release tag .. current
tag), decide which release surfaces actually changed so the release workflow can
skip building/publishing the components that did not.

Design constraints (from the Shipwright contract — proposed spec SWR-REL-CHANGES):

* Single version per tag (SWR-REL-VERSION): a published component always bundles
  binaries stamped to the tag version. Therefore any component whose artifact
  bundles a native binary (VSIX bundles lsp+mcp, JetBrains bundles lsp) requires
  the binary build whenever it ships — `activation-verify` is `onMismatch: error`.
  A Rust change is consequently a *full* release: every downstream artifact must
  rebuild. The website is the only fully decoupled surface.
* Diff with native git, never `tj-actions/changed-files` (SWR-SEC-ACTION-PINNING
  flags that action's CVE history).
* Unknown / unclassified changes default to a full release (fail safe, never a
  silent empty release).

Outputs (GitHub Actions `name=value` lines, also `--json`):
  rust, vscode, jetbrains, website   per-surface booleans
  full          rust OR release-infra OR nothing-classified -> build+publish all
  any_component full OR vscode OR jetbrains -> the binary build matrix must run
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys

# Surface -> path prefixes. A prefix ending in "/" matches a directory subtree;
# otherwise it matches that exact repo-root file. `infra` and any unclassified
# path force a full release because they can affect how every artifact is built.
SURFACE_PREFIXES: dict[str, list[str]] = {
    "rust": ["crates/", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/", "docs/models/"],
    "vscode": ["clients/vscode/"],
    "jetbrains": ["clients/jetbrains/"],
    "website": ["site/"],
    "infra": [
        "shipwright.json",
        ".github/workflows/release.yml",
        ".github/workflows/deploy-pages.yml",
        "scripts/",
        "Makefile",
    ],
}

# Paths that change no shippable artifact: they trigger neither a surface nor a
# full release. `infra` above is matched first, so the two release workflows still
# force full while the rest of `.github/` (CI, CodeQL, dependabot) stays neutral.
# A path matching NOTHING here or in SURFACE_PREFIXES is treated as unknown and
# forces a full release (fail safe). `docs/models/` is rust (build.rs consumes it).
NEUTRAL_PREFIXES: list[str] = [
    # Docs & marketing images — not shipped in any artifact (the website is site/).
    "docs/", "README.md", "AGENTS.md", "CLAUDE.md", "SECURITY.md", "CHANGELOG.md",
    "LICENSE", "examples/", "deslop-home-desktop.png", "deslop-home-mobile.png",
    # Agent / editor / tooling config.
    ".github/", ".claude/", ".agents/", ".codex/", ".clinerules", ".cursorrules",
    ".windsurfrules", "opencode.json", ".devcontainer/", ".vscode/",
    # Repo meta & non-release gates (CI thresholds change no shipped artifact).
    ".gitignore", ".gitattributes", ".editorconfig", "rustfmt.toml",
    "coverage-thresholds.json", ".deslop.toml", "lcov-mcp.info",
]


def _matches(path: str, prefix: str) -> bool:
    """Match a dir-subtree prefix ('foo/'), or a bare name as that file OR dir."""
    if prefix.endswith("/"):
        return path.startswith(prefix)
    return path == prefix or path.startswith(prefix + "/")


def classify_path(path: str) -> str:
    """Map one changed path to a surface, or 'unknown' if nothing claims it."""
    for surface, prefixes in SURFACE_PREFIXES.items():
        if any(_matches(path, prefix) for prefix in prefixes):
            return surface
    if any(_matches(path, prefix) for prefix in NEUTRAL_PREFIXES):
        return "neutral"
    return "unknown"


def _git(args: list[str], root: str) -> str:
    """Run a read-only git command in `root` and return stripped stdout."""
    result = subprocess.run(
        ["git", "-C", root, *args],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def _parse_version(tag: str) -> tuple[int, int, int] | None:
    """Parse `vX.Y.Z` (ignoring any prerelease suffix); None if not semver."""
    core = tag[1:].split("-", 1)[0] if tag.startswith("v") else ""
    parts = core.split(".")
    if len(parts) != 3 or not all(part.isdigit() for part in parts):
        return None
    major, minor, patch = (int(part) for part in parts)
    return (major, minor, patch)


def prior_release_tag(head_tag: str, root: str) -> str | None:
    """Greatest `vX.Y.Z` tag strictly below `head_tag`, or None if none exist."""
    head_version = _parse_version(head_tag)
    candidates = []
    for tag in _git(["tag", "--list", "v*"], root).splitlines():
        version = _parse_version(tag)
        if version is not None and (head_version is None or version < head_version):
            candidates.append((version, tag))
    if not candidates:
        return None
    return max(candidates)[1]


def changed_paths(base: str, head: str, root: str) -> list[str]:
    """Files differing between base and head (base..head)."""
    return [line for line in _git(["diff", "--name-only", f"{base}..{head}"], root).splitlines() if line]


def classify(paths: list[str]) -> dict[str, bool]:
    """Fold classified paths into the surface/full/any_component decision."""
    surfaces = {classify_path(path) for path in paths}
    rust = "rust" in surfaces
    vscode = "vscode" in surfaces
    jetbrains = "jetbrains" in surfaces
    website = "website" in surfaces
    infra = "infra" in surfaces
    classified_a_surface = bool(surfaces & {"rust", "vscode", "jetbrains", "website", "infra"})
    full = rust or infra or "unknown" in surfaces or not classified_a_surface
    return {
        "rust": rust,
        "vscode": vscode,
        "jetbrains": jetbrains,
        "website": website,
        "full": full,
        "any_component": full or vscode or jetbrains,
    }


def resolve_range(args: argparse.Namespace) -> tuple[str | None, str]:
    """Resolve (base, head); base is None when there is no prior release."""
    head = args.head
    if args.base:
        return (args.base, head)
    head_tag = head if _parse_version(head) else _git(["describe", "--tags", "--exact-match", head], args.root)
    return (prior_release_tag(head_tag, args.root), head)


def emit(outcome: dict[str, bool], args: argparse.Namespace, base: str | None) -> None:
    """Write GitHub Actions outputs, a human summary, and optional JSON."""
    lines = [f"{key}={'true' if value else 'false'}" for key, value in outcome.items()]
    output_path = args.github_output or os.environ.get("GITHUB_OUTPUT")
    if output_path:
        with open(output_path, "a", encoding="utf-8") as handle:
            handle.write("\n".join(lines) + "\n")
    if args.json:
        print(json.dumps({**outcome, "base": base, "head": args.head}))
    else:
        print(f"release-changes: base={base or '<none>'} head={args.head}", file=sys.stderr)
        for line in lines:
            print(f"  {line}", file=sys.stderr)


def main() -> int:
    """Classify the release range and emit the per-surface decision."""
    parser = argparse.ArgumentParser(description="Shipwright change-sensitive release classifier")
    parser.add_argument("--manifest", default="shipwright.json", help="path to shipwright.json")
    parser.add_argument("--root", default=".", help="repo root for git operations")
    parser.add_argument("--base", default="", help="prior release ref (auto-detect prior v* tag if empty)")
    parser.add_argument("--head", default="HEAD", help="ref being released (tag or HEAD)")
    parser.add_argument("--github-output", default="", help="path to write name=value outputs")
    parser.add_argument("--json", action="store_true", help="print the decision as JSON to stdout")
    args = parser.parse_args()

    if not os.path.exists(os.path.join(args.root, args.manifest)):
        print(f"release-changes: manifest {args.manifest} not found; forcing full release", file=sys.stderr)
        emit(classify([]) | {"full": True, "any_component": True, "rust": True}, args, None)
        return 0

    base, head = resolve_range(args)
    if base is None:
        print("release-changes: no prior release tag; forcing full release", file=sys.stderr)
        emit({"rust": True, "vscode": True, "jetbrains": True, "website": True, "full": True, "any_component": True}, args, None)
        return 0

    emit(classify(changed_paths(base, head, args.root)), args, base)
    return 0


if __name__ == "__main__":
    sys.exit(main())
