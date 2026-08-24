#!/usr/bin/env python3
"""Build the Deslop issue atlas data from GitHub's issue metadata."""

import argparse
import json
import os
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable, NotRequired, TypedDict, cast


class RawLabel(TypedDict):
    name: str
    color: str
    description: str | None


class RawUser(TypedDict):
    login: str
    avatar_url: NotRequired[str]


class RawNamedValue(TypedDict):
    name: str
    color: NotRequired[str]
    description: NotRequired[str | None]


class RawSummary(TypedDict):
    total: int
    completed: NotRequired[int]
    percent_completed: NotRequired[int]


class RawDependencySummary(TypedDict):
    blocked_by: NotRequired[int]
    blocking: NotRequired[int]
    total_blocked_by: NotRequired[int]
    total_blocking: NotRequired[int]


class RawFieldOption(TypedDict):
    id: int
    name: str
    color: str


class RawFieldValue(TypedDict):
    issue_field_name: str
    data_type: str
    single_select_option: NotRequired[RawFieldOption | None]


class RawIssue(TypedDict):
    number: int
    title: str
    body: str | None
    html_url: str
    created_at: str
    updated_at: str
    state: NotRequired[str]
    labels: list[RawLabel]
    type: RawNamedValue | None
    issue_field_values: NotRequired[list[RawFieldValue]]
    assignees: list[RawUser]
    milestone: RawNamedValue | None
    pull_request: NotRequired[object]
    sub_issues_summary: NotRequired[RawSummary]
    issue_dependencies_summary: NotRequired[RawDependencySummary]
    sub_issue_numbers: NotRequired[list[int]]
    blocked_by_numbers: NotRequired[list[int]]
    blocking_numbers: NotRequired[list[int]]


from scripts.issues.rules import (
    DEFAULT_EFFORT_UNITS,
    EFFORT_FIELD,
    EFFORT_UNITS,
    LANE_LABEL_PREFIX,
    PRIORITIES,
    PRIORITY_FIELD,
    PRIORITY_UNSET,
    PriorityRule,
    TYPE_EFFORT_UNITS,
    UNASSIGNED_LANE,
    URGENT_RANK,
    WORKSTREAMS,
    WorkstreamRule,
)


class LabelData(TypedDict):
    name: str
    color: str
    description: str


class AssigneeData(TypedDict):
    login: str
    avatar: str


class PlanData(TypedDict):
    offset: int
    effort_units: int
    track: int


class IssueData(TypedDict):
    number: int
    title: str
    url: str
    excerpt: str
    type: str
    labels: list[LabelData]
    assignees: list[AssigneeData]
    milestone: str | None
    created_at: str
    updated_at: str
    lifecycle: str
    priority: str
    workstream: str
    inbound_links: int
    plan: NotRequired[PlanData]


class RelationshipData(TypedDict):
    source: int
    target: int
    kind: str


class SummaryData(TypedDict):
    open: int
    verify: int
    showstoppers: int
    critical: int
    linked: int
    relationships: int


class WorkstreamData(WorkstreamRule):
    id: str
    count: int
    urgent: int
    verify: int


class PriorityData(PriorityRule):
    id: str
    count: int


class MetaData(TypedDict):
    repo: str
    published_at: str
    published_at_long: str
    source_url: str
    method: str
    planning_note: str

class ReportData(TypedDict):
    meta: MetaData
    summary: SummaryData
    workstreams: list[WorkstreamData]
    priorities: list[PriorityData]
    issues: list[IssueData]
    relationships: list[RelationshipData]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", "Nimblesite/Deslop"))
    parser.add_argument("--output", default="site/src/assets/data/issues.json")
    parser.add_argument("--input", help="Read GitHub REST issue JSON from disk instead of calling gh.")
    parser.add_argument("--published-at", "--generated-at", dest="published_at", help="Override the publication instant (ISO 8601; defaults to current UTC).")
    return parser.parse_args()


def gh_json(endpoint: str) -> list[RawIssue]:
    command = ["gh", "api", "--paginate", endpoint]
    result = subprocess.run(command, check=True, capture_output=True, text=True)
    return cast(list[RawIssue], json.loads(result.stdout))


def relation_numbers(items: list[RawIssue]) -> list[int]:
    return [item["number"] for item in items if item.get("state") == "open"]


