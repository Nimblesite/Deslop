import ast
import json
import unittest
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable
from unittest.mock import MagicMock, patch

from scripts.issues.generate_issue_report import (
    RawFieldValue,
    RawIssue,
    build_report,
    effort_for,
    extract_references,
    lifecycle_for,
    long_publication_time,
    plain_excerpt,
    populate_relationships,
    priority_for,
    workstream_for,
)
from scripts.issues.rules import EFFORT_FIELD, PRIORITY_FIELD, PRIORITY_UNSET, UNASSIGNED_LANE

PUBLISHED_AT = datetime(2026, 8, 21, 10, 0, tzinfo=timezone.utc)
REPO = "Nimblesite/Deslop"

# Canonical issue section headings — must agree with `.agents/skills/log-issue/SKILL.md`
# and `.github/ISSUE_TEMPLATE/issue.yml`.
HEADING_TLDR_AGENT = "## TL;DR"
HEADING_TLDR_FORM = "### TL;DR"
HEADING_DETAILS = "## Details"
HEADING_DETAILS_AI = "## Details (for AI)"
HEADING_ACCEPTANCE_AI = "## Acceptance Criteria (for AI)"

TLDR_SUMMARY = "Getter pair reported as one cluster across two languages."
DETAILS_NARRATIVE = "Long human narrative that must never reach the excerpt. " * 40

# The org-level Priority field options — must agree with `gh api orgs/Nimblesite/issue-fields`.
SHOWSTOPPER = "showstopper"
CRITICAL = "critical"
NORMAL = "normal"
LOW = "low"

EFFORT_HIGH = "High"
EFFORT_MEDIUM = "Medium"
EFFORT_LOW = "Low"

LABEL_FIXED_ON_MAIN = "fixed-on-main"
LABEL_FALSE_NEGATIVE = "false-negative"
LANE_ACCURACY = "lane/accuracy"
LANE_QUALITY = "lane/quality"
LANE_DETECTION = "lane/detection"

TYPE_BUG = "Bug"
TYPE_TASK = "Task"
TYPE_FEATURE = "Feature"


def field_values(priority: str | None, effort: str | None) -> list[RawFieldValue]:
    chosen = {PRIORITY_FIELD: priority, EFFORT_FIELD: effort}
    return [
        {
            "issue_field_name": name,
            "data_type": "single_select",
            "single_select_option": {"id": index, "name": option, "color": "red"},
        }
        for index, (name, option) in enumerate(chosen.items())
        if option is not None
    ]


def issue(
    number: int,
    title: str,
    labels: Iterable[str] = (),
    body: str = "",
    issue_type: str = TYPE_BUG,
    sub_issues: Iterable[int] = (),
    priority: str | None = None,
    effort: str | None = None,
) -> RawIssue:
    return {
        "number": number,
        "title": title,
        "body": body,
        "html_url": f"https://github.com/{REPO}/issues/{number}",
        "created_at": "2026-08-01T00:00:00Z",
        "updated_at": "2026-08-02T00:00:00Z",
        "labels": [
            {"name": name, "color": "ff0000", "description": f"{name} label"}
            for name in labels
        ],
        "type": {"name": issue_type, "color": "blue", "description": ""},
        "issue_field_values": field_values(priority, effort),
        "assignees": [],
        "milestone": None,
        "sub_issue_numbers": list(sub_issues),
        "blocked_by_numbers": [],
        "blocking_numbers": [],
    }


