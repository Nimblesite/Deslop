#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["tree-sitter==0.25.2", "tree-sitter-rust==0.24.2", "tree-sitter-typescript==0.23.2"]
# ///
"""Refuse a rule identifier that code or tests cite and no document defines
([SPEC-ID-GATE]).

Run from the repository root, in the lint gate. Exits non-zero and names every
orphan with its citation sites, so an invented identifier cannot merge without
the written rule it points at.

    uv run scripts/repository/spec-id-gate.py
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from spec_ids import doc_definitions, source_citations  # noqa: E402

MAX_SITES_PRINTED = 6


def orphans() -> dict[str, list]:
    """Identifiers cited in a source comment that no document defines."""
    definitions = doc_definitions()
    comments, _literals = source_citations()
    return {
        identifier: sorted(sites)
        for identifier, sites in sorted(comments.items())
        if not definitions.get(identifier)
    }


def report(found: dict[str, list]) -> str:
    """Name every orphan and where it is cited, newest information first."""
    if not found:
        return "spec-id gate: every cited rule identifier resolves to a document."
    lines = [
        f"spec-id gate: {len(found)} rule identifier(s) are cited in code or tests "
        "and defined in no document under docs/.",
        "",
        "Give each one a section carrying its identifier, rename the citation to the "
        "identifier it meant, or delete the citation with the code it describes.",
        "",
    ]
    for identifier, sites in found.items():
        code = sum(not site.is_test for site in sites)
        tests = sum(site.is_test for site in sites)
        lines.append(f"  [{identifier}] — {code} code site(s), {tests} test site(s)")
        for site in sites[:MAX_SITES_PRINTED]:
            lines.append(f"      {site.path}:{site.line}")
        if len(sites) > MAX_SITES_PRINTED:
            lines.append(f"      … {len(sites) - MAX_SITES_PRINTED} more")
    return "\n".join(lines)


def main() -> int:
    """Print the verdict and fail the gate when anything is unresolved."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    found = orphans()
    print(report(found))
    return 1 if found else 0


if __name__ == "__main__":
    raise SystemExit(main())