def populate_relationships(item: RawIssue, repo: str) -> None:
    number = item["number"]
    sub_count = item.get("sub_issues_summary", {}).get("total", 0)
    dependency = item.get("issue_dependencies_summary", {})
    item["sub_issue_numbers"] = relation_numbers(gh_json(f"repos/{repo}/issues/{number}/sub_issues")) if sub_count else []
    item["blocked_by_numbers"] = relation_numbers(gh_json(f"repos/{repo}/issues/{number}/dependencies/blocked_by")) if dependency.get("blocked_by") else []
    item["blocking_numbers"] = relation_numbers(gh_json(f"repos/{repo}/issues/{number}/dependencies/blocking")) if dependency.get("blocking") else []


def fetch_issues(repo: str) -> list[RawIssue]:
    items = gh_json(f"repos/{repo}/issues?state=open&per_page=100")
    issues = [item for item in items if "pull_request" not in item]
    for item in issues:
        populate_relationships(item, repo)
    return issues


def load_issues(path: str | None, repo: str) -> list[RawIssue]:
    if path:
        return cast(list[RawIssue], json.loads(Path(path).read_text(encoding="utf-8")))
    return fetch_issues(repo)


def extract_references(body: str | None, open_numbers: set[int], own_number: int) -> list[int]:
    references: set[int] = set()
    text = body or ""
    for index, character in enumerate(text):
        if character != "#" or index + 1 >= len(text) or not text[index + 1].isdigit():
            continue
        end = index + 1
        while end < len(text) and text[end].isdigit():
            end += 1
        number = int(text[index + 1 : end])
        if number in open_numbers and number != own_number:
            references.add(number)
    return sorted(references)


def label_names(item: RawIssue) -> set[str]:
    return {label["name"].lower() for label in item.get("labels", [])}


def issue_type_name(item: RawIssue) -> str:
    issue_type = item.get("type")
    return issue_type["name"] if issue_type else "Unclassified"


def lifecycle_for(labels: set[str]) -> str:
    if "fixed-on-main" in labels:
        return "verify"
    return "active"


def field_option(item: RawIssue, field: str) -> str | None:
    """Name of the single-select option set on one GitHub issue field."""
    for value in item.get("issue_field_values") or []:
        option = value.get("single_select_option") if value.get("issue_field_name") == field else None
        if option:
            return option["name"]
    return None


def priority_for(item: RawIssue) -> str:
    """The issue's Priority field option. Unknown options are a rules drift, not a default."""
    option = field_option(item, PRIORITY_FIELD)
    if option is None:
        return PRIORITY_UNSET
    if option not in PRIORITIES:
        raise ValueError(f"issue #{item['number']}: unknown {PRIORITY_FIELD} option {option!r} — update scripts/issues/rules.py")
    return option


def priority_rank(issue: IssueData) -> int:
    return PRIORITIES[issue["priority"]]["rank"]


def workstream_for(item: RawIssue) -> str:
    """The issue's lane, taken from its `lane/<id>` label."""
    lanes = {name.removeprefix(LANE_LABEL_PREFIX) for name in label_names(item) if name.startswith(LANE_LABEL_PREFIX)}
    return next((lane for lane in WORKSTREAMS if lane in lanes), UNASSIGNED_LANE)


# Canonical TL;DR section marker — matches `## TL;DR` from the log-bug skill and
# `### TL;DR` from the GitHub issue form (.github/ISSUE_TEMPLATE/issue.yml).
TLDR_HEADING = "tldr"
EXCERPT_SOFT_LIMIT = 260
EXCERPT_HARD_LIMIT = 280


def readable_line(raw_line: str) -> str:
    """Strip markdown decoration from one body line."""
    return raw_line.strip().lstrip("#>*- ").replace("`", "")


def normalized_heading(line: str) -> str:
    """Alphanumeric-only, lowercased heading text (``## TL;DR`` -> ``tldr``)."""
    return "".join(character for character in line.lstrip("#").strip().lower() if character.isalnum())


def tldr_lines(body: str | None) -> list[str] | None:
    """Readable lines of the canonical TL;DR section, or None when absent."""
    collected: list[str] | None = None
    in_code = False
    for raw_line in (body or "").splitlines():
        stripped = raw_line.strip()
        if stripped.startswith("```"):
            in_code = not in_code
            continue
        if in_code:
            continue
        is_heading = stripped.startswith("#")
        if collected is not None and is_heading:
            break
        if is_heading:
            if normalized_heading(stripped) == TLDR_HEADING:
                collected = []
            continue
        line = readable_line(raw_line)
        if collected is not None and line and not line.startswith("!["):
            collected.append(line)
    return collected


