"""Static triage rules for the Deslop issue atlas.

Single source of truth for workstream keyword routing and priority ordering.
Must agree with `.agents/skills/triage/SKILL.md` (Step 4 documents the exact
derivation) and `.agents/skills/log-bug/SKILL.md`.
"""

from typing import TypedDict


class WorkstreamRule(TypedDict):
    name: str
    description: str
    color: str
    keywords: tuple[str, ...]


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
