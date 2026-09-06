import { EXTERNAL_TYPES, HEADER_PRELUDE } from "./paths.mjs";
import { TYPE_CONFIG } from "./type-config.mjs";

function splitItems(rust) {
  const lines = rust.split("\n");
  const items = [];
  let current = [];
  for (const line of lines) {
    if (line.startsWith("pub struct ") || line.startsWith("pub enum ") ||
        line.startsWith("pub type ")) {
      if (current.length > 0) {
        items.push(current.join("\n"));
      }
      current = [line];
    } else if (current.length > 0) {
      current.push(line);
    }
  }
  if (current.length > 0) {
    items.push(current.join("\n"));
  }
  return items.map((item) => item.replace(/\s+$/u, ""));
}

function typeNameOf(item) {
  const match = item.match(/^pub (?:struct|enum|type) (\w+)/u);
  return match ? match[1] : null;
}

// Rewrites field types both in `pub field: T,` struct lines and inline
// enum variant fields like `Variant { field: T, field: T },`. Inline
// variants are split onto their own lines so per-field doc comments
// satisfy the workspace `missing_docs` lint. Also injects per-field
// `#[serde(...)]` attrs (e.g. `default`, `skip_serializing_if`) when the
// TYPE_CONFIG entry's `fieldSerdeAttrs` declares them.
function applyFieldOverrides(item, overrides, fieldDocs, fieldSerdeAttrs) {
  if (!overrides && !fieldDocs && !fieldSerdeAttrs) return item;
  const lines = item.split("\n");
  const out = [];
  for (const line of lines) {
    const structMatch = line.match(/^(\s*)pub (\w+):\s*(.+?),?\s*$/u);
    if (structMatch) {
      const [, indent, fieldName, originalType] = structMatch;
      const newType = overrideType(overrides, fieldName, originalType);
      if (fieldDocs && fieldDocs[fieldName]) {
        out.push(`${indent}/// ${fieldDocs[fieldName]}`);
      }
      const serdeAttrs = fieldSerdeAttrs && fieldSerdeAttrs[fieldName];
      if (serdeAttrs && serdeAttrs.length > 0) {
        out.push(`${indent}#[serde(${serdeAttrs.join(", ")})]`);
      }
      out.push(`${indent}pub ${fieldName}: ${newType},`);
      continue;
    }
    const inlineVariant = line.match(
      /^(\s*)(\w+)\s*\{\s*(.+?)\s*\}\s*,?\s*$/u,
    );
    if (inlineVariant) {
      const [, indent, variantName, fieldsBlob] = inlineVariant;
      const childIndent = `${indent}    `;
      const fieldEntries = splitVariantFields(fieldsBlob);
      out.push(`${indent}${variantName} {`);
      for (const entry of fieldEntries) {
        const [fieldName, originalType] = entry;
        const newType = overrideType(overrides, fieldName, originalType);
        if (fieldDocs && fieldDocs[fieldName]) {
          out.push(`${childIndent}/// ${fieldDocs[fieldName]}`);
        }
        out.push(`${childIndent}${fieldName}: ${newType},`);
      }
      out.push(`${indent}},`);
      continue;
    }
    out.push(line);
  }
  return out.join("\n");
}

function overrideType(overrides, fieldName, originalType) {
  return overrides && overrides[fieldName]
    ? overrides[fieldName]
    : originalType;
}

// Splits `path: String, start_byte: i64, end_byte: i64` into entries
// while respecting angle-bracket nesting (`Option<List<T>>` etc.).
function splitVariantFields(blob) {
  const entries = [];
  let depth = 0;
  let current = "";
  for (const char of blob) {
    if (char === "<") depth += 1;
    else if (char === ">") depth -= 1;
    if (char === "," && depth === 0) {
      entries.push(current.trim());
      current = "";
      continue;
    }
    current += char;
  }
  if (current.trim().length > 0) entries.push(current.trim());
  return entries.map((entry) => {
    const colon = entry.indexOf(":");
    if (colon < 0) {
      throw new Error(`typediagram-gen: malformed variant field \`${entry}\``);
    }
    return [entry.slice(0, colon).trim(), entry.slice(colon + 1).trim()];
  });
}

