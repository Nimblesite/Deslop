#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["tree-sitter==0.25.2", "tree-sitter-rust==0.24.2", "tree-sitter-typescript==0.23.2"]
# ///
"""Audit the attributed spec citations through source ASTs ([SPEC-TOPIC-FILES]).

Run with an environment containing the dependencies above, or with `uv run`.
Only AST comments count as rule citations. Test strings are reported separately.
The markdown is generated from current source and documentation; it does not
claim that a matching citation proves the implementation or assertion correct.
"""

import argparse
import json
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import tree_sitter
import tree_sitter_rust
import tree_sitter_typescript

SOURCE_ROOTS = ("crates", "clients/vscode/src")
COMMENT_TYPES = frozenset(("line_comment", "block_comment", "comment"))
STRING_TYPES = frozenset(("string_literal", "string"))
EXAMPLES = frozenset(("BRACKETED-ID", "SPEC-ID", "SKIP-BREAKING-CI"))
TEST_BUILD_RULES = frozenset(("TEST-ONE-BINARY",))
DEFAULT_ATTRIBUTION = "reports/regression-attribution-2026-09-07.md"
DEFAULT_OUTPUT = "reports/spec-crossrefs-2026-09-07.md"
TOP_CLUSTER_FIELDS = ("id", "kind", "mass", "occurrence_count", "occurrences_total", "rank", "rank_band")
CANONICAL = {
    "AUTOFIX-CONSOLIDATE-CODE-ACTION": ("AUTOFIX-CONSOLIDATE-SURFACE",),
    "CLONE-NOISE-EMBEDDING-ROLE": ("CLONE-NOISE-EMBEDDING-ROLE-MISMATCH",),
    "CORPUS-SCORE-GATE": ("CORPUS-SCORE",),
    "DESLOP-LIVE": ("LIVE-SCHEDULER",),
    "FACET-GROUP-BY-SEVERITY": ("FACET-GROUP-BY-KIND",),
    "FUSED-RANK-MASS": ("RANK-MASS-SUM",),
    "LSP-SEVERITY-BAND": ("LSP-SEVERITY-BUCKET",),
    "MCP-ROOT-CANONICAL": ("MCP-IPC-DISCOVERY",),
    "MCP-TOOLS-FIND-SIMILAR": ("MCP-TOOL-FINDSIMILAR",),
    "PERF-SAMPLE": ("PIPELINE-OBSERVABILITY-STAGES",),
    "RANK-SCORE": ("RANK-MASS-SUM",),
    "REPAIR-COSINE-MERGE": ("FUSED-PAIR-SIGNALS",),
    "REPAIR-RENAME-ANCHOR-MASS": ("FUSED-CONTENT-GATE",),
    "REPORTING-CONTEXT": ("REPORT-CONTEXT-CLUSTER", "CONFIG-CROSS-LANGUAGE", "OUTPUT-SCHEMA-JSON"),
    "TESTS-NO-INDEXING": (),
    "VSIX-REACTIVITY-DIRTY": ("VSIX-STATE-DIRTY",),
    "VSIX-SETTINGS-RANKING": ("RANK-STRUCTURAL-ONLY",),
}


@dataclass(frozen=True, order=True)
class Citation:
    """A comment or literal selected by its parsed node, with a stable location."""

    path: str
    line: int
    is_test: bool = False

    def link(self) -> str:
        """Render a repository-relative markdown link."""
        return f"[{self.path}:{self.line}](../{self.path}#L{self.line})"


def identifiers(text: str) -> set[str]:
    """Read bracket-delimited slugs without treating code as text patterns."""
    result = set()
    for fragment in text.split("[")[1:]:
        candidate, closing, _rest = fragment.partition("]")
        parts = candidate.split("-")
        if closing and len(parts) > 1 and all(part and part.isalnum() and part.upper() == part for part in parts):
            result.add(candidate)
    return result


