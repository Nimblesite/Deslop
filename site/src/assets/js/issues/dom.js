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

function readableLabelInk(color) {
  const hex = color.replace("#", "");
  const channels = [0, 2, 4].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255);
  const linear = channels.map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  const luminance = 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
  return 1.05 / (luminance + 0.05) > (luminance + 0.05) / 0.05 ? "#fff" : "#000";
}

export function labelChip(label) {
  const chip = element("span", { className: "label-chip", text: label.name });
  const color = label.color.startsWith("#") ? label.color : `#${label.color}`;
  chip.style.setProperty("--label-color", color);
  chip.style.setProperty("--label-ink", readableLabelInk(color));
  if (label.description) chip.title = label.description;
  return chip;
}

export function publicationStamp(report) {
  return element("time", {
    className: "atlas-publication",
    text: `Published ${report.meta.published_at_long} UTC`,
    attrs: { datetime: report.meta.published_at },
  });
}

export function emptyState() {
  return element("div", { className: "view-empty" }, [
    element("strong", { text: "Nothing matches these filters." }),
    element("p", { text: "Reset a filter to bring the map back into view." }),
  ]);
}
