import { clear, element, emptyState, svgElement } from "./dom.js";
import { priorityColor, streamMap, visibleRelationships } from "./model.js";

const WIDTH = 1200;
const HEIGHT = 720;
const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

function streamCenter(index) {
  const column = index % 4;
  const row = Math.floor(index / 4);
  return { x: 150 + column * 300, y: 190 + row * 340 };
}

function streamPositions(items, center) {
  return items.map((issue, index) => {
    const radius = index === 0 ? 0 : 28 * Math.sqrt(index);
    const angle = index * GOLDEN_ANGLE;
    return { issue, x: center.x + Math.cos(angle) * radius, y: center.y + Math.sin(angle) * radius };
  });
}

function layoutIssues(report, issues) {
  const grouped = new Map(report.workstreams.map((stream) => [stream.id, []]));
  for (const issue of issues) grouped.get(issue.workstream)?.push(issue);
  const positions = report.workstreams.flatMap((stream, index) => streamPositions(grouped.get(stream.id), streamCenter(index)));
  return new Map(positions.map((position) => [position.issue.number, position]));
}

function haloSize(count) {
  const diameter = Math.max(180, Math.min(290, 120 + Math.sqrt(count) * 34));
  return { width: diameter, height: diameter * 0.82 };
}

function appendHalo(layer, stream, index, count) {
  const center = streamCenter(index);
  const size = haloSize(count);
  const halo = svgElement("ellipse", { cx: center.x, cy: center.y, rx: size.width / 2, ry: size.height / 2, class: "graph-halo" });
  halo.style.fill = stream.color;
  halo.style.stroke = stream.color;
  layer.append(halo, textNode(stream.name, center.x, center.y - size.height / 2 + 20, "graph-cluster-label"));
  layer.append(textNode(`${count} issues`, center.x, center.y - size.height / 2 + 38, "graph-cluster-count"));
}

function textNode(text, x, y, className) {
  const node = svgElement("text", { x, y, class: className });
  node.textContent = text;
  return node;
}

function appendHalos(layer, report, issues) {
  const counts = new Map(report.workstreams.map((stream) => [stream.id, 0]));
  for (const issue of issues) counts.set(issue.workstream, counts.get(issue.workstream) + 1);
  report.workstreams.forEach((stream, index) => {
    if (counts.get(stream.id)) appendHalo(layer, stream, index, counts.get(stream.id));
  });
}

function appendEdges(layer, relationships, positions) {
  for (const edge of relationships) {
    const source = positions.get(edge.source);
    const target = positions.get(edge.target);
    if (!source || !target) continue;
    const line = svgElement("line", { x1: source.x, y1: source.y, x2: target.x, y2: target.y, class: `graph-edge graph-edge--${edge.kind}` });
    line.dataset.source = String(edge.source);
    line.dataset.target = String(edge.target);
    layer.append(line);
  }
}

function nodeRadius(issue) {
  return Math.min(18, 9 + issue.inbound_links * 1.35);
}

function nodeClass(issue) {
  const lifecycle = issue.lifecycle === "verify" ? " graph-node--verify" : "";
  return `graph-node${lifecycle}`;
}

function createNode(position, stream, onSelect, tooltip) {
  const issue = position.issue;
  const radius = nodeRadius(issue);
  const group = svgElement("g", { class: nodeClass(issue), role: "button", tabindex: "0", "aria-label": `Issue ${issue.number}: ${issue.title}` });
  group.dataset.issue = String(issue.number);
  group.setAttribute("transform", `translate(${position.x} ${position.y})`);
  const ring = svgElement("circle", { r: radius + 4, class: "graph-node__ring" });
  const dot = svgElement("circle", { r: radius, class: "graph-node__dot", fill: stream.color });
  const label = textNode(String(issue.number), 0, 0.5, "graph-node__label");
  if (issue.lifecycle !== "verify" && issue.priority !== "release_blocker") ring.style.display = "none";
  if (issue.priority === "release_blocker") ring.style.stroke = priorityColor(issue);
  group.append(ring, dot, label);
  bindNodeEvents(group, issue, onSelect, tooltip);
  return group;
}

function bindNodeEvents(node, issue, onSelect, tooltip) {
  node.addEventListener("click", (event) => { event.stopPropagation(); onSelect(issue); });
  node.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") onSelect(issue);
  });
  node.addEventListener("pointerenter", (event) => showTooltip(tooltip, issue, event));
  node.addEventListener("pointermove", (event) => placeTooltip(tooltip, event));
  node.addEventListener("pointerleave", () => tooltip.classList.remove("is-visible"));
}

function showTooltip(tooltip, issue, event) {
  clear(tooltip);
  tooltip.append(element("span", { text: `#${issue.number} · ${issue.priority_name}` }), document.createTextNode(issue.title));
  tooltip.classList.add("is-visible");
  placeTooltip(tooltip, event);
}

function placeTooltip(tooltip, event) {
  const bounds = tooltip.parentElement.getBoundingClientRect();
  const left = Math.min(event.clientX - bounds.left + 14, bounds.width - 320);
  const top = Math.min(event.clientY - bounds.top + 14, bounds.height - 120);
  tooltip.style.left = `${Math.max(8, left)}px`;
  tooltip.style.top = `${Math.max(8, top)}px`;
}

