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
    assignees: list[RawUser]
    milestone: RawNamedValue | None
    pull_request: NotRequired[object]
    sub_issues_summary: NotRequired[RawSummary]
    issue_dependencies_summary: NotRequired[RawDependencySummary]
    sub_issue_numbers: NotRequired[list[int]]
    blocked_by_numbers: NotRequired[list[int]]
    blocking_numbers: NotRequired[list[int]]


class WorkstreamRule(TypedDict):
    name: str
    description: str
    color: str
    keywords: tuple[str, ...]


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
    priority_rank: int
    priority_name: str
    priority_reason: str
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
    release_blockers: int
    accuracy_critical: int
    linked: int
    relationships: int


class WorkstreamData(WorkstreamRule):
    id: str
    count: int
    urgent: int
    verify: int


class PriorityData(TypedDict):
    id: str
    rank: int
    name: str
    description: str
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


WORKSTREAMS: dict[str, WorkstreamRule] = {
    "accuracy": {
        "name": "Accuracy",
        "description": "False positives, false negatives, and correctness of clone claims.",
        "color": "#ff5449",
        "keywords": ("false positive", "false negative", "accuracy", "incorrect", "lies"),
    },
    "detection": {
        "name": "Detection engine",
        "description": "Parsing, matching, scoring, embeddings, and cluster formation.",
        "color": "#ff8a65",
        "keywords": ("clone", "bucket", "fingerprint", "lsh", "embedding", "tree sitter", "parser", "fused", "rename"),
    },
    "performance": {
        "name": "Performance & live",
        "description": "Incremental analysis, caches, memory, scheduling, and throughput.",
        "color": "#ffd166",
        "keywords": ("performance", "incremental", "cache", "memory", "rss", "slow", "scheduler", "reactiv", "watcher"),
    },
    "editor": {
        "name": "Editor experience",
        "description": "VS Code, JetBrains, diagnostics, panels, hovers, and webviews.",
        "color": "#9bcbff",
        "keywords": ("vsix", "vs code", "vscode", "jetbrains", "webview", "panel", "hover", "diagnostic", "editor"),
    },
    "integrations": {
        "name": "CLI, MCP & automation",
        "description": "Command-line, MCP, actions, and agent-facing workflows.",
        "color": "#8bd3c7",
        "keywords": ("mcp", "cli", "find similar", "top offenders", "github action", "autofix", "tool"),
    },
    "delivery": {
        "name": "Release & delivery",
        "description": "Packaging, signing, releases, deployment, and platform contracts.",
        "color": "#c6a0f6",
        "keywords": ("release", "shipwright", "deploy", "publish", "sign", "notar", "marketplace", "dependabot", "codeql"),
    },
    "reporting": {
        "name": "Reporting & metrics",
        "description": "Human-readable reports, metrics, summaries, and documentation.",
        "color": "#89b4fa",
        "keywords": ("report", "summary", "metric", "percentage", "documentation", "docs", "context"),
    },
    "quality": {
        "name": "Quality system",
        "description": "Tests, specifications, CI, fixtures, and repository health.",
        "color": "#a6e3a1",
        "keywords": ("test", "spec", "fixture", "coverage", "ci", "flaky", "ignored", "tech debt"),
    },
}

PRIORITIES: dict[str, tuple[int, str, str]] = {
    "verify_release": (0, "Verify next release", "To the best of our knowledge, fixed on main; verify in the next release before closing."),
    "release_blocker": (1, "Stop the line", "An unresolved showstopper blocks safe delivery."),
    "accuracy_critical": (2, "Protect accuracy", "Critical correctness risk to duplicate detection."),
    "critical": (3, "Critical path", "Serious user or delivery impact; tackle soon."),
    "assurance": (4, "Restore assurance", "A spec or ignored-test gap weakens confidence in the system."),
    "defect": (5, "Fix defects", "Open bug without a higher-severity signal."),
    "feature": (6, "Planned product work", "Feature work after correctness and release risk."),
    "task": (7, "Maintenance & tasks", "Task or unclassified work after higher-priority queues."),
}


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


def priority_for(labels: set[str], issue_type: str) -> tuple[str, int, str, str]:
    if "fixed-on-main" in labels:
        key = "verify_release"
    elif "showstopper" in labels:
        key = "release_blocker"
    elif "critical" in labels and labels & {"false-negative", "false-positive"}:
        key = "accuracy_critical"
    elif "critical" in labels:
        key = "critical"
    elif labels & {"ignored-test", "spec-violation"}:
        key = "assurance"
    else:
        key = {"Bug": "defect", "Feature": "feature"}.get(issue_type, "task")
    rank, name, reason = PRIORITIES[key]
    return key, rank, name, reason


def normalized_words(text: str) -> str:
    punctuation = "`~!@#$%^&*()_+-={}[]|\\:;\"'<>,.?/\n\r\t"
    return " ".join(text.lower().translate(str.maketrans(punctuation, " " * len(punctuation))).split())


def workstream_score(item: RawIssue, stream: WorkstreamRule) -> int:
    title = normalized_words(item.get("title", ""))
    body = normalized_words((item.get("body") or "")[:1600])
    labels = " ".join(sorted(label_names(item)))
    return sum(4 for term in stream["keywords"] if term in title) + sum(1 for term in stream["keywords"] if term in body) + sum(3 for term in stream["keywords"] if term in labels)


