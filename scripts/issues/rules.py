"""Static triage rules for the Deslop issue atlas.

Single source of truth for lane metadata and the Priority ladder. Both are
derived from GitHub data — never from AI, never from the triager's judgement:

* Lane   = the `lane/<id>` label on the issue.
* Ladder = the org-level **Priority** issue field (`gh api orgs/<org>/issue-fields`).

`PRIORITIES` mirrors that field's options: the key is the option name verbatim,
the rank is the option's own ordering, the description is its GitHub description.
Change an option in GitHub, change this table — `priority_for` rejects an option
it does not know rather than mis-ranking the atlas.

Must agree with `.agents/skills/triage/SKILL.md` and `.agents/skills/log-issue/SKILL.md`.
"""

from typing import TypedDict


class WorkstreamRule(TypedDict):
    name: str
    description: str
    color: str


class PriorityRule(TypedDict):
    rank: int
    name: str
    description: str
    color: str


LANE_LABEL_PREFIX = "lane/"
UNASSIGNED_LANE = "unassigned"

WORKSTREAMS: dict[str, WorkstreamRule] = {
    "accuracy": {
        "name": "Accuracy",
        "description": "False positives, false negatives, and correctness of clone claims.",
        "color": "#ff5449",
    },
    "detection": {
        "name": "Detection engine",
        "description": "Parsing, matching, scoring, embeddings, and cluster formation.",
        "color": "#ff8a65",
    },
    "performance": {
        "name": "Performance & live",
        "description": "Incremental analysis, caches, memory, scheduling, and throughput.",
        "color": "#ffd166",
    },
    "editor": {
        "name": "Editor experience",
        "description": "VS Code, JetBrains, diagnostics, panels, hovers, and webviews.",
        "color": "#9bcbff",
    },
    "integrations": {
        "name": "CLI, MCP & automation",
        "description": "Command-line, MCP, actions, and agent-facing workflows.",
        "color": "#8bd3c7",
    },
    "delivery": {
        "name": "Release & delivery",
        "description": "Packaging, signing, releases, deployment, and platform contracts.",
        "color": "#c6a0f6",
    },
    "reporting": {
        "name": "Reporting & metrics",
        "description": "Human-readable reports, metrics, summaries, and documentation.",
        "color": "#89b4fa",
    },
    "quality": {
        "name": "Quality system",
        "description": "Tests, specifications, CI, fixtures, and repository health.",
        "color": "#a6e3a1",
    },
    UNASSIGNED_LANE: {
        "name": "Unassigned",
        "description": "No lane/* label on the issue — triage owes it one.",
        "color": "#8e8b8a",
    },
}

PRIORITY_FIELD = "Priority"
EFFORT_FIELD = "Effort"
PRIORITY_UNSET = "unset"
URGENT_RANK = 2

PRIORITIES: dict[str, PriorityRule] = {
    "showstopper": {
        "rank": 1,
        "name": "Showstopper",
        "description": "reserved for regressions on main that haven't been released yet, or recent regressions in a release",
        "color": "#ff5449",
    },
    "critical": {
        "rank": 2,
        "name": "Critical",
        "description": "seriously impacting accuracy or the usefulness of the tool",
        "color": "#f06292",
    },
    "normal": {
        "rank": 3,
        "name": "Normal",
        "description": "a problem that impacts the usefulness of the tool",
        "color": "#ffd166",
    },
    "low": {
        "rank": 4,
        "name": "Low",
        "description": "AKA don't worry about it for now",
        "color": "#a6e3a1",
    },
    PRIORITY_UNSET: {
        "rank": 5,
        "name": "No priority set",
        "description": "No Priority value on the issue — triage owes it one.",
        "color": "#8e8b8a",
    },
}

# Effort units drive the indicative runway only. The org-level Effort field wins
# where a human set it; otherwise the issue type supplies the default.
EFFORT_UNITS: dict[str, int] = {"High": 8, "Medium": 4, "Low": 2}
TYPE_EFFORT_UNITS: dict[str, int] = {"Feature": 8, "Bug": 5, "Task": 4}
DEFAULT_EFFORT_UNITS = 4