def requested_ids(path: Path) -> list[str]:
    """Read identifier cells from the attribution report's markdown table."""
    result = set()
    for row in path.read_text().splitlines():
        cells = row.split("|")
        if len(cells) > 2 and cells[1].strip().startswith("`["):
            result.update(identifiers(cells[1]))
    if not result:
        raise ValueError(f"No attributed identifiers found in {path}")
    return sorted(result)


def doc_definitions() -> dict[str, list[Citation]]:
    """Index requirement headings and bold requirement bullets, not mentions."""
    result = defaultdict(list)
    for path in sorted(Path("docs").rglob("*.md")):
        fenced = False
        for line, text in enumerate(path.read_text().splitlines(), start=1):
            if text.lstrip().startswith("```"):
                fenced = not fenced
            if fenced or not text.startswith(("#", "- **[")):
                continue
            prefix, opening, suffix = text.partition("[")
            if opening and not prefix.strip("# *-"):
                for identifier in identifiers("[" + suffix.partition("]")[0] + "]"):
                    result[identifier].append(Citation(str(path), line))
    return result


def parser_set() -> dict[str, tree_sitter.Parser]:
    """Keep parsers and their languages alive for every traversal."""
    languages = {
        ".rs": tree_sitter.Language(tree_sitter_rust.language()),
        ".ts": tree_sitter.Language(tree_sitter_typescript.language_typescript()),
        ".tsx": tree_sitter.Language(tree_sitter_typescript.language_tsx()),
    }
    return {suffix: tree_sitter.Parser(language) for suffix, language in languages.items()}


def test_context(path: Path, node: tree_sitter.Node) -> bool:
    """Include integration suites and inline Rust test modules."""
    if set(path.parts) & {"tests", "test", "fixtures", "__tests__"} or test_name(path.stem):
        return True
    parent = node.parent
    while parent is not None:
        if parent.type == "mod_item":
            name = parent.child_by_field_name("name")
            if name and test_name(name.text.decode()):
                return True
        parent = parent.parent
    return False


def test_name(name: str) -> bool:
    """Recognize test names without misclassifying names such as latest_report."""
    return name in {"test", "tests"} or name.startswith("test_") or name.endswith(("_tests", "_test", ".test"))


def parsed_citations(path: Path, parser: tree_sitter.Parser):
    """Yield comment/literal references only; never search raw source."""
    data = path.read_bytes()
    tree = parser.parse(data)
    pending = [tree.root_node]
    while pending:
        node = pending.pop()
        if node.type in COMMENT_TYPES | STRING_TYPES:
            citation = Citation(str(path), node.start_point.row + 1, test_context(path, node))
            for identifier in identifiers(node.text.decode()):
                yield identifier, node.type in COMMENT_TYPES, citation
        else:
            pending.extend(reversed(node.children))


def source_citations():
    """Collect current citations independently of git's tracked-file list."""
    comments, literals = defaultdict(set), defaultdict(set)
    parsers = parser_set()
    for root in SOURCE_ROOTS:
        for path in sorted(Path(root).rglob("*")):
            if path.suffix not in parsers:
                continue
            for identifier, is_comment, citation in parsed_citations(path, parsers[path.suffix]):
                (comments if is_comment else literals)[identifier].add(citation)
    return comments, literals


def problems(identifier, canonical, definitions, comments):
    """Reject missing definitions, stale aliases, and unlinked contracts."""
    failures = []
    if identifier not in canonical and comments[identifier]:
        failures.append("old citation remains")
    for target in canonical:
        if not definitions[target]:
            failures.append(f"{target}: missing definition")
        if not any(site.is_test for site in comments[target]):
            failures.append(f"{target}: no test citation")
        if target not in TEST_BUILD_RULES and not any(not site.is_test for site in comments[target]):
            failures.append(f"{target}: no implementation citation")
    return failures