function appendNodes(layer, report, positions, onSelect, tooltip) {
  const streams = streamMap(report);
  for (const position of positions.values()) {
    layer.append(createNode(position, streams.get(position.issue.workstream), onSelect, tooltip));
  }
}

function graphLayers(svg, report, issues, positions, onSelect, tooltip) {
  const viewport = svgElement("g", { class: "graph-viewport", "data-zoom": "1" });
  const halos = svgElement("g");
  const edges = svgElement("g");
  const nodes = svgElement("g");
  appendHalos(halos, report, issues);
  appendEdges(edges, visibleRelationships(report, issues), positions);
  appendNodes(nodes, report, positions, onSelect, tooltip);
  viewport.append(halos, edges, nodes);
  svg.append(viewport);
  return viewport;
}

function applyTransform(viewport, state) {
  viewport.setAttribute("transform", `translate(${state.x} ${state.y}) scale(${state.scale})`);
  viewport.dataset.zoom = state.scale.toFixed(2);
}

function zoomAt(viewport, state, factor, anchor = { x: WIDTH / 2, y: HEIGHT / 2 }) {
  const next = Math.max(0.55, Math.min(3.5, state.scale * factor));
  state.x = anchor.x - ((anchor.x - state.x) * next) / state.scale;
  state.y = anchor.y - ((anchor.y - state.y) * next) / state.scale;
  state.scale = next;
  applyTransform(viewport, state);
}

function wheelAnchor(svg, event) {
  const rect = svg.getBoundingClientRect();
  return { x: ((event.clientX - rect.left) / rect.width) * WIDTH, y: ((event.clientY - rect.top) / rect.height) * HEIGHT };
}

function bindWheel(svg, viewport, state) {
  svg.addEventListener("wheel", (event) => {
    event.preventDefault();
    zoomAt(viewport, state, event.deltaY < 0 ? 1.14 : 0.88, wheelAnchor(svg, event));
  }, { passive: false });
}

function bindPan(svg, canvas, viewport, state) {
  let pointer = null;
  svg.addEventListener("pointerdown", (event) => {
    if (event.target.closest(".graph-node")) return;
    pointer = { x: event.clientX, y: event.clientY };
    canvas.classList.add("is-panning");
    svg.setPointerCapture(event.pointerId);
  });
  svg.addEventListener("pointermove", (event) => {
    if (!pointer) return;
    const ratio = WIDTH / svg.getBoundingClientRect().width;
    state.x += (event.clientX - pointer.x) * ratio;
    state.y += (event.clientY - pointer.y) * ratio;
    pointer = { x: event.clientX, y: event.clientY };
    applyTransform(viewport, state);
  });
  svg.addEventListener("pointerup", () => { pointer = null; canvas.classList.remove("is-panning"); });
}

function resetTransform(viewport, state) {
  Object.assign(state, { x: 0, y: 0, scale: 1 });
  applyTransform(viewport, state);
}

function toolButton(label, text) {
  return element("button", { className: "network-tool", text, attrs: { type: "button", "aria-label": label, title: label } });
}

function networkTools(viewport, state) {
  const zoomIn = toolButton("Zoom in", "+");
  const zoomOut = toolButton("Zoom out", "−");
  const reset = toolButton("Reset graph position", "↺");
  zoomIn.addEventListener("click", () => zoomAt(viewport, state, 1.2));
  zoomOut.addEventListener("click", () => zoomAt(viewport, state, 0.82));
  reset.addEventListener("click", () => resetTransform(viewport, state));
  return element("div", { className: "network-tools" }, [
    element("span", { className: "network-hint", text: "Scroll to zoom · drag the canvas to move · select a node for context" }),
    element("div", { className: "network-tools__buttons" }, [zoomOut, reset, zoomIn]),
  ]);
}

function legend(report, issues) {
  const present = new Set(issues.map((issue) => issue.workstream));
  const items = report.workstreams.filter((stream) => present.has(stream.id)).map((stream) => {
    const swatch = element("span", { className: "legend-swatch" });
    swatch.style.setProperty("--swatch", stream.color);
    return element("span", { className: "legend-item" }, [swatch, document.createTextNode(stream.name)]);
  });
  return element("div", { className: "graph-legend" }, items);
}

export function renderNetwork(container, report, issues, onSelect) {
  clear(container);
  if (!issues.length) return container.append(emptyState());
  const shell = element("section", { className: "network-view", attrs: { "data-view-panel": "network" } });
  const canvas = element("div", { className: "network-canvas" });
  const tooltip = element("div", { className: "graph-tooltip", attrs: { role: "tooltip" } });
  const svg = svgElement("svg", { class: "network-svg", viewBox: `0 0 ${WIDTH} ${HEIGHT}`, "aria-label": "Issue relationship network" });
  const positions = layoutIssues(report, issues);
  const viewport = graphLayers(svg, report, issues, positions, onSelect, tooltip);
  const state = { x: 0, y: 0, scale: 1 };
  bindWheel(svg, viewport, state);
  bindPan(svg, canvas, viewport, state);
  canvas.append(svg, tooltip);
  shell.append(networkTools(viewport, state), canvas, legend(report, issues));
  container.append(shell);
}
