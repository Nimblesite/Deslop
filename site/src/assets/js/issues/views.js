import { clear, element, emptyState, labelChip } from "./dom.js";
import { CRITICAL, SHOWSTOPPER, priorityMap, streamMap, visibleRelationships } from "./model.js";

function cardLabels(issue) {
  return element("div", { className: "card-labels" }, issue.labels.map(labelChip));
}

function issueCard(issue, streams, onSelect) {
  const stream = streams.get(issue.workstream);
  const card = element("button", { className: "issue-card", attrs: { type: "button", "data-issue": issue.number } });
  const meta = element("div", { className: "issue-card__meta" }, [
    element("span", { text: `#${issue.number}` }),
    element("span", { text: stream.name }),
  ]);
  card.append(meta, element("strong", { text: issue.title }));
  if (issue.labels.length) card.append(cardLabels(issue));
  card.addEventListener("click", () => onSelect(issue));
  return card;
}

function boardLane(priority, issues, streams, onSelect) {
  const lane = element("section", { className: "board-lane", attrs: { "data-priority": priority.id } });
  const head = element("header", { className: "board-lane__head" }, [
    element("span", { className: "board-lane__order", text: `${String(priority.rank).padStart(2, "0")} · ${issues.length} issues` }),
    element("h3", { text: priority.name }),
    element("p", { text: priority.description }),
  ]);
  head.style.setProperty("--lane-color", priority.color);
  const list = element("div", { className: "board-list" }, issues.map((issue) => issueCard(issue, streams, onSelect)));
  lane.append(head, list);
  return lane;
}

export function renderBoard(container, report, issues, onSelect) {
  clear(container);
  if (!issues.length) return container.append(emptyState());
  const streams = streamMap(report);
  const board = element("div", { className: "priority-board", attrs: { id: "panel-board", role: "tabpanel", "aria-labelledby": "tab-board", "data-view-panel": "board" } });
  for (const priority of report.priorities) {
    const laneIssues = issues.filter((issue) => issue.priority === priority.id);
    if (laneIssues.length) board.append(boardLane(priority, laneIssues, streams, onSelect));
  }
  container.append(board);
}

function statisticsValues(report, issues) {
  const relationships = visibleRelationships(report, issues);
  const linked = new Set(relationships.flatMap((edge) => [edge.source, edge.target]));
  return {
    open: issues.length,
    verify: issues.filter((issue) => issue.lifecycle === "verify").length,
    showstoppers: issues.filter((issue) => issue.priority === SHOWSTOPPER).length,
    critical: issues.filter((issue) => issue.priority === CRITICAL).length,
    linked: linked.size,
  };
}

function summaryCard(label, value, key, caption, modifier = "") {
  return element("article", { className: `summary-card ${modifier}`.trim() }, [
    element("span", { text: label }),
    element("strong", { text: String(value), attrs: { "data-summary": key } }),
    element("p", { text: caption }),
  ]);
}

export function renderStatistics(container, report, issues) {
  clear(container);
  const values = statisticsValues(report, issues);
  const summary = element("section", { className: "atlas-summary", attrs: { "aria-label": "Backlog summary" } }, [
    summaryCard("Open backlog", values.open, "open", "open issues"),
    summaryCard("Release verification", values.verify, "verify", "believed fixed on main", "summary-card--verify"),
    summaryCard("Showstoppers", values.showstoppers, "showstoppers", "unreleased regressions", "summary-card--blocker"),
    summaryCard("Critical", values.critical, "critical", "accuracy or usefulness at risk"),
    summaryCard("Connected work", values.linked, "linked", "linked issues"),
  ]);
  const note = element("aside", { className: "verification-note" }, [
    element("span", { className: "verification-note__mark", text: "✓", attrs: { "aria-hidden": "true" } }),
    element("div", {}, [
      element("strong", { text: "“fixed-on-main” is a verification state, not done." }),
      element("p", { text: "To the best of our knowledge, the bug is fixed on main. It stays open until someone verifies the fix in a real release." }),
    ]),
  ]);
  container.append(element("section", { className: "statistics-view", attrs: { id: "panel-statistics", role: "tabpanel", "aria-labelledby": "tab-statistics", "data-view-panel": "statistics" } }, [
    element("header", { className: "statistics-view__header" }, [
      element("div", {}, [element("span", { className: "atlas-map-eyebrow", text: "Public GitHub issue data" }), element("h2", { text: "Issue statistics" })]),
      element("p", { text: "Compact counts reflect the filters above." }),
    ]),
    summary,
    note,
    element("p", { className: "statistics-source", text: report.meta.method }),
  ]));
}

function sequenceBounds(issues) {
  const starts = issues.map((issue) => issue.plan.offset);
  const ends = issues.map((issue) => issue.plan.offset + issue.plan.effort_units);
  const start = Math.min(...starts);
  const end = Math.max(...ends);
  return { start, end, span: Math.max(end - start, 1) };
}

function stepCount(bounds) {
  return Math.max(1, Math.ceil(bounds.span / 7));
}