def workstream_for(item: RawIssue) -> str:
    labels = label_names(item)
    if labels & {"false-negative", "false-positive"}:
        return "accuracy"
    scores = {key: workstream_score(item, value) for key, value in WORKSTREAMS.items()}
    best = max(scores.items(), key=lambda entry: entry[1])[0]
    return best if scores[best] else "quality"


def plain_excerpt(body: str | None) -> str:
    lines: list[str] = []
    in_code = False
    for raw_line in (body or "").splitlines():
        if raw_line.strip().startswith("```"):
            in_code = not in_code
            continue
        line = raw_line.strip().lstrip("#>*- ").replace("`", "")
        if line and not in_code and not line.startswith("!["):
            lines.append(line)
        if sum(len(value) for value in lines) > 260:
            break
    excerpt = " ".join(lines)
    return excerpt[:277].rstrip() + "…" if len(excerpt) > 280 else excerpt


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
    labels = label_names(item)
    issue_type = issue_type_name(item)
    milestone = item.get("milestone")
    priority, rank, priority_name, priority_reason = priority_for(labels, issue_type)
    return {
        "number": item["number"], "title": item["title"], "url": item["html_url"],
        "excerpt": plain_excerpt(item.get("body", "")), "type": issue_type,
        "labels": compact_labels(item), "assignees": compact_assignees(item),
        "milestone": milestone["name"] if milestone else None,
        "created_at": item["created_at"], "updated_at": item["updated_at"],
        "lifecycle": lifecycle_for(labels), "priority": priority, "priority_rank": rank,
        "priority_name": priority_name, "priority_reason": priority_reason,
        "workstream": workstream_for(item), "inbound_links": inbound_count,
    }


def effort_for(issue: IssueData) -> int:
    if issue["lifecycle"] == "verify":
        return 2
    if issue["priority"] == "release_blocker":
        return 3
    if issue["priority"] in {"accuracy_critical", "critical"}:
        return 4
    return {"Feature": 8, "Task": 4, "Bug": 5}.get(issue["type"], 4)


def sequence_issues(issues: list[IssueData]) -> None:
    availability: defaultdict[str, list[int]] = defaultdict(lambda: [0, 0])
    for issue in issues:
        track = min(range(2), key=lambda index: availability[issue["workstream"]][index])
        offset = availability[issue["workstream"]][track]
        effort = effort_for(issue)
        availability[issue["workstream"]][track] = offset + effort
        issue["plan"] = {"offset": offset, "effort_units": effort, "track": track}


def sort_issues(issues: list[IssueData], relationships: list[RelationshipData]) -> list[IssueData]:
    by_number = {item["number"]: item for item in issues}
    remaining = set(by_number)
    blockers: defaultdict[int, set[int]] = defaultdict(set)
    for edge in relationships:
        if edge["kind"] == "blocks" and edge["source"] in remaining and edge["target"] in remaining:
            blockers[edge["target"]].add(edge["source"])
    ordered: list[IssueData] = []
    key: Callable[[IssueData], tuple[int, int, str, int]] = lambda item: (item["priority_rank"], -item["inbound_links"], item["created_at"], item["number"])
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
        "open": len(issues), "verify": priorities["verify_release"],
        "release_blockers": priorities["release_blocker"],
        "accuracy_critical": priorities["accuracy_critical"],
        "linked": len(linked), "relationships": len(relationships),
    }


def workstream_data(issues: list[IssueData]) -> list[WorkstreamData]:
    counts = Counter(item["workstream"] for item in issues)
    urgent = Counter(item["workstream"] for item in issues if item["priority_rank"] <= 3)
    verify = Counter(item["workstream"] for item in issues if item["lifecycle"] == "verify")
    return [{"id": key, **value, "count": counts[key], "urgent": urgent[key], "verify": verify[key]} for key, value in WORKSTREAMS.items()]


def priority_data(issues: list[IssueData]) -> list[PriorityData]:
    counts = Counter(item["priority"] for item in issues)
    return [{"id": key, "rank": value[0], "name": value[1], "description": value[2], "count": counts[key]} for key, value in PRIORITIES.items()]


def build_report(raw_issues: list[RawIssue], repo: str, published_at: datetime) -> ReportData:
    relationships = relationship_edges(raw_issues)
    inbound = Counter(edge["target"] for edge in relationships)
    issues = sort_issues([compact_issue(item, inbound[item["number"]]) for item in raw_issues], relationships)
    sequence_issues(issues)
    return {
        "meta": {
            "repo": repo, "published_at": published_at.isoformat().replace("+00:00", "Z"),
            "published_at_long": long_publication_time(published_at),
            "source_url": f"https://github.com/{repo}/issues",
            "method": "GitHub metadata, explicit relationships, cross-references, and documented keyword rules. No AI enrichment.",
            "planning_note": "Indicative only — not a schedule. Relative sequencing uses two parallel tracks per workstream and default effort units: verify 2, showstopper 3, critical 4, bug 5, task 4, feature 8.",
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