def body_lines(body: str | None) -> list[str]:
    """Readable lines from the whole body, capped near the excerpt limit."""
    lines: list[str] = []
    in_code = False
    for raw_line in (body or "").splitlines():
        if raw_line.strip().startswith("```"):
            in_code = not in_code
            continue
        line = readable_line(raw_line)
        if line and not in_code and not line.startswith("!["):
            lines.append(line)
        if sum(len(value) for value in lines) > EXCERPT_SOFT_LIMIT:
            break
    return lines


def plain_excerpt(body: str | None) -> str:
    """Excerpt for the issue atlas: the TL;DR section when the canonical
    structure is present, otherwise the leading readable body lines."""
    lines = tldr_lines(body)
    if lines is None:
        lines = body_lines(body)
    excerpt = " ".join(lines)
    return excerpt[:EXCERPT_HARD_LIMIT - 3].rstrip() + "…" if len(excerpt) > EXCERPT_HARD_LIMIT else excerpt


def relationship_edges(issues: list[RawIssue]) -> list[RelationshipData]:
    open_numbers = {item["number"] for item in issues}
    edges: set[tuple[int, int, str]] = set()
    for item in issues:
        source = item["number"]
        for target in item.get("sub_issue_numbers", []):
            if target in open_numbers:
                edges.add((source, target, "sub_issue"))
        for target in item.get("blocked_by_numbers", []):
            if target in open_numbers:
                edges.add((target, source, "blocks"))
        for target in item.get("blocking_numbers", []):
            if target in open_numbers:
                edges.add((source, target, "blocks"))
        for target in extract_references(item.get("body", ""), open_numbers, source):
            edges.add((source, target, "reference"))
    return [{"source": source, "target": target, "kind": kind} for source, target, kind in sorted(edges)]


def compact_labels(item: RawIssue) -> list[LabelData]:
    return [{"name": label["name"], "color": label["color"], "description": label.get("description") or ""} for label in item.get("labels", [])]


def compact_assignees(item: RawIssue) -> list[AssigneeData]:
    return [{"login": user["login"], "avatar": user.get("avatar_url", "")} for user in item.get("assignees", [])]


def compact_issue(item: RawIssue, inbound_count: int) -> IssueData:
    milestone = item.get("milestone")
    return {
        "number": item["number"], "title": item["title"], "url": item["html_url"],
        "excerpt": plain_excerpt(item.get("body", "")), "type": issue_type_name(item),
        "labels": compact_labels(item), "assignees": compact_assignees(item),
        "milestone": milestone["name"] if milestone else None,
        "created_at": item["created_at"], "updated_at": item["updated_at"],
        "lifecycle": lifecycle_for(label_names(item)), "priority": priority_for(item),
        "workstream": workstream_for(item), "inbound_links": inbound_count,
    }


def effort_for(item: RawIssue) -> int:
    """Runway units: the Effort field where a human set it, else the type default."""
    option = field_option(item, EFFORT_FIELD)
    if option in EFFORT_UNITS:
        return EFFORT_UNITS[option]
    return TYPE_EFFORT_UNITS.get(issue_type_name(item), DEFAULT_EFFORT_UNITS)


def sequence_issues(issues: list[IssueData], relationships: list[RelationshipData], efforts: dict[int, int]) -> None:
    """Lay issues onto two parallel tracks per workstream.

    `sort_issues` already guarantees a blocker is ordered before the work it
    blocks, but ordering alone does not sequence it: the blocked issue simply
    took the other track and started at the same offset, so the runway drew
    dependent work running alongside its own prerequisite. An issue therefore
    cannot start before every blocker it is waiting on has finished, whichever
    track or workstream that blocker landed in.
    """
    availability: defaultdict[str, list[int]] = defaultdict(lambda: [0, 0])
    blockers: defaultdict[int, set[int]] = defaultdict(set)
    for edge in relationships:
        if edge["kind"] == "blocks":
            blockers[edge["target"]].add(edge["source"])
    finish: dict[int, int] = {}
    for issue in issues:
        ready = max((finish[number] for number in blockers[issue["number"]] if number in finish), default=0)
        track = min(range(2), key=lambda index: max(availability[issue["workstream"]][index], ready))
        offset = max(availability[issue["workstream"]][track], ready)
        effort = efforts[issue["number"]]
        availability[issue["workstream"]][track] = offset + effort
        finish[issue["number"]] = offset + effort
        issue["plan"] = {"offset": offset, "effort_units": effort, "track": track}


