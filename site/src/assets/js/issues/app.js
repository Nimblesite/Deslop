import { clear, element, labelChip } from "./dom.js";
import { allLabels, filteredIssues, priorityColor, relatedIssues, streamMap } from "./model.js";
import { renderNetwork } from "./graph.js";
import { renderBoard, renderQueue, renderRunway } from "./views.js";

const root = document.querySelector("[data-issue-atlas]");
const stage = root.querySelector("[data-stage]");
const filters = root.querySelector("[data-filters]");
const drawer = root.querySelector("[data-issue-drawer]");
const drawerScrim = root.querySelector("[data-drawer-scrim]");
const defaultView = root.dataset.defaultView || "network";
const allowedViews = new Set([
  defaultView,
  ...Array.from(root.querySelectorAll("[data-view]"), (tab) => tab.dataset.view),
]);
const state = { view: defaultView, search: "", workstream: "", priority: "", label: "" };
let report;

function option(value, text) {
  return element("option", { text, attrs: { value } });
}

function fillSelect(name, values) {
  const select = filters.elements.namedItem(name);
  for (const value of values) select.append(option(value.id, value.name));
}

function populateFilters() {
  fillSelect("workstream", report.workstreams);
  fillSelect("priority", report.priorities);
  fillSelect("label", allLabels(report).map((name) => ({ id: name, name })));
}

function filtersFromForm() {
  const values = new FormData(filters);
  state.search = String(values.get("search") || "").trim();
  state.workstream = String(values.get("workstream") || "");
  state.priority = String(values.get("priority") || "");
  state.label = String(values.get("label") || "");
}

function updateCount(visible) {
  const noun = visible.length === 1 ? "issue" : "issues";
  root.querySelector("[data-visible-count]").textContent = `Showing ${visible.length} of ${report.issues.length} ${noun}`;
}

const renderers = {
  network: renderNetwork,
  runway: renderRunway,
  board: renderBoard,
  queue: renderQueue,
};

function render() {
  const visible = filteredIssues(report, state);
  updateCount(visible);
  renderers[state.view](stage, report, visible, openDrawer);
  updateUrl();
}

function updateTabs(view) {
  for (const tab of root.querySelectorAll("[data-view]")) {
    const active = tab.dataset.view === view;
    tab.classList.toggle("is-active", active);
    tab.setAttribute("aria-selected", String(active));
  }
}

function selectView(view) {
  if (!allowedViews.has(view)) return;
  state.view = view;
  updateTabs(view);
  render();
}

function updateUrl(issue) {
  const url = new URL(window.location.href);
  if (state.view === defaultView) url.searchParams.delete("view");
  else url.searchParams.set("view", state.view);
  if (issue) url.searchParams.set("issue", String(issue.number));
  else if (!drawer.classList.contains("is-open")) url.searchParams.delete("issue");
  history.replaceState({}, "", url);
}

function drawerFact(label, value) {
  return element("div", { className: "drawer-fact" }, [
    element("span", { text: label }),
    element("strong", { text: value || "—" }),
  ]);
}

function relationshipList(issue) {
  const relations = relatedIssues(report, issue).slice(0, 8);
  if (!relations.length) return element("p", { className: "drawer-excerpt", text: "No explicit open-issue relationships are recorded." });
  const list = element("div", { className: "drawer-related" });
  for (const relation of relations) list.append(relationshipLink(relation));
  return list;
}

function relationshipLink(relation) {
  const link = element("a", { attrs: { href: relation.issue.url, target: "_blank", rel: "noopener noreferrer" } });
  link.append(element("span", { text: `#${relation.issue.number} · ${relation.kind.replace("_", " ")}` }));
  link.append(element("strong", { text: relation.issue.title }));
  return link;
}

function drawerFacts(issue, streams) {
  const assignees = issue.assignees.map((assignee) => `@${assignee.login}`).join(", ");
  return element("div", { className: "drawer-facts" }, [
    drawerFact("Workstream", streams.get(issue.workstream).name),
    drawerFact("Issue type", issue.type),
    drawerFact("Assignee", assignees || "Unassigned"),
    drawerFact("Inbound links", String(issue.inbound_links)),
    drawerFact("Relative effort", `${issue.plan.effort_units} units`),
    drawerFact("Milestone", issue.milestone || "None"),
  ]);
}