def audit_rows(requested, definitions, comments, literals):
    """Build reproducible migration rows and unresolved findings."""
    rows, failures = [], []
    for identifier in requested:
        canonical = () if identifier in EXAMPLES else CANONICAL.get(identifier, (identifier,))
        issues = problems(identifier, canonical, definitions, comments)
        sites = set().union(*(comments[target] for target in canonical))
        code = sum(not site.is_test for site in sites)
        tests = sum(site.is_test for site in sites)
        links = ", ".join(site.link() for target in canonical for site in definitions[target])
        resolution = ", ".join(f"`[{target}]`" for target in canonical) or "example/helper prose; no rule"
        status = "; ".join(issues) or "linked"
        if identifier in EXAMPLES and literals[identifier]:
            status += f"; {len(literals[identifier])} deliberate test literal(s)"
        rows.append(f"| `[{identifier}]` | {resolution} | {code} | {tests} | {status} | {links} |")
        failures.extend(f"{identifier}: {issue}" for issue in issues)
    return rows, failures


def render_report(requested, definitions, comments, literals, attribution):
    """Render scope, every result, and concrete evidence links mechanically."""
    rows, failures = audit_rows(requested, definitions, comments, literals)
    lines = ["# Spec cross-reference audit", "", "Generated by `scripts/repository/spec-crossrefs.py` from current AST comments and documentation definitions.", "",
             f"Input: `{attribution}` plus any explicit extra IDs. Requested identifiers: **{len(requested)}**. Unresolved references: **{len(failures)}**.", "",
             "Counts are citation sites, split into implementation and test context. This checks references and definitions; it does not claim that citations prove behavior. Test fixture strings are not requirements.", "",
             "| Attributed identifier | Current contract | Code citations | Test citations | Result | Definition |",
             "| --- | --- | ---: | ---: | --- | --- |", *rows, "", "## Source and test evidence", ""]
    targets = sorted({target for identifier in requested if identifier not in EXAMPLES for target in CANONICAL.get(identifier, (identifier,))})
    for target in targets:
        lines.extend((f"### [{target}]", "", ", ".join(site.link() for site in sorted(comments[target])), ""))
    if failures:
        lines.extend(("## Unresolved", "", *(f"- {item}" for item in failures), ""))
    return "\n".join(lines), failures


def validation_section(path: Path | None) -> tuple[str, bool]:
    """Render command outcomes captured by the caller's validation runner."""
    if path is None:
        return "", False
    records = json.loads(path.read_text())
    if not isinstance(records, list) or not records:
        raise ValueError("Validation manifest must contain command results")
    rows = [validation_row(record) for record in records]
    lines = ["", "## Validation", "", f"Command outcomes read from `{path}`.", "",
             "Historical attempts are retained and link to the latest recorded result for the same check. Only latest failures and pending checks remain unresolved. Pending rows have not completed. These records do not establish that `make ci` passed.", "",
             "| Result / exit code | Working directory | Command | Captured result |",
             "| --- | --- | --- | --- |", *(row for row, _failed in rows), ""]
    return "\n".join(lines), any(failed for _row, failed in rows)


def table_text(value: str) -> str:
    """Keep literal commands and log text inside a markdown table cell."""
    return value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("|", "&#124;").replace("`", "&#96;").replace("\n", " ")


def validation_row(record) -> tuple[str, bool]:
    """Render completed or explicitly pending checks without inventing an outcome."""
    command, exit_code = record["command"], record["exit_code"]
    pending = record.get("status") == "pending" and exit_code is None
    if not isinstance(command, str) or not (pending or type(exit_code) is int):
        raise ValueError("Each result needs a command and integer exit_code, or explicit pending status")
    historical = record.get("status") == "historical" and bool(record.get("superseded_by"))
    result = "pending" if pending else f"{'historical' if historical else 'latest'} / {exit_code}"
    directory = table_text(record.get("cwd", "."))
    log = record.get("log")
    evidence = f"[Output](../{log})" if log else "Awaiting recorded output"
    if log:
        summaries = [line.strip() for line in Path(log).read_text().splitlines() if line.startswith("test result:")]
        evidence += " " + table_text("; ".join(summaries))
    if historical:
        evidence += f"; superseded by [latest result](../{record['superseded_by']})"
    return f"| {result} | {directory} | <code>{table_text(command)}</code> | {evidence} |", pending or (not historical and exit_code != 0)