class IssueReportTests(unittest.TestCase):
    def test_generator_has_no_ai_integration(self) -> None:
        sources = {
            path: (Path(__file__).parent / path).read_text(encoding="utf-8")
            for path in ("generate_issue_report.py", "rules.py")
        }
        allowed = {"argparse", "collections", "datetime", "json", "os", "pathlib", "scripts", "subprocess", "sys", "typing"}
        for path, source in sources.items():
            tree = ast.parse(source)
            imported = {node.names[0].name.split(".")[0] for node in ast.walk(tree) if isinstance(node, ast.Import)}
            imported |= {node.module.split(".")[0] for node in ast.walk(tree) if isinstance(node, ast.ImportFrom) and node.module}
            self.assertLessEqual(imported, allowed, path)
            for provider in ("openai", "anthropic", "gemini", "ollama"):
                self.assertNotIn(provider, source.lower(), path)

    def test_pages_deploy_always_generates_fresh_issue_data(self) -> None:
        root = Path(__file__).resolve().parents[2]
        package = json.loads((root / "site/package.json").read_text(encoding="utf-8"))
        workflow = (root / ".github/workflows/deploy-pages.yml").read_text(encoding="utf-8")

        self.assertEqual(package["scripts"]["issues:generate"], "python3 ../scripts/issues/generate_issue_report.py --output src/assets/data/issues.json")
        self.assertEqual(package["scripts"]["build"], "npm run issues:generate && npx @11ty/eleventy")
        self.assertIn("issues: read", workflow)
        build_step = workflow.split("- name: Build site", 1)[1].split("- uses: actions/configure-pages", 1)[0]
        self.assertIn("working-directory: site", build_step)
        self.assertIn("GITHUB_TOKEN: ${{ github.token }}", build_step)
        self.assertIn("run: npm run build", build_step)
        self.assertLess(workflow.index("- name: Build site"), workflow.index("actions/upload-pages-artifact"))

    def test_site_ci_enforces_issue_report_browser_contract(self) -> None:
        root = Path(__file__).resolve().parents[2]
        workflow = (root / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        site_job = workflow.split("\n  site:", 1)[1].split("\n  security:", 1)[0]

        self.assertIn("npx playwright install --with-deps chromium", site_job)
        self.assertIn("run: npm test -- --workers=1", site_job)

    @patch("scripts.issues.generate_issue_report.gh_json")
    def test_rest_dependency_summary_fetches_native_open_edges(self, mock_gh_json: MagicMock) -> None:
        item = issue(20, "Dependent work")
        item["issue_dependencies_summary"] = {
            "blocked_by": 1,
            "blocking": 1,
            "total_blocked_by": 1,
            "total_blocking": 1,
        }
        blocker = issue(10, "Open blocker")
        blocker["state"] = "open"
        blocked = issue(30, "Open blocked work")
        blocked["state"] = "open"
        mock_gh_json.side_effect = [[blocker], [blocked]]

        populate_relationships(item, REPO)

        self.assertEqual(item.get("blocked_by_numbers"), [10])
        self.assertEqual(item.get("blocking_numbers"), [30])
        report = build_report([blocker, item, blocked], REPO, PUBLISHED_AT)
        self.assertIn({"source": 10, "target": 20, "kind": "blocks"}, report["relationships"])
        self.assertIn({"source": 20, "target": 30, "kind": "blocks"}, report["relationships"])

    def test_references_are_unique_open_issue_numbers(self) -> None:
        body = "Related: #12, #12 and Nimblesite/Deslop#13; not #99."
        self.assertEqual(extract_references(body, {12, 13}, 13), [12])

    def test_priority_is_read_from_the_github_priority_field(self) -> None:
        for option in (SHOWSTOPPER, CRITICAL, NORMAL, LOW):
            self.assertEqual(priority_for(issue(1, "Field-driven", priority=option)), option)
        self.assertEqual(priority_for(issue(2, "Nothing set")), PRIORITY_UNSET)

        report = build_report(
            [
                issue(1, "Normal work", priority=NORMAL),
                issue(2, "Do not worry", priority=LOW),
                issue(3, "Regression on main", priority=SHOWSTOPPER),
                issue(4, "Accuracy risk", priority=CRITICAL),
                issue(5, "Never triaged"),
            ],
            REPO,
            PUBLISHED_AT,
        )

        self.assertEqual([item["number"] for item in report["issues"]], [3, 4, 1, 2, 5])
        self.assertEqual([item["priority"] for item in report["issues"]], [SHOWSTOPPER, CRITICAL, NORMAL, LOW, PRIORITY_UNSET])
        self.assertEqual(report["summary"]["showstoppers"], 1)
        self.assertEqual(report["summary"]["critical"], 1)
        ladder = {entry["id"]: entry for entry in report["priorities"]}
        self.assertEqual([entry["rank"] for entry in report["priorities"]], [1, 2, 3, 4, 5])
        self.assertEqual(ladder[SHOWSTOPPER]["name"], "Showstopper")
        self.assertEqual(ladder[SHOWSTOPPER]["description"], "reserved for regressions on main that haven't been released yet, or recent regressions in a release")
        self.assertEqual(ladder[CRITICAL]["description"], "seriously impacting accuracy or the usefulness of the tool")
        self.assertEqual(ladder[NORMAL]["description"], "a problem that impacts the usefulness of the tool")
        self.assertEqual(ladder[LOW]["description"], "AKA don't worry about it for now")
        self.assertEqual(ladder[PRIORITY_UNSET]["count"], 1)
        self.assertEqual([entry["count"] for entry in report["priorities"]], [1, 1, 1, 1, 1])
        for entry in report["priorities"]:
            self.assertTrue(entry["color"].startswith("#"), entry["id"])

    def test_unknown_priority_option_fails_the_report(self) -> None:
        with self.assertRaises(ValueError) as raised:
            priority_for(issue(7, "Renamed option", priority="blocker"))

        self.assertIn("blocker", str(raised.exception))
        self.assertIn("rules.py", str(raised.exception))

    def test_deleted_severity_labels_no_longer_set_priority(self) -> None:
        """`showstopper`/`critical` are Priority-field options, not labels."""
        item = issue(1, "Label-only issue", (SHOWSTOPPER, CRITICAL), priority=NORMAL)

        self.assertEqual(priority_for(item), NORMAL)
        self.assertEqual(priority_for(issue(2, "Label-only issue", (SHOWSTOPPER,))), PRIORITY_UNSET)

    def test_fixed_on_main_is_a_verification_lifecycle_not_a_priority(self) -> None:
        labels = {LABEL_FIXED_ON_MAIN}
        self.assertEqual(lifecycle_for(labels), "verify")
        self.assertEqual(lifecycle_for(set()), "active")

        report = build_report([issue(1, "Believed fixed", labels, priority=CRITICAL)], REPO, PUBLISHED_AT)

        self.assertEqual(report["issues"][0]["lifecycle"], "verify")
        self.assertEqual(report["issues"][0]["priority"], CRITICAL)
        self.assertEqual(report["summary"]["verify"], 1)

    def test_lane_label_decides_the_workstream(self) -> None:
        self.assertEqual(workstream_for(issue(1, "VSIX panel misses a clone", (LANE_ACCURACY,))), "accuracy")
        self.assertEqual(workstream_for(issue(2, "Report percentages drift", (LANE_QUALITY,))), "quality")
        self.assertEqual(workstream_for(issue(3, "No lane label", (LABEL_FALSE_NEGATIVE,))), UNASSIGNED_LANE)

        report = build_report(
            [issue(1, "Clustering gap", (LANE_DETECTION,), priority=CRITICAL), issue(2, "Untriaged")],
            REPO,
            PUBLISHED_AT,
        )
        lanes = {entry["id"]: entry for entry in report["workstreams"]}

        self.assertEqual(lanes["detection"]["count"], 1)
        self.assertEqual(lanes["detection"]["urgent"], 1)
        self.assertEqual(lanes[UNASSIGNED_LANE]["count"], 1)
        self.assertEqual(lanes[UNASSIGNED_LANE]["urgent"], 0)
        self.assertEqual(lanes[UNASSIGNED_LANE]["name"], "Unassigned")

    def test_report_builds_relationships_and_indicative_sequence(self) -> None:
        issues = [
            issue(10, "Parent pipeline work", priority=CRITICAL, sub_issues=(11,)),
            issue(11, "Cache implementation", body="Related to #12", priority=NORMAL),
            issue(12, "Release verification", (LABEL_FIXED_ON_MAIN,), priority=SHOWSTOPPER),
        ]

        report = build_report(issues, REPO, PUBLISHED_AT)

        self.assertEqual(report["summary"]["open"], 3)
        self.assertEqual(report["summary"]["verify"], 1)
        self.assertEqual(report["issues"][0]["number"], 12)
        self.assertEqual(report["issues"][0]["lifecycle"], "verify")
        first_issue = report["issues"][0]
        self.assertIn("plan", first_issue)
        plan = first_issue.get("plan", {})
        self.assertEqual(plan["offset"], 0)
        self.assertEqual(plan["effort_units"], 5)
        self.assertNotIn("start", plan)
        self.assertNotIn("end", plan)
        self.assertIn("not a schedule", report["meta"]["planning_note"].lower())
        self.assertIn(
            {"source": 10, "target": 11, "kind": "sub_issue"},
            report["relationships"],
        )
        self.assertIn(
            {"source": 11, "target": 12, "kind": "reference"},
            report["relationships"],
        )

    def test_planner_orders_blockers_before_blocked_work_without_discarding_priority(self) -> None:
        blocker = issue(10, "Prerequisite task", issue_type=TYPE_TASK, priority=NORMAL)
        blocker["blocking_numbers"] = [20]
        blocked = issue(20, "Release verification", (LABEL_FIXED_ON_MAIN,), priority=CRITICAL)
        unrelated = issue(30, "Independent regression", priority=SHOWSTOPPER)

        report = build_report([blocked, blocker, unrelated], REPO, PUBLISHED_AT)

        self.assertEqual([item["number"] for item in report["issues"]], [30, 10, 20])
        self.assertIn(
            {"source": 10, "target": 20, "kind": "blocks"},
            report["relationships"],
        )

    def test_runway_starts_blocked_work_after_its_blocker_finishes(self) -> None:
        blocker = issue(10, "Prerequisite task", issue_type=TYPE_TASK, priority=NORMAL)
        blocker["blocking_numbers"] = [20]
        blocked = issue(20, "Dependent release verification", (LABEL_FIXED_ON_MAIN,), priority=CRITICAL)

        report = build_report([blocked, blocker], REPO, PUBLISHED_AT)
        plans = {item["number"]: item.get("plan", {}) for item in report["issues"]}
        blocker_finish = plans[10]["offset"] + plans[10]["effort_units"]

        self.assertGreaterEqual(plans[20]["offset"], blocker_finish)

    def test_effort_comes_from_the_effort_field_then_the_issue_type(self) -> None:
        issues = [
            issue(1, "Explicitly large", effort=EFFORT_HIGH, issue_type=TYPE_TASK),
            issue(2, "Explicitly middling", effort=EFFORT_MEDIUM, issue_type=TYPE_FEATURE),
            issue(3, "Explicitly small", effort=EFFORT_LOW, issue_type=TYPE_BUG),
            issue(4, "Bug default", priority=SHOWSTOPPER),
            issue(5, "Feature default", issue_type=TYPE_FEATURE),
            issue(6, "Task default", issue_type=TYPE_TASK),
        ]

        for item in issues:
            self.assertIsInstance(effort_for(item), int)
        report = build_report(issues, REPO, PUBLISHED_AT)
        efforts = {item["number"]: item.get("plan", {})["effort_units"] for item in report["issues"]}

        self.assertEqual(efforts, {1: 8, 2: 4, 3: 2, 4: 5, 5: 8, 6: 4})
        for entry in report["issues"]:
            self.assertEqual(set(entry.get("plan", {})), {"offset", "effort_units", "track"})
        self.assertIn("not a schedule", report["meta"]["planning_note"].lower())
        self.assertIn("Priority field", report["meta"]["method"])
        self.assertIn("No AI enrichment", report["meta"]["method"])
        repeated = build_report(issues, REPO, PUBLISHED_AT)
        self.assertEqual(report["issues"], repeated["issues"])

    def test_excerpt_prefers_canonical_tldr_section(self) -> None:
        body = "\n".join([
            HEADING_TLDR_AGENT,
            TLDR_SUMMARY,
            "",
            HEADING_DETAILS,
            DETAILS_NARRATIVE,
            "",
            HEADING_DETAILS_AI,
            "cluster id: 123",
        ])

        self.assertEqual(plain_excerpt(body), TLDR_SUMMARY)

    def test_excerpt_reads_form_heading_levels_and_stops_at_next_section(self) -> None:
        body = "\n".join([HEADING_TLDR_FORM, "Form-filed summary.", "", HEADING_DETAILS, "Ignored narrative."])

        self.assertEqual(plain_excerpt(body), "Form-filed summary.")

    def test_excerpt_falls_back_to_plain_body_without_tldr(self) -> None:
        self.assertEqual(plain_excerpt("Plain old body line."), "Plain old body line.")
        self.assertEqual(plain_excerpt(None), "")

    def test_excerpt_skips_code_fences_inside_tldr_section(self) -> None:
        body = "\n".join([HEADING_TLDR_AGENT, "```", "deslop . --output /tmp/deslop", "```", "Real summary."])

        self.assertEqual(plain_excerpt(body), "Real summary.")

    def test_publication_timestamp_is_full_utc_and_long_formatted(self) -> None:
        published_at = datetime(2026, 9, 14, 22, 0, tzinfo=timezone.utc)
        report = build_report([], REPO, published_at)

        self.assertEqual(report["meta"]["published_at"], "2026-09-14T22:00:00Z")
        self.assertEqual(report["meta"]["published_at_long"], "14th of September 2026, 10pm")
        expected = {1: "1st", 2: "2nd", 3: "3rd", 4: "4th", 11: "11th", 12: "12th", 13: "13th"}
        for day, ordinal in expected.items():
            value = datetime(2026, 9, day, 22, 0, tzinfo=timezone.utc)
            self.assertTrue(long_publication_time(value).startswith(ordinal))


if __name__ == "__main__":
    unittest.main()