def sort_issues(issues: list[IssueData], relationships: list[RelationshipData]) -> list[IssueData]:
    by_number = {item["number"]: item for item in issues}
    remaining = set(by_number)
    blockers: defaultdict[int, set[int]] = defaultdict(set)
    for edge in relationships:
        if edge["kind"] == "blocks" and edge["source"] in remaining and edge["target"] in remaining:
            blockers[edge["target"]].add(edge["source"])
    ordered: list[IssueData] = []
    key: Callable[[IssueData], tuple[int, int, str, int]] = lambda item: (priority_rank(item), -item["inbound_links"], item["created_at"], item["number"])
    while remaining:
        ready = [by_number[number] for number in remaining if not blockers[number] & remaining]
        selected = min(ready or [by_number[number] for number in remaining], key=key)
        ordered.append(selected)
        remaining.remove(selected["number"])
    return ordered


def summarize(issues: list[IssueData], relationships: list[RelationshipData]) -> SummaryData:
    priorities = Counter(item["priority"] for item in issues)
    linked = {edge["source"] for edge in relationships} | {edge["target"] for edge in relationships}
    return {
        "open": len(issues), "verify": sum(1 for item in issues if item["lifecycle"] == "verify"),
        "showstoppers": priorities["showstopper"], "critical": priorities["critical"],
        "linked": len(linked), "relationships": len(relationships),
    }


def workstream_data(issues: list[IssueData]) -> list[WorkstreamData]:
    counts = Counter(item["workstream"] for item in issues)
    urgent = Counter(item["workstream"] for item in issues if priority_rank(item) <= URGENT_RANK)
    verify = Counter(item["workstream"] for item in issues if item["lifecycle"] == "verify")
    return [{"id": key, **value, "count": counts[key], "urgent": urgent[key], "verify": verify[key]} for key, value in WORKSTREAMS.items()]


def priority_data(issues: list[IssueData]) -> list[PriorityData]:
    counts = Counter(item["priority"] for item in issues)
    return [{"id": key, **value, "count": counts[key]} for key, value in PRIORITIES.items()]


def build_report(raw_issues: list[RawIssue], repo: str, published_at: datetime) -> ReportData:
    relationships = relationship_edges(raw_issues)
    inbound = Counter(edge["target"] for edge in relationships)
    issues = sort_issues([compact_issue(item, inbound[item["number"]]) for item in raw_issues], relationships)
    sequence_issues(issues, relationships, {item["number"]: effort_for(item) for item in raw_issues})
    return {
        "meta": {
            "repo": repo, "published_at": published_at.isoformat().replace("+00:00", "Z"),
            "published_at_long": long_publication_time(published_at),
            "source_url": f"https://github.com/{repo}/issues",
            "method": "GitHub metadata: the Priority field, lane/* labels, explicit relationships, and cross-references. No AI enrichment.",
            "planning_note": "Indicative only — not a schedule. Relative sequencing uses two parallel tracks per workstream, holds blocked work until every blocker finishes, and takes effort units from the Effort field (high 8, medium 4, low 2) or, when it is unset, the issue type: feature 8, bug 5, task 4.",
        },
        "summary": summarize(issues, relationships), "workstreams": workstream_data(issues),
        "priorities": priority_data(issues), "issues": issues, "relationships": relationships,
    }


def long_publication_time(value: datetime) -> str:
    suffix = "th" if value.day % 100 in (11, 12, 13) else {1: "st", 2: "nd", 3: "rd"}.get(value.day % 10, "th")
    hour = value.strftime("%I%p").lstrip("0").lower()
    return f"{value.day}{suffix} of {value.strftime('%B %Y')}, {hour}"

def report_time(value: str | None) -> datetime:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00")) if value else datetime.now(timezone.utc)
    return parsed.replace(tzinfo=parsed.tzinfo or timezone.utc).astimezone(timezone.utc).replace(microsecond=0)

def write_report(report: ReportData, output: str) -> None:
    path = Path(output)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    temporary.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def main() -> int:
    args = parse_args()
    try:
        issues = load_issues(args.input, args.repo)
        report = build_report(issues, args.repo, report_time(args.published_at))
        write_report(report, args.output)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"issue report generation failed: {error}", file=sys.stderr)
        return 1
    print(f"Generated {args.output} from {len(issues)} open issues in {args.repo}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
