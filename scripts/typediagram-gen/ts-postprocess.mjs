import { EXTERNAL_TS_TYPES, TS_HEADER } from "./paths.mjs";
import { TYPE_CONFIG } from "./type-config.mjs";

export { TS_HEADER };

export function postprocessTs(ts) {
  let out = ts;
  out = out.replace(/undefined<([^>]+)>/gu, "$1 | null");
  out = rewriteUnions(out);
  out = markOptionalFields(out);
  out = dropSkippedTsBlocks(out);
  return out;
}

// Drops `export type|interface X` blocks whose TYPE_CONFIG entry sets
// `skipTs: true`. Used for wire types with no TS consumer (e.g. the
// JSON-RPC envelope, only consumed by the Rust MCP server).
// TODO(typeDiagram#27): drop this once per-target gating (`@targets(rust)`)
// is a first-class language attribute upstream.
function dropSkippedTsBlocks(ts) {
  const lines = ts.split("\n");
  const out = [];
  let dropping = false;
  for (const line of lines) {
    if (dropping) {
      const trimmed = line.trim();
      if (trimmed.endsWith(";") || trimmed === "}") dropping = false;
      continue;
    }
    const start =
      line.match(/^export type (\w+)\s*=/u) ??
      line.match(/^export interface (\w+)\s*\{/u);
    if (start && TYPE_CONFIG[start[1]]?.skipTs) {
      const trimmed = line.trim();
      if (!trimmed.endsWith(";") && !trimmed.endsWith("}")) dropping = true;
      continue;
    }
    out.push(line);
  }
  return out.join("\n");
}

function markOptionalFields(ts) {
  const lines = ts.split("\n");
  const out = [];
  let blockName = null;
  for (const line of lines) {
    const start = line.match(/^export interface (\w+)\s*\{/u);
    if (start) {
      blockName = start[1];
      out.push(line);
      continue;
    }
    if (blockName && line.trim() === "}") {
      blockName = null;
      out.push(line);
      continue;
    }
    const config = blockName ? TYPE_CONFIG[blockName] : null;
    const optional = config?.tsOptional ?? [];
    const field = optional.length > 0 && line.match(/^(\s*)(\w+):\s*(.+);\s*$/u);
    if (field && optional.includes(field[2])) {
      const [, indent, fieldName, fieldType] = field;
      out.push(`${indent}${fieldName}?: ${fieldType};`);
      continue;
    }
    out.push(line);
  }
  return out.join("\n");
}

function rewriteUnions(ts) {
  const lines = ts.split("\n");
  const out = [];
  let blockName = null;
  let blockLines = [];
  for (const line of lines) {
    const start = line.match(/^export type (\w+)\s*=\s*$/u);
    if (!blockName && start) {
      blockName = start[1];
      blockLines = [line];
      continue;
    }
    if (blockName) {
      blockLines.push(line);
      if (line.trim().endsWith(";")) {
        out.push(...rewriteUnionBlock(blockName, blockLines));
        blockName = null;
        blockLines = [];
      }
      continue;
    }
    out.push(line);
  }
  if (blockName) out.push(...blockLines);
  return out.join("\n");
}

function rewriteUnionBlock(name, blockLines) {
  const config = TYPE_CONFIG[name];
  if (!config) return blockLines;
  const variantName = variantCaseFn(config);
  const tagName = tagNameOf(config) ?? "kind";
  const variants = blockLines.slice(1).map(parseVariantLine);
  if (!variantName || variants.some((variant) => !variant)) return blockLines;
  if (variants.every((variant) => variant.fields.trim().length === 0)) {
    const values = variants.map((variant) => `"${variantName(variant.name)}"`);
    return [`export type ${name} = ${values.join(" | ")};`];
  }
  return [
    blockLines[0],
    ...variants.map((variant) => {
      const fields = variant.fields ? `;${variant.fields}` : "";
      return `${variant.indent}| { ${tagName}: "${variantName(variant.name)}"${fields} }${variant.end}`;
    }),
  ];
}

function parseVariantLine(line) {
  const match = line.match(/^(\s*)\|\s*\{\s*kind:\s*"(\w+)"(.*?)\s*\}(;?)\s*$/u);
  if (!match) return null;
  const [, indent, name, rawFields, end] = match;
  const fields = rawFields.replace(/^;/u, "").trimEnd();
  return { indent, name, fields, end };
}

function tagNameOf(config) {
  for (const attr of config?.serdeAttrs ?? []) {
    const match = attr.match(/^tag\s*=\s*"(\w+)"$/u);
    if (match) return match[1];
  }
  return null;
}

function variantCaseFn(config) {
  for (const attr of config?.serdeAttrs ?? []) {
    const match = attr.match(/^rename_all\s*=\s*"(\w+)"$/u);
    if (!match) continue;
    if (match[1] === "snake_case") return toSnakeCase;
    if (match[1] === "lowercase") return (name) => name.toLowerCase();
    if (match[1] === "UPPERCASE") return (name) => name.toUpperCase();
  }
  return null;
}

function toSnakeCase(name) {
  return name
    .replace(/([A-Z])([A-Z][a-z])/gu, "$1_$2")
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase();
}

export function tsImports(body) {
  return Object.entries(EXTERNAL_TS_TYPES)
    .filter(([name]) => referencesExternalType(body, name))
    .map(([name, path]) => `import type { ${name} } from "${path}";`)
    .sort();
}

function referencesExternalType(body, name) {
  const declared = new RegExp(`export (interface|type) ${name}\\b`, "u");
  const referenced = new RegExp(`\\b${name}\\b`, "u");
  return !declared.test(body) && referenced.test(body);
}
