#!/usr/bin/env python3
"""Attribute reported regressions to the pull request that introduced them.

Every row in the emitted report is derived from `git log -S` over the first-parent
history of the release branch, then re-checked by asserting the presence or absence
of the anchor string at the commit and at its parent. Nothing here is hand-asserted:
re-running the script reproduces the table, and a wrong anchor shows up as an
UNVERIFIED row rather than as a confident-looking sentence.

The orphaned-identifier section (gh #526) is a lexical scan of comment text, which
is the method the issue itself specifies; it does not inspect source structure.

Usage:
    python3 scripts/repository/regression-attribution.py \
        --sweep <probe-output.txt> --output reports/regression-attribution-<date>.md
"""

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

BRANCH = "main"
DOC_DIR = "docs/"
CODE_DIRS = ("clients/vscode/src", "crates")
ID_PATTERN = re.compile(r"\[[A-Z][A-Z0-9]+(?:-[A-Z0-9]+)+\]")
PR_PATTERN = re.compile(r"\(#(\d+)\)\s*$")
SOURCE_SUFFIXES = (".rs", ".ts", ".tsx")
TESTS_MARKER = "test"
INTRODUCED = "introduced"
REMOVED = "removed"
UNVERIFIED = "UNVERIFIED"
NO_COMMIT = "no commit touches this anchor"


@dataclass(frozen=True)
class Anchor:
    """One measurable transition: a string entering or leaving the tree."""

    issue: int
    what: str
    needle: str
    paths: tuple[str, ...]
    direction: str


@dataclass(frozen=True)
class Attribution:
    """The commit a transition landed in, plus the presence check that proves it."""

    anchor: Anchor
    sha: str
    subject: str
    pull_request: str
    verdict: str


ANCHORS = (
    Anchor(520, "literal-variation filter bails out on any body-carrying call",
           "carries_body", ("crates/deslop-core/src/cluster_filters/calls.rs",), INTRODUCED),
    Anchor(520, "structural family split stage added to the cluster pipeline",
           "structural_family_split", ("crates",), INTRODUCED),
    Anchor(521, "cluster colour table rekeyed from clone kind to rank band",
           "nearly_identical", ("clients/vscode/src/tree/nodes.ts",), REMOVED),
    Anchor(521, "spec table mapping NearlyIdentical to a colour deleted",
           "NearlyIdentical", ("docs/specs/severity.md",), REMOVED),
    Anchor(521, "assertion that a demoted family is not painted act-now deleted",
           "demoted shape-only family",
           ("clients/vscode/src/test/unit/severity.unit.test.ts",), REMOVED),
    Anchor(521, "band-keyed colour table added under the SEVERITY-COLOR id",
           "SEVERITY_STYLE", ("clients/vscode/src/tree/nodes.ts",), INTRODUCED),
    Anchor(521, "bucket-keyed colour table removed",
           "CATEGORY_STYLE", ("clients/vscode/src/tree/nodes.ts",), REMOVED),
    Anchor(522, "percentile ramp table added",
           "export const SEVERITY_COLOR", ("clients/vscode/src/design.ts",), INTRODUCED),
    Anchor(522, "chart-palette table added to the tree surface",
           "charts.red", ("clients/vscode/src/tree/nodes.ts",), INTRODUCED),
    Anchor(522, "evidence-keyed paint table added",
           "DESLOP_SEVERITY_COLOR", ("clients/vscode/src/design.ts",), INTRODUCED),
    Anchor(522, "comment claiming the ramp is not the paint added",
           "is NOT the paint", ("clients/vscode/src/design.ts",), INTRODUCED),
    Anchor(522, "paint table fed from the rank band",
           "RANK_BAND_SEVERITY", ("clients/vscode/src/severity.ts",), INTRODUCED),
    Anchor(523, "IPC report deserialised with no version negotiation",
           "ipc report parse", ("crates",), INTRODUCED),
    Anchor(523, "wire field renamed, firing the unversioned parse",
           "pub mass: u64", ("crates/deslop-core/src/cluster.rs",), INTRODUCED),
    Anchor(524, "compare-against-canonical removed",
           "compareWithCanonical", ("clients/vscode",), REMOVED),
    Anchor(524, "per-row two-step selection added",
           "Select for comparison", ("clients/vscode",), INTRODUCED),
    Anchor(524, "test pinning the removed commands added",
           "IMPLICIT_COMPARE_COMMANDS", ("clients/vscode",), INTRODUCED),
    Anchor(525, "two-decimal helper added",
           "export function formatScore", ("clients/vscode/src",), INTRODUCED),
    Anchor(525, "integer mass routed through the two-decimal helper",
           "formatScore(cluster.mass)", ("clients/vscode",), INTRODUCED),
    Anchor(525, "wire field became an integer",
           "pub mass: u64", ("crates/deslop-core/src/cluster.rs",), INTRODUCED),
)


