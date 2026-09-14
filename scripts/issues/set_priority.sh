#!/usr/bin/env bash
# Set the GitHub Priority field on one issue. Priority is an org-level issue
# field, not a label — `gh issue edit` cannot set it, so this goes through the
# setIssueFieldValue GraphQL mutation. Options come from GitHub itself, so a
# renamed or added option needs no change here.
#
#   scripts/issues/set_priority.sh 442 critical
set -euo pipefail

REPO="${DESLOP_REPO:-Nimblesite/Deslop}"
ORG="${REPO%%/*}"
FIELD="Priority"

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <issue-number> <showstopper|critical|normal|low>" >&2
  exit 2
fi

issue_number="$1"
option_name="$2"

fields_query='query($org: String!) { organization(login: $org) { issueFields(first: 20) { nodes { ... on IssueFieldSingleSelect { id name options { id name } } } } } }'
selector=".data.organization.issueFields.nodes[] | select(.name == \"$FIELD\")"

field_id=$(gh api graphql -f query="$fields_query" -F org="$ORG" --jq "$selector | .id")
option_id=$(gh api graphql -f query="$fields_query" -F org="$ORG" --jq "$selector | .options[] | select(.name == \"$option_name\") | .id")

if [ -z "$option_id" ]; then
  echo "unknown $FIELD option '$option_name' in $ORG" >&2
  exit 1
fi

issue_id=$(gh api "repos/$REPO/issues/$issue_number" --jq .node_id)

gh api graphql --jq '.data.setIssueFieldValue.issue.number' \
  -f query='mutation($issue: ID!, $field: ID!, $option: ID!) { setIssueFieldValue(input: { issueId: $issue, issueFields: [{ fieldId: $field, singleSelectOptionId: $option }] }) { issue { number } } }' \
  -F issue="$issue_id" -F field="$field_id" -F option="$option_id" >/dev/null

echo "#$issue_number $FIELD = $option_name"
