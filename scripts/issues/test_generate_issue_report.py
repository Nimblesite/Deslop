import unittest
from datetime import date
from pathlib import Path
from typing import Iterable
from unittest.mock import MagicMock, patch

from scripts.issues.generate_issue_report import (
    RawIssue,
    build_report,
    extract_references,
    lifecycle_for,
    populate_relationships,
    priority_for,
    workstream_for,
)


def issue(
    number: int,
    title: str,
    labels: Iterable[str] = (),
    body: str = "",
    issue_type: str = "Bug",
    sub_issues: Iterable[int] = (),
) -> RawIssue:
    return {
        "number": number,
        "title": title,
        "body": body,
        "html_url": f"https://github.com/Nimblesite/Deslop/issues/{number}",
        "created_at": "2026-08-01T00:00:00Z",
        "updated_at": "2026-08-02T00:00:00Z",
        "labels": [
            {"name": name, "color": "ff0000", "description": f"{name} label"}
            for name in labels
        ],
        "type": {"name": issue_type, "color": "blue", "description": ""},
        "assignees": [],
        "milestone": None,
        "sub_issue_numbers": list(sub_issues),
        "blocked_by_numbers": [],
        "blocking_numbers": [],
    }


class IssueReportTests(unittest.TestCase):
    def test_pages_deploy_always_generates_fresh_issue_data(self) -> None:
        root = Path(__file__).resolve().parents[2]
        package = (root / "site/package.json").read_text(encoding="utf-8")
        workflow = (root / ".github/workflows/deploy-pages.yml").read_text(encoding="utf-8")

        self.assertIn('"build": "npm run issues:generate && npx @11ty/eleventy"', package)
        self.assertIn("issues: read", workflow)
        self.assertIn("GITHUB_TOKEN: ${{ github.token }}", workflow)
        self.assertIn("run: npm run build", workflow)

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

        populate_relationships(item, "Nimblesite/Deslop")

        self.assertEqual(item.get("blocked_by_numbers"), [10])
        self.assertEqual(item.get("blocking_numbers"), [30])
        report = build_report([blocker, item, blocked], "Nimblesite/Deslop", date(2026, 8, 21))
        self.assertIn({"source": 10, "target": 20, "kind": "blocks"}, report["relationships"])
        self.assertIn({"source": 20, "target": 30, "kind": "blocks"}, report["relationships"])

    def test_references_are_unique_open_issue_numbers(self) -> None:
        body = "Related: #12, #12 and Nimblesite/Deslop#13; not #99."
        self.assertEqual(extract_references(body, {12, 13}, 13), [12])

    def test_fixed_on_main_is_a_verification_lifecycle(self) -> None:
        labels = {"showstopper", "fixed-on-main"}
        self.assertEqual(lifecycle_for(labels), "verify")
        self.assertEqual(priority_for(labels, "Bug")[0], "verify_release")

    def test_accuracy_label_wins_workstream_routing(self) -> None:
        item = issue(1, "VSIX panel misses a clone", ("false-negative",))
        self.assertEqual(workstream_for(item), "accuracy")

    def test_report_builds_relationships_and_indicative_sequence(self) -> None:
        issues = [
            issue(10, "Parent pipeline work", ("critical",), sub_issues=(11,)),
            issue(11, "Cache implementation", body="Related to #12"),
            issue(12, "Release verification", ("fixed-on-main",)),
        ]

        report = build_report(issues, "Nimblesite/Deslop", date(2026, 8, 21))

        self.assertEqual(report["summary"]["open"], 3)
        self.assertEqual(report["summary"]["verify"], 1)
        self.assertEqual(report["issues"][0]["number"], 12)
        self.assertEqual(report["issues"][0]["lifecycle"], "verify")
        first_issue = report["issues"][0]
        self.assertIn("plan", first_issue)
        plan = first_issue.get("plan", {})
        self.assertEqual(plan["offset"], 0)
        self.assertEqual(plan["effort_units"], 2)
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
        blocker = issue(10, "Prerequisite task", issue_type="Task")
        blocker["blocking_numbers"] = [20]
        blocked = issue(20, "Release verification", ("fixed-on-main",))
        unrelated = issue(30, "Independent showstopper", ("showstopper",))

        report = build_report([blocked, blocker, unrelated], "Nimblesite/Deslop", date(2026, 8, 21))

        self.assertEqual([item["number"] for item in report["issues"]], [30, 10, 20])
        self.assertIn(
            {"source": 10, "target": 20, "kind": "blocks"},
            report["relationships"],
        )

    def test_default_effort_is_deterministic_date_free_and_not_ai_enriched(self) -> None:
        issues = [
            issue(1, "Verify", ("fixed-on-main",)),
            issue(2, "Blocker", ("showstopper",)),
            issue(3, "Critical", ("critical",)),
            issue(4, "Bug"),
            issue(5, "Feature", issue_type="Feature"),
            issue(6, "Task", issue_type="Task"),
        ]

        report = build_report(issues, "Nimblesite/Deslop", date(2026, 8, 21))
        efforts = {item["number"]: item.get("plan", {})["effort_units"] for item in report["issues"]}

        self.assertEqual(efforts, {1: 2, 2: 3, 3: 4, 4: 5, 5: 8, 6: 4})
        for item in report["issues"]:
            self.assertEqual(set(item.get("plan", {})), {"offset", "effort_units", "track"})
        self.assertIn("not a schedule", report["meta"]["planning_note"].lower())
        self.assertIn("No AI enrichment", report["meta"]["method"])
        repeated = build_report(issues, "Nimblesite/Deslop", date(2026, 8, 21))
        self.assertEqual(report["issues"], repeated["issues"])


if __name__ == "__main__":
    unittest.main()
