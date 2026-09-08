#!/usr/bin/env python3
"""Re-test every issue in the regression attribution report and render the
outcome ([SPEC-ID-GATE], [CORPUS-SCORE-RENDER] conventions).

Each row is a command this script ran, with the exit code it returned and the
evidence line it printed. Nothing here is transcribed by hand: the table is
built from captured process output, so it can be re-run and diffed.

    python3 scripts/repository/regression-verification.py

Options let the slower checks be skipped so the fast ones stay runnable:

    python3 scripts/repository/regression-verification.py --skip vsix
"""

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = "reports/regression-verification.md"
FIXTURE = "crates/deslop/tests/fixtures/js-cluster-extent-statement-runs"
NODE_FLOORS = (12, 30, 45)
CLI = "target/release/deslop"
SITE_TESTS = "site/tests"
TIMEOUT_SECONDS = 3600


@dataclass
class Check:
    """One executed verification step."""

    issue: str
    claim: str
    command: str
    cwd: str = "."
    group: str = "command"
    exit_code: int | None = None
    evidence: str = ""
    tags: list[str] = field(default_factory=list)

    @property
    def passed(self) -> bool:
        """Whether the step returned success."""
        return self.exit_code == 0


def run(check: Check, evidence_filter=None) -> Check:
    """Execute one check and record its exit code and evidence."""
    completed = subprocess.run(
        check.command,
        shell=True,
        cwd=REPO_ROOT / check.cwd,
        capture_output=True,
        text=True,
        timeout=TIMEOUT_SECONDS,
        check=False,
    )
    check.exit_code = completed.returncode
    output = f"{completed.stdout}\n{completed.stderr}".strip()
    check.evidence = (evidence_filter or default_evidence)(output)
    return check


# Runner summary lines worth quoting as evidence.
RUST_SUMMARY = "test result:"
GATE_SUMMARY = "spec-id gate:"
MOCHA_SUMMARY = ("passing", "failing")
MAX_EVIDENCE_LINES = 3


def default_evidence(output: str) -> str:
    """The runner summaries a reader needs, ignoring incidental scan chatter."""
    lines = [line.strip() for line in output.splitlines() if line.strip()]
    selected = [line for line in lines if is_summary(line)]
    return "; ".join(selected[:MAX_EVIDENCE_LINES]) if selected else (lines[-1] if lines else "no output")


def is_summary(line: str) -> bool:
    """Whether one line is a runner's own verdict rather than progress noise."""
    if line.startswith((RUST_SUMMARY, GATE_SUMMARY)):
        return RUST_SUMMARY not in line or " 0 passed;" not in line
    parts = line.split()
    return len(parts) >= 2 and parts[0].isdigit() and parts[1].rstrip("()") in MOCHA_SUMMARY


def scan_figure(report_path: Path) -> str:
    """The repository figure the renderer wrote, read from the report itself."""
    for line in report_path.read_text().splitlines():
        if line.startswith("repo:"):
            return line.strip()
    return "no repository figure printed"


def cluster_shape(report_path: Path, fixture_root: Path) -> dict:
    """Measure the report the way gh #520 states the defect."""
    document = json.loads(report_path.read_text())
    clusters = document["clusters"]
    mixed, cross_file, mid_line = 0, 0, 0
    for cluster in clusters:
        spans = {occ["end_line"] - occ["start_line"] + 1 for occ in cluster["occurrences"]}
        mixed += len(spans) > 1
        cross_file += len({occ["path"] for occ in cluster["occurrences"]}) > 1
        mid_line += sum(opens_mid_line(occ, fixture_root) for occ in cluster["occurrences"])
    return {
        "percent": document["metrics"]["duplication_percent"],
        "clusters": len(clusters),
        "mixed_span_clusters": mixed,
        "cross_file_clusters": cross_file,
        "mid_line_occurrences": mid_line,
    }


def opens_mid_line(occurrence: dict, fixture_root: Path) -> bool:
    """Whether a multi-line region begins part-way along its opening line."""
    if occurrence["end_line"] == occurrence["start_line"]:
        return False
    source = (fixture_root / occurrence["path"]).read_bytes()
    line_start = source.rfind(b"\n", 0, occurrence["start_byte"]) + 1
    return bool(source[line_start : occurrence["start_byte"]].strip())


def fixture_rows(binary: Path, label: str, output_dir: Path) -> list[dict]:
    """Scan the statement-run fixture at every floor with one engine."""
    rows = []
    fixture_root = REPO_ROOT / FIXTURE
    for floor in NODE_FLOORS:
        prefix = output_dir / f"{label}-{floor}"
        subprocess.run(
            [
                str(binary), str(fixture_root), "--embeddings", "off", "--no-incremental",
                "--min-nodes", str(floor), "--output", str(prefix), "--nohtml",
            ],
            cwd=REPO_ROOT, capture_output=True, check=True, timeout=TIMEOUT_SECONDS,
        )
        measured = cluster_shape(prefix.with_suffix(".json"), fixture_root)
        rows.append({"engine": label, "min_nodes": floor, **measured})
    return rows