function stepHeaders(steps) {
  const cells = [];
  for (let index = 0; index < steps; index += 1) {
    cells.push(element("span", { className: "runway-step", text: `Step ${String(index + 1).padStart(2, "0")}` }));
  }
  const row = element("div", { className: "runway-steps" }, cells);
  row.style.setProperty("--steps", steps);
  return row;
}

function runwayHead(steps) {
  return element("div", { className: "runway-head" }, [
    element("div", { className: "runway-head__label", text: "Recommended order · default effort" }),
    stepHeaders(steps),
  ]);
}

function streamHeader(stream, count, steps) {
  const label = element("div", { className: "runway-stream" }, [
    document.createTextNode(stream.name),
    element("span", { text: ` · ${count}` }),
  ]);
  label.style.setProperty("--stream-color", stream.color);
  const track = element("div", { className: "runway-track" });
  track.style.setProperty("--steps", steps);
  return element("div", { className: "runway-row" }, [label, track]);
}

function barPosition(issue, bounds) {
  const start = ((issue.plan.offset - bounds.start) / bounds.span) * 100;
  const duration = (issue.plan.effort_units / bounds.span) * 100;
  return { start: `${start}%`, duration: `${duration}%` };
}

function runwayIssue(issue, bounds, steps, onSelect, priority) {
  const track = element("div", { className: "runway-track" });
  const bar = element("button", { className: "runway-bar", text: `#${issue.number} · ${issue.title}`, attrs: { type: "button", title: `${issue.plan.effort_units} default effort units · ${priority.name}` } });
  const position = barPosition(issue, bounds);
  track.style.setProperty("--steps", steps);
  bar.style.setProperty("--start", position.start);
  bar.style.setProperty("--duration", position.duration);
  bar.style.setProperty("--bar-color", priority.color);
  bar.addEventListener("click", () => onSelect(issue));
  track.append(bar);
  return element("div", { className: "runway-row", attrs: { "data-issue": issue.number } }, [
    element("div", { className: "runway-row__label", text: `#${issue.number} ${issue.title}`, attrs: { title: issue.title } }),
    track,
  ]);
}

function appendStreamRows(grid, report, issues, bounds, steps, onSelect) {
  const priorities = priorityMap(report);
  for (const stream of report.workstreams) {
    const streamIssues = issues.filter((issue) => issue.workstream === stream.id);
    if (!streamIssues.length) continue;
    grid.append(streamHeader(stream, streamIssues.length, steps));
    for (const issue of streamIssues) grid.append(runwayIssue(issue, bounds, steps, onSelect, priorities.get(issue.priority)));
  }
}

export function renderRunway(container, report, issues, onSelect) {
  clear(container);
  if (!issues.length) return container.append(emptyState());
  const bounds = sequenceBounds(issues);
  const steps = stepCount(bounds);
  const grid = element("div", { className: "runway-grid" });
  grid.style.setProperty("--steps", steps);
  grid.append(runwayHead(steps));
  appendStreamRows(grid, report, issues, bounds, steps, onSelect);
  const notice = element("div", { className: "runway-notice" }, [
    element("strong", { text: "Indicative only — not a schedule" }),
    element("span", { text: "Sequence and bar length show relative order and default effort—not dates, deadlines, estimates, or commitments." }),
  ]);
  const scroll = element("div", { className: "runway-scroll" }, [grid]);
  container.append(element("div", { className: "runway-view", attrs: { id: "panel-runway", role: "tabpanel", "aria-labelledby": "tab-runway", "data-view-panel": "runway" } }, [notice, scroll]));
}

function labelCell(issue) {
  const labels = issue.labels.map(labelChip);
  return element("div", { className: "queue-labels" }, labels);
}

function queueRow(issue, streams, priorities, onSelect) {
  const row = element("tr", { attrs: { tabindex: "0", "data-issue": issue.number } }, [
    element("td", { text: `#${issue.number}` }),
    element("td", { className: "issue-table__title", text: issue.title }),
    element("td", { text: priorities.get(issue.priority).name }),
    element("td", { text: streams.get(issue.workstream).name }),
    element("td", {}, [labelCell(issue)]),
  ]);
  row.addEventListener("click", () => onSelect(issue));
  row.addEventListener("keydown", (event) => {
    if (event.key === "Enter") onSelect(issue);
  });
  return row;
}

function tableHead() {
  const labels = ["Issue", "Title", "Recommended queue", "Workstream", "Labels"];
  return element("thead", {}, [element("tr", {}, labels.map((label) => element("th", { text: label })))]);
}

export function renderQueue(container, report, issues, onSelect) {
  clear(container);
  if (!issues.length) return container.append(emptyState());
  const streams = streamMap(report);
  const table = element("table", { className: "issue-table" });
  const priorities = priorityMap(report);
  const body = element("tbody", {}, issues.map((issue) => queueRow(issue, streams, priorities, onSelect)));
  table.append(tableHead(), body);
  container.append(element("div", { className: "queue-wrap", attrs: { id: "panel-queue", role: "tabpanel", "aria-labelledby": "tab-queue", "data-view-panel": "queue" } }, [table]));
}