def scan_snapshot(path: Path) -> dict:
    """Read Rust-produced report values without calculating duplication figures."""
    snapshot = json.loads(path.read_text())
    if not isinstance(snapshot, dict) or not isinstance(snapshot.get("metrics"), dict):
        raise ValueError(f"{path}: expected a canonical report with metrics")
    if not isinstance(snapshot.get("clusters"), list):
        raise ValueError(f"{path}: expected the engine's ordered cluster list")
    return snapshot


def snapshot_lines(path: Path, label: str, snapshot: dict) -> list[str]:
    """Copy the engine's first cluster and complete metrics object verbatim in value."""
    top = next(iter(snapshot["clusters"]), None)
    fields = {key: top[key] for key in TOP_CLUSTER_FIELDS} if top is not None else None
    summary = {"tool_version": snapshot["tool_version"], "top_cluster": fields}
    return [f"### {label}", "", f"Source: [{path}](../{path})", "",
            "Engine's first published cluster (`null` when there are no clusters):", "",
            "```json", json.dumps(summary, indent=2, ensure_ascii=False), "```", "",
            "Engine metrics:", "", "```json", json.dumps(snapshot["metrics"], ensure_ascii=False), "```", ""]


def baseline_identity_lines(baseline: dict, current: dict) -> list[str]:
    """Check the exact original cluster identity without judging other findings."""
    first = next(iter(baseline["clusters"]), None)
    if first is None:
        return ["The baseline has no first cluster to track.", ""]
    original_id = first["id"]
    present = any(cluster["id"] == original_id for cluster in current["clusters"])
    return ["### Original reported cluster", "", f"Baseline first cluster: `{original_id}`.", "",
            f"That exact identifier remains in the current report: **{str(present).lower()}**.", "",
            "This is an identity-presence check; changed membership can produce a different identifier.", ""]


def scan_section(scan: Path | None, baseline: Path | None) -> str:
    """Render optional baseline/current evidence; no rank, mass, or percentage is recomputed."""
    if scan is None:
        if baseline is not None:
            raise ValueError("--baseline requires --scan")
        return ""
    current = scan_snapshot(scan)
    lines = ["", "## Recorded scan outputs", "",
             "Metrics and cluster order below come directly from Rust-produced report JSON. No duplication formula is evaluated by this report generator.", ""]
    if baseline is not None:
        previous = scan_snapshot(baseline)
        lines.extend(snapshot_lines(baseline, "Baseline", previous))
    lines.extend(snapshot_lines(scan, "Current", current))
    if baseline is not None:
        lines.extend(baseline_identity_lines(previous, current))
    return "\n".join(lines)


def arguments():
    """Read the report inputs without hard-coding additional workstream IDs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--attribution", type=Path, default=Path(DEFAULT_ATTRIBUTION))
    parser.add_argument("--output", type=Path, default=Path(DEFAULT_OUTPUT))
    parser.add_argument("--extra-id", action="append", default=[])
    parser.add_argument("--validation", type=Path)
    parser.add_argument("--scan", type=Path)
    parser.add_argument("--baseline", type=Path)
    return parser.parse_args()


def main() -> int:
    """Write one markdown artifact and fail when a reference remains broken."""
    args = arguments()
    definitions = doc_definitions()
    comments, literals = source_citations()
    requested = sorted(set(requested_ids(args.attribution)) | set(args.extra_id))
    report, failures = render_report(requested, definitions, comments, literals, args.attribution)
    validation, failed_validation = validation_section(args.validation)
    scans = scan_section(args.scan, args.baseline)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(report + scans + validation)
    print(args.output)
    return bool(failures) or failed_validation


if __name__ == "__main__":
    raise SystemExit(main())
