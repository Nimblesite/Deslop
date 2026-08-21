import unittest
from datetime import date
from typing import Iterable

from scripts.issues.generate_issue_report import (
    RawIssue,
    build_report,
    extract_references,
    lifecycle_for,
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

    def test_report_builds_relationships_and_schedule(self) -> None:
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
        self.assertEqual(first_issue.get("plan", {})["effort_days"], 2)
        self.assertIn(
            {"source": 10, "target": 11, "kind": "sub_issue"},
            report["relationships"],
        )
        self.assertIn(
            {"source": 11, "target": 12, "kind": "reference"},
            report["relationships"],
        )


if __name__ == "__main__":
    unittest.main()