def render(checks: list[Check], shape_rows: list[dict], site_line: str) -> str:
    """Render the verification document from captured results only."""
    failing = [check for check in checks if not check.passed]
    lines = [
        "# Regression re-test — gh #520 to #526",
        "",
        "Generated by `scripts/repository/regression-verification.py`. Every row is a "
        "command this script ran; the exit code and evidence are the process's own output.",
        "",
        f"Checks run: **{len(checks)}**. Failing: **{len(failing)}**.",
        "",
        "## Issue checks",
        "",
        "| gh | What was tested | Exit | Evidence |",
        "| --- | --- | ---: | --- |",
    ]
    lines += [
        f"| {check.issue} | {check.claim} | {check.exit_code} | {table_text(check.evidence)} |"
        for check in checks
    ]
    lines += [
        "",
        "## gh #520 — the statement-run fixture, measured on both engines",
        "",
        "`main` is the engine before this branch; `branch` is the engine under test. "
        "A duplication of one region repeated covers the same rows in every occurrence, "
        "so a mixed span count above zero is the defect gh #520 reports.",
        "",
        "| engine | --min-nodes | duplication % | clusters | mixed-span clusters | cross-file clusters | mid-line occurrences |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    lines += [
        f"| {row['engine']} | {row['min_nodes']} | {row['percent']:.4f} | {row['clusters']} | "
        f"{row['mixed_span_clusters']} | {row['cross_file_clusters']} | {row['mid_line_occurrences']} |"
        for row in shape_rows
    ]
    lines += ["", "## gh #520 — the reported scan", "", f"`{SITE_TESTS}` at default settings: `{site_line}`", ""]
    if failing:
        lines += ["## Failing checks", "", *(f"- {check.issue}: `{check.command}` exited {check.exit_code}" for check in failing), ""]
    return "\n".join(lines)


def table_text(value: str) -> str:
    """Keep captured output inside one markdown table cell."""
    return value.replace("|", "&#124;").replace("\n", " ")[:400]


def checks_to_run(skip: set[str]) -> list[Check]:
    """The verification steps, in issue order."""
    planned = [
        Check("#520", "Unrelated statement runs publish nothing, at every floor",
              "cargo test -p deslop --test suite unrelated_statement_runs", group="rust"),
        Check("#520", "Cluster occurrences describe one authored view",
              "cargo test -p deslop --test suite cluster_extent", group="rust"),
        Check("#521/#522/#524/#525", "Extension host: one paint table, tap-to-compare, whole-number mass",
              "npm test", cwd="clients/vscode", group="vsix"),
        Check("#523", "A reply from another build is refused by name, not by serde",
              "cargo test -p deslop-mcp backend::wire", group="rust"),
        Check("#524/#525", "Rendered panel: row taps compare, mass renders whole",
              "npx playwright test scripts/playwright-webview-smoke.spec.ts",
              cwd="clients/vscode", group="vsix"),
        Check("#526", "Every cited rule identifier resolves to a written rule",
              "cargo test -p deslop --test suite spec_id_traceability", group="rust"),
    ]
    return [check for check in planned if check.group not in skip]


def main() -> int:
    """Run every check, render the report, and fail if any check failed."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--skip", action="append", default=[], choices=["rust", "vsix"])
    parser.add_argument("--main-binary", default="", help="A pre-branch deslop for comparison")
    arguments = parser.parse_args()

    output_dir = REPO_ROOT / "target" / "regression-verification"
    output_dir.mkdir(parents=True, exist_ok=True)

    checks = [run(check) for check in checks_to_run(set(arguments.skip))]
    site_prefix = output_dir / "site"
    _scan = run(
        Check(
            "#520",
            "Reported scan",
            f"{CLI} {SITE_TESTS} --embeddings off --no-incremental --nohtml "
            f"--output {site_prefix}",
        )
    )
    site_line = scan_figure(site_prefix.with_suffix(".txt"))

    shape_rows = fixture_rows(REPO_ROOT / CLI, "branch", output_dir)
    if arguments.main_binary:
        shape_rows = fixture_rows(Path(arguments.main_binary), "main", output_dir) + shape_rows

    document = render(checks, shape_rows, site_line)
    destination = REPO_ROOT / arguments.output
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(document)
    print(destination)
    return 0 if all(check.passed for check in checks) else 1


if __name__ == "__main__":
    sys.exit(main())
