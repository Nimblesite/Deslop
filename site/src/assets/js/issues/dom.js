export function element(tag, options = {}, children = []) {
  const node = document.createElement(tag);
  if (options.className) node.className = options.className;
  if (options.text !== undefined) node.textContent = options.text;
  for (const [name, value] of Object.entries(options.attrs || {})) {
    if (value !== undefined && value !== null) node.setAttribute(name, String(value));
  }
  for (const child of children) node.append(child);
  return node;
}

export function svgElement(tag, attributes = {}) {
  const node = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, String(value));
  return node;
}

export function clear(node) {
  node.replaceChildren();
}

export function shortDate(value) {
  return new Intl.DateTimeFormat("en-AU", { day: "numeric", month: "short", year: "numeric" }).format(new Date(value));
}

export function labelChip(label) {
  const chip = element("span", { className: "label-chip", text: label.name });
  chip.style.setProperty("--label-color", label.color);
  if (label.description) chip.title = label.description;
  return chip;
}

export function emptyState() {
  return element("div", { className: "view-empty" }, [
    element("strong", { text: "Nothing matches these filters." }),
    element("p", { text: "Reset a filter to bring the map back into view." }),
  ]);
}
