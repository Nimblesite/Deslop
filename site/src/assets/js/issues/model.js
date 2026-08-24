// Priority option ids from the GitHub Priority issue field — see scripts/issues/rules.py.
export const SHOWSTOPPER = "showstopper";
export const CRITICAL = "critical";

export function priorityMap(report) {
  return new Map(report.priorities.map((priority) => [priority.id, priority]));
}

export function streamMap(report) {
  return new Map(report.workstreams.map((stream) => [stream.id, stream]));
}

export function issueMap(report) {
  return new Map(report.issues.map((issue) => [issue.number, issue]));
}

function searchableText(issue) {
  const labels = issue.labels.map((label) => label.name).join(" ");
  return `${issue.number} ${issue.title} ${issue.excerpt} ${labels} ${issue.type}`.toLowerCase();
}

function isIssueNumber(value) {
  return value.length > 1 && value.startsWith("#") && [...value.slice(1)].every((character) => character >= "0" && character <= "9");
}

function matchesSearch(issue, value) {
  const normalized = value.toLowerCase();
  if (isIssueNumber(normalized)) return String(issue.number) === normalized.slice(1);
  return searchableText(issue).includes(normalized);
}

function matches(issue, filters) {
  if (filters.search && !matchesSearch(issue, filters.search)) return false;
  if (filters.workstream && issue.workstream !== filters.workstream) return false;
  if (filters.priority && issue.priority !== filters.priority) return false;
  if (filters.label && !issue.labels.some((label) => label.name === filters.label)) return false;
  return true;
}

export function filteredIssues(report, filters) {
  return report.issues.filter((issue) => matches(issue, filters));
}

export function visibleRelationships(report, issues) {
  const numbers = new Set(issues.map((issue) => issue.number));
  return report.relationships.filter((edge) => numbers.has(edge.source) && numbers.has(edge.target));
}

export function allLabels(report) {
  const labels = new Set(report.issues.flatMap((issue) => issue.labels.map((label) => label.name)));
  return [...labels].sort((left, right) => left.localeCompare(right));
}

export function relatedIssues(report, issue) {
  const lookup = issueMap(report);
  return report.relationships.flatMap((edge) => relatedForEdge(edge, issue.number, lookup));
}

function relatedForEdge(edge, number, lookup) {
  if (edge.source === number && lookup.has(edge.target)) return [{ issue: lookup.get(edge.target), kind: edge.kind, direction: "out" }];
  if (edge.target === number && lookup.has(edge.source)) return [{ issue: lookup.get(edge.source), kind: edge.kind, direction: "in" }];
  return [];
}