function applyVariantDocs(item, variantDocs) {
  if (!variantDocs) return item;
  const lines = item.split("\n");
  const out = [];
  for (const line of lines) {
    const variantMatch = line.match(/^(\s*)(\w+)\s*(\{|\(|=|,|$)/u);
    if (
      variantMatch &&
      variantMatch[1].length > 0 &&
      variantDocs[variantMatch[2]]
    ) {
      out.push(`${variantMatch[1]}/// ${variantDocs[variantMatch[2]]}`);
    }
    out.push(line);
  }
  return out.join("\n");
}

// Collapses single-field struct-form variants `Variant { value: T }`
// (in either inline or expanded multi-line form) into tuple variants
// `Variant(T)` when `tupleVariants` lists the variant name. Required
// for `#[serde(untagged)]` enums where each variant must serialise as
// the bare payload (e.g. JSON-RPC `RequestId` round-tripping bare
// numbers and bare strings). typeDiagram has no native tuple-variant
// syntax so we declare struct-form in the .td and collapse here.
//
// TODO(typeDiagram#24): drop this once tuple-variant syntax lands
// upstream (`union RequestId { Number(Int) String(String) }`).
function applyTupleVariants(item, tupleVariants) {
  if (!tupleVariants) return item;
  const inline = item.replace(
    /^(\s+)(\w+)\s*\{\s*\w+:\s*([^},]+),?\s*\}\s*,?\s*$/gmu,
    (line, indent, variantName, fieldType) =>
      tupleVariants.includes(variantName)
        ? `${indent}${variantName}(${fieldType.trim()}),`
        : line,
  );
  return inline.replace(
    /^(\s+)(\w+)\s*\{\n\s+(?:\/\/\/[^\n]*\n\s+)?\w+:\s*([^,\n]+),\n\s+\},?$/gmu,
    (line, indent, variantName, fieldType) =>
      tupleVariants.includes(variantName)
        ? `${indent}${variantName}(${fieldType.trim()}),`
        : line,
  );
}

// Rewrites unit enum variants `Foo,` to `Foo = <discriminant>,` when
// `variantDiscriminants` declares an explicit value for `Foo`. Idempotent
// no-op when no entry is provided. Lets us declare numeric error codes
// (JSON-RPC -32_700, etc.) on a typeDiagram-defined enum without
// hand-rolling the Rust source.
//
// TODO(typeDiagram#25): drop this once explicit discriminant syntax
// lands upstream (`union ErrorCode { ParseError = -32700, ... }`).
function applyVariantDiscriminants(item, variantDiscriminants) {
  if (!variantDiscriminants) return item;
  return item
    .split("\n")
    .map((line) => {
      const match = line.match(/^(\s+)(\w+),\s*$/u);
      if (!match) return line;
      const [, indent, variantName] = match;
      const disc = variantDiscriminants[variantName];
      if (disc === undefined) return line;
      return `${indent}${variantName} = ${disc},`;
    })
    .join("\n");
}

function decorateItem(item, config) {
  const before = [];
  before.push(`/// ${config.docs}`);
  if (config.derives && config.derives.length > 0) {
    before.push(`#[derive(${config.derives.join(", ")})]`);
  }
  // Enums declaring numeric discriminants must pin a fixed layout so
  // `as i32` casts (used in JsonRpcError::new for ErrorCode) are
  // well-defined. Default to i32 — JSON-RPC error codes fit easily.
  // TODO(typeDiagram#30): drop this auto-injection once `@repr(i32)` is
  // a first-class language attribute upstream.
  if (config.variantDiscriminants) {
    before.push(`#[repr(${config.repr ?? "i32"})]`);
  }
  if (config.serdeAttrs && config.serdeAttrs.length > 0) {
    before.push(`#[serde(${config.serdeAttrs.join(", ")})]`);
  }
  return [...before, item].join("\n");
}

export function postprocess(rust) {
  const items = splitItems(rust);
  const decorated = [];
  const seen = new Set();
  for (const rawItem of items) {
    const name = typeNameOf(rawItem);
    if (!name) continue;
    const config = TYPE_CONFIG[name];
    if (!config) {
      throw new Error(
        `typediagram-gen: missing TYPE_CONFIG entry for \`${name}\`. ` +
          "Add an entry in scripts/typediagram-gen.mjs or remove the type from docs/models/live-ipc.td.",
      );
    }
    seen.add(name);
    let item = rawItem;
    item = applyFieldOverrides(
      item,
      config.fieldOverrides,
      config.fieldDocs,
      config.fieldSerdeAttrs,
    );
    item = applyTupleVariants(item, config.tupleVariants);
    item = applyVariantDiscriminants(item, config.variantDiscriminants);
    item = applyVariantDocs(item, config.variantDocs);
    item = decorateItem(item, config);
    decorated.push(item);
  }
  for (const expected of Object.keys(TYPE_CONFIG)) {
    if (!seen.has(expected)) {
      throw new Error(
        `typediagram-gen: TYPE_CONFIG declares \`${expected}\` but ` +
          "the .td source did not produce it. Update either side.",
      );
    }
  }
  const body = decorated.join("\n\n");
  const externalImports = collectExternalImports(body);
  const header = externalImports.length > 0
    ? `${HEADER_PRELUDE}\n${externalImports.join("\n")}\n`
    : HEADER_PRELUDE;
  return `${header}\n${body}\n`;
}

// Scans the post-processed body for whole-word references to each
// external type and returns the matching `use` lines (sorted, no dups).
// Skips types that are themselves defined in this same generated file
// (TYPE_CONFIG keys) so an in-spec type never collides with a stale
// `crate::report::*` import. Keeps the import block free of
// `unused_imports` warnings without forcing the caller to declare them.
function collectExternalImports(body) {
  const imports = new Set();
  for (const [type, path] of Object.entries(EXTERNAL_TYPES)) {
    if (TYPE_CONFIG[type]) continue;
    const word = new RegExp(`\\b${type}\\b`, "u");
    if (word.test(body)) {
      imports.add(`use ${path};`);
    }
  }
  return [...imports].sort();
}