def git(*args: str) -> str:
    """Run a read-only git command and return its stdout."""
    done = subprocess.run(("git", *args), capture_output=True, text=True, check=False)
    return done.stdout if done.returncode == 0 else ""


def touching_commits(needle: str, paths: tuple[str, ...]) -> list[tuple[str, str]]:
    """Commits on the branch whose diff changes the number of `needle` occurrences."""
    out = git("log", BRANCH, "--oneline", "--no-abbrev-commit", "-S", needle, "--", *paths)
    rows = [line.split(" ", 1) for line in out.splitlines() if " " in line]
    return [(sha, subject) for sha, subject in rows]


def pull_request_of(subject: str) -> str:
    """The PR number a squashed subject line carries, or a marker when it carries none."""
    found = PR_PATTERN.search(subject)
    return f"#{found.group(1)}" if found else "(no PR in subject)"


def occurrence_count(rev: str, needle: str, paths: tuple[str, ...]) -> int:
    """How many lines contain `needle` at `rev` under `paths`."""
    out = git("grep", "-F", "-c", needle, rev, "--", *paths)
    return sum(int(line.rsplit(":", 1)[-1]) for line in out.splitlines() if ":" in line)


def verify(anchor: Anchor, sha: str) -> str:
    """Re-check the transition by counting the anchor at the commit and its parent."""
    before = occurrence_count(f"{sha}^", anchor.needle, anchor.paths)
    after = occurrence_count(sha, anchor.needle, anchor.paths)
    if anchor.direction == INTRODUCED and before == 0 and after > 0:
        return f"verified: absent at parent, {after} occurrence(s) after"
    if anchor.direction == REMOVED and before > after:
        return f"verified: {before} occurrence(s) at parent, {after} after"
    return f"{UNVERIFIED}: parent={before} commit={after}"


def is_transition(anchor: Anchor, sha: str) -> bool:
    """Whether this commit is the kind of transition the anchor is looking for."""
    before = occurrence_count(f"{sha}^", anchor.needle, anchor.paths)
    after = occurrence_count(sha, anchor.needle, anchor.paths)
    if anchor.direction == INTRODUCED:
        return before == 0 and after > 0
    return before > after


def attribute(anchor: Anchor) -> Attribution:
    """Resolve one anchor to the most recent commit that introduced or removed it.

    Most recent, not earliest: a string deleted and later restored — as
    `structural_family_split` was by #485 and #518 — would otherwise be credited
    to the commit that first wrote it rather than the one that put it in the tree
    standing today.
    """
    commits = touching_commits(anchor.needle, anchor.paths)
    if not commits:
        return Attribution(anchor, "-", NO_COMMIT, "-", UNVERIFIED)
    matches = [commit for commit in commits if is_transition(anchor, commit[0])]
    sha, subject = matches[0] if matches else commits[0]
    return Attribution(anchor, sha, subject, pull_request_of(subject), verify(anchor, sha))


def source_files() -> list[Path]:
    """Every tracked source file the identifier convention applies to."""
    listed = git("ls-files", *CODE_DIRS).splitlines()
    return [Path(name) for name in listed if name.endswith(SOURCE_SUFFIXES)]


def identifiers_in(paths: list[Path]) -> set[str]:
    """Bracketed identifiers cited anywhere in the given files."""
    found: set[str] = set()
    for path in paths:
        text = path.read_text(encoding="utf-8", errors="replace")
        found.update(ID_PATTERN.findall(text))
    return found


def documented_identifiers() -> set[str]:
    """Bracketed identifiers defined anywhere under the documentation tree."""
    listed = [Path(name) for name in git("ls-files", DOC_DIR).splitlines()]
    return identifiers_in(listed)


def orphan_identifiers() -> list[str]:
    """Identifiers cited by code or tests that no document defines."""
    return sorted(identifiers_in(source_files()) - documented_identifiers())


def cited_in_production(identifier: str) -> int:
    """How many non-test source files cite the identifier."""
    files = [p for p in source_files() if TESTS_MARKER not in str(p)]
    return sum(1 for path in files if identifier in path.read_text(encoding="utf-8", errors="replace"))