function drawerPriority(issue) {
  const block = element("div", { className: "drawer-priority" }, [
    element("strong", { text: issue.priority_name }),
    element("span", { text: issue.priority_reason }),
  ]);
  block.style.setProperty("--priority-color", priorityColor(issue));
  return block;
}

function drawerContent(issue) {
  const streams = streamMap(report);
  const close = element("button", { className: "drawer-close", text: "×", attrs: { type: "button", "aria-label": "Close issue details" } });
  close.addEventListener("click", closeDrawer);
  return [
    close,
    element("p", { className: "drawer-kicker", text: `Issue #${issue.number} · ${issue.lifecycle === "verify" ? "verify next release" : "open"}` }),
    element("h2", { text: issue.title }),
    drawerPriority(issue),
    element("p", { className: "drawer-excerpt", text: issue.excerpt || "No description provided." }),
    drawerFacts(issue, streams),
    element("div", { className: "drawer-labels" }, issue.labels.map(labelChip)),
    element("h3", { className: "drawer-section-title", text: "Connected issues" }),
    relationshipList(issue),
    element("a", { className: "drawer-link", attrs: { href: issue.url, target: "_blank", rel: "noopener noreferrer" } }, [document.createTextNode("Open the full issue on GitHub"), element("span", { text: "↗" })]),
  ];
}

function openDrawer(issue) {
  clear(drawer);
  drawer.append(...drawerContent(issue));
  drawer.classList.add("is-open");
  drawerScrim.classList.add("is-open");
  drawer.setAttribute("aria-hidden", "false");
  document.body.style.overflow = "hidden";
  root.querySelectorAll(".graph-node").forEach((node) => node.classList.toggle("is-selected", Number(node.dataset.issue) === issue.number));
  updateUrl(issue);
  drawer.querySelector(".drawer-close").focus();
}

function closeDrawer() {
  drawer.classList.remove("is-open");
  drawerScrim.classList.remove("is-open");
  drawer.setAttribute("aria-hidden", "true");
  document.body.style.overflow = "";
  root.querySelectorAll(".graph-node").forEach((node) => node.classList.remove("is-selected"));
  updateUrl();
}

function bindEvents() {
  root.querySelectorAll("[data-view]").forEach((tab) => tab.addEventListener("click", () => selectView(tab.dataset.view)));
  filters.addEventListener("input", () => { filtersFromForm(); render(); });
  filters.addEventListener("change", () => { filtersFromForm(); render(); });
  filters.addEventListener("reset", () => setTimeout(() => { filtersFromForm(); render(); }));
  drawerScrim.addEventListener("click", closeDrawer);
  document.addEventListener("keydown", (event) => { if (event.key === "Escape") closeDrawer(); });
}

function restoreUrlState() {
  const params = new URLSearchParams(window.location.search);
  const requestedView = params.get("view");
  if (renderers[requestedView] && allowedViews.has(requestedView)) state.view = requestedView;
  updateTabs(state.view);
  return Number(params.get("issue")) || null;
}

function showLoadError(error) {
  clear(stage);
  stage.append(element("div", { className: "view-empty" }, [
    element("strong", { text: "The issue map could not load." }),
    element("p", { text: error.message }),
  ]));
}

async function initialise() {
  try {
    const response = await fetch("/assets/data/issues.json");
    if (!response.ok) throw new Error(`Issue data request failed (${response.status}).`);
    report = await response.json();
    populateFilters();
    bindEvents();
    const selected = restoreUrlState();
    render();
    const selectedIssue = report.issues.find((issue) => issue.number === selected);
    if (selectedIssue) openDrawer(selectedIssue);
  } catch (error) {
    showLoadError(error instanceof Error ? error : new Error(String(error)));
  }
}

initialise();
