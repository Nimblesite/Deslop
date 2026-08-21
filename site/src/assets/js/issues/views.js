import { clear, element, emptyState, labelChip } from "./dom.js";
import { priorityColor, streamMap } from "./model.js";

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
  const color = priorityColor(priority.id);
  const lane = element("section", { className: "board-lane", attrs: { "data-priority": priority.id } });
  const head = element("header", { className: "board-lane__head" }, [
    element("span", { className: "board-lane__order", text: `${String(priority.rank + 1).padStart(2, "0")} · ${issues.length} issues` }),
    element("h3", { text: priority.name }),
    element("p", { text: priority.description }),
  ]);
  head.style.setProperty("--lane-color", color);
  const list = element("div", { className: "board-list" }, issues.map((issue) => issueCard(issue, streams, onSelect)));
  lane.append(head, list);
  return lane;
}

export function renderBoard(container, report, issues, onSelect) {
  clear(container);
  if (!issues.length) return container.append(emptyState());
  const streams = streamMap(report);
  const board = element("div", { className: "priority-board", attrs: { "data-view-panel": "board" } });
  for (const priority of report.priorities) {
    const laneIssues = issues.filter((issue) => issue.priority === priority.id);
    if (laneIssues.length) board.append(boardLane(priority, laneIssues, streams, onSelect));
  }
  container.append(board);
}

function timelineBounds(issues) {
  const starts = issues.map((issue) => issue.plan.offset);
  const ends = issues.map((issue) => issue.plan.offset + issue.plan.effort_units);
  const start = Math.min(...starts);
  const end = Math.max(...ends);
  return { start, end, span: Math.max(end - start, 1) };
}

function weekCount(bounds) {
  return Math.max(1, Math.ceil(bounds.span / 7));
}

function stepHeaders(steps) {
  const cells = [];
  for (let index = 0; index < steps; index += 1) {
    cells.push(element("span", { className: "runway-step", text: `Step ${String(index + 1).padStart(2, "0")}` }));
  }
  const row = element("div", { className: "runway-steps" }, cells);
  row.style.setProperty("--weeks", steps);
  return row;
}

function runwayHead(steps) {
  return element("div", { className: "runway-head" }, [
    element("div", { className: "runway-head__label", text: "Recommended order · default effort" }),
    stepHeaders(steps),
  ]);
}

function streamHeader(stream, count, weeks) {
  const label = element("div", { className: "runway-stream" }, [
    document.createTextNode(stream.name),
    element("span", { text: ` · ${count}` }),
  ]);
  label.style.setProperty("--stream-color", stream.color);
  const track = element("div", { className: "runway-track" });
  track.style.setProperty("--weeks", weeks);
  return element("div", { className: "runway-row" }, [label, track]);
}

function barPosition(issue, bounds) {
  const start = ((issue.plan.offset - bounds.start) / bounds.span) * 100;
  const duration = (issue.plan.effort_units / bounds.span) * 100;
  return { start: `${start}%`, duration: `${duration}%` };
}

function runwayIssue(issue, bounds, weeks, onSelect) {
  const track = element("div", { className: "runway-track" });
  const bar = element("button", { className: "runway-bar", text: `#${issue.number} · ${issue.title}`, attrs: { type: "button", title: `${issue.plan.effort_units} default effort units · ${issue.priority_name}` } });
  const position = barPosition(issue, bounds);
  track.style.setProperty("--weeks", weeks);
  bar.style.setProperty("--start", position.start);
  bar.style.setProperty("--duration", position.duration);
  bar.style.setProperty("--bar-color", priorityColor(issue));
  bar.addEventListener("click", () => onSelect(issue));
  track.append(bar);
  return element("div", { className: "runway-row", attrs: { "data-issue": issue.number } }, [
    element("div", { className: "runway-row__label", text: `#${issue.number} ${issue.title}`, attrs: { title: issue.title } }),
    track,
  ]);
}

function appendStreamRows(grid, report, issues, bounds, weeks, onSelect) {
  for (const stream of report.workstreams) {
    const streamIssues = issues.filter((issue) => issue.workstream === stream.id);
    if (!streamIssues.length) continue;
    grid.append(streamHeader(stream, streamIssues.length, weeks));
    for (const issue of streamIssues) grid.append(runwayIssue(issue, bounds, weeks, onSelect));
  }
}

export function renderRunway(container, report, issues, onSelect) {
  clear(container);
  if (!issues.length) return container.append(emptyState());
  const bounds = timelineBounds(issues);
  const weeks = weekCount(bounds);
  const grid = element("div", { className: "runway-grid" });
  grid.style.setProperty("--weeks", weeks);
  grid.append(runwayHead(weeks));
  appendStreamRows(grid, report, issues, bounds, weeks, onSelect);
  const notice = element("div", { className: "runway-notice" }, [
    element("strong", { text: "Indicative only — not a schedule" }),
    element("span", { text: "Relative sequence and default effort help compare the backlog; they are not delivery commitments." }),
  ]);
  container.append(element("div", { className: "runway-view", attrs: { "data-view-panel": "runway" } }, [notice, grid]));
}

function labelCell(issue) {
  const labels = issue.labels.map(labelChip);
  return element("div", { className: "queue-labels" }, labels);
}

function queueRow(issue, streams, onSelect) {
  const row = element("tr", { attrs: { tabindex: "0", "data-issue": issue.number } }, [
    element("td", { text: `#${issue.number}` }),
    element("td", { className: "issue-table__title", text: issue.title }),
    element("td", { text: issue.priority_name }),
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
  const body = element("tbody", {}, issues.map((issue) => queueRow(issue, streams, onSelect)));
  table.append(tableHead(), body);
  container.append(element("div", { className: "queue-wrap", attrs: { "data-view-panel": "queue" } }, [table]));
}