def label(commit: tuple[str, str]) -> str:
    """A commit rendered as its PR number, falling back to its short sha."""
    sha, subject = commit
    found = PR_PATTERN.search(subject)
    return f"#{found.group(1)}" if found else f"`{sha[:8]}`"


def drop_label(identifier: str, commit: tuple[str, str]) -> str:
    """The commit that removed an identifier from the documents, once confirmed."""
    sha = commit[0]
    before = occurrence_count(f"{sha}^", identifier, (DOC_DIR,))
    after = occurrence_count(sha, identifier, (DOC_DIR,))
    suffix = "" if before > 0 and after == 0 else f" ({UNVERIFIED} {before}/{after})"
    return f"{label(commit)}{suffix}"


def identifier_history(identifier: str) -> tuple[str, str]:
    """The PR that first cited the identifier, and the PR that dropped its document."""
    cited = touching_commits(identifier, CODE_DIRS)
    documented = touching_commits(identifier, (DOC_DIR,))
    first = label(cited[-1]) if cited else "-"
    dropped = drop_label(identifier, documented[0]) if documented else "never documented"
    return first, dropped


def branch_order() -> dict[str, int]:
    """Every commit on the branch, mapped to its age rank (0 = oldest)."""
    shas = git("rev-list", "--first-parent", BRANCH).split()
    return {sha[:8]: index for index, sha in enumerate(reversed(shas))}


def sweep_rows(sweeps: list[Path]) -> list[str]:
    """Probe lines from every sweep file, de-duplicated and ordered oldest first."""
    lines: dict[str, str] = {}
    for sweep in sweeps:
        for line in sweep.read_text(encoding="utf-8").splitlines():
            if line.strip():
                lines[line.split(" ", 1)[0]] = line
    order = branch_order()
    return [lines[sha] for sha in sorted(lines, key=lambda s: order.get(s, -1))]


def render_attributions(results: list[Attribution]) -> list[str]:
    """The attribution table, one row per measured transition."""
    lines = ["| gh | transition | PR | commit | check |", "| --- | --- | --- | --- | --- |"]
    for item in results:
        lines.append(
            f"| #{item.anchor.issue} | {item.anchor.what} | {item.pull_request} "
            f"| `{item.sha[:8]}` | {item.verdict} |"
        )
    return lines


def render_orphans(identifiers: list[str]) -> list[str]:
    """The orphaned-identifier table for gh #526."""
    lines = [
        f"Orphaned identifiers: **{len(identifiers)}**.",
        "",
        "| identifier | production files citing it | first cited by | document dropped by |",
        "| --- | --- | --- | --- |",
    ]
    for identifier in identifiers:
        first, dropped = identifier_history(identifier)
        lines.append(
            f"| `{identifier}` | {cited_in_production(identifier)} | {first} | {dropped} |"
        )
    return lines


def render_header() -> list[str]:
    """Title and provenance line."""
    head = git("rev-parse", "--short=8", BRANCH).strip()
    return [
        "# Regression attribution — gh #520 to #526",
        "",
        f"Measured against `{BRANCH}` at `{head}` by "
        "`scripts/repository/regression-attribution.py`. Every row is a `git log -S` "
        "result re-checked by counting the anchor string at the commit and at its parent.",
        "",
    ]


def render_sweep(sweep: list[str]) -> list[str]:
    """The measured detector behaviour section for gh #520."""
    return [
        "## gh #520 — measured detector behaviour",
        "",
        "One frozen copy of `site/tests` scanned by a release build of each commit; "
        "`mixed` counts published clusters whose occurrences do not all span the same "
        "number of lines.",
        "",
        "```",
        *sweep,
        "```",
        "",
    ]


def render(results: list[Attribution], identifiers: list[str], sweep: list[str]) -> str:
    """Assemble the whole report."""
    lines = [
        *render_header(),
        "## Attributed transitions",
        "",
        *render_attributions(results),
        "",
        *render_sweep(sweep),
        "## gh #526 — identifiers cited in code but defined in no document",
        "",
        *render_orphans(identifiers),
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    """Emit the attribution report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sweep", type=Path, nargs="*", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    results = [attribute(anchor) for anchor in ANCHORS]
    report = render(results, orphan_identifiers(), sweep_rows(args.sweep))
    args.output.write_text(report, encoding="utf-8")
    unverified = sum(1 for item in results if item.verdict.startswith(UNVERIFIED))
    print(f"wrote {args.output} ({len(results)} anchors, {unverified} unverified)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
