#!/usr/bin/env node
// Generates Rust IPC model code from `docs/models/live-ipc.td` using the
// typediagram CLI (https://typediagram.dev/docs/cli.html), then post-
// processes the output to satisfy the Deslop workspace lints (serde
// derives, doc comments, precise integer widths, serde tag attributes,
// import statements). The .td file is the single source of truth; the
// emitted `.rs` is gitignored and rebuilt on every `cargo build` via
// `crates/deslop-core/build.rs`.
//
// Per CLAUDE.md: "ALL MODELS TRANSFERRED ACROSS THE WIRE MUST USE
// typeDiagram. NO IFS. NO BUTS." This script is the build-side adapter
// from the typediagram CLI's bare struct output to wire-ready Rust.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..");
const TD_PATH = resolve(REPO_ROOT, "docs/models/live-ipc.td");
const OUT_PATH = resolve(
  REPO_ROOT,
  "crates/deslop-core/src/wire_generated.rs",
);

// Per-type generation hints. Drives the post-processor: every entry maps
// a type name (struct, enum, or alias) to the derives, serde attrs,
// field-type overrides, and crate-level `use` lines required to make
// the bare typediagram output compile against the existing wire shape.
const TYPE_CONFIG = {
  OllamaModelInfo: {
    docs: "One row from the Ollama `/api/tags` enumeration. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { size_bytes: "u64" },
    fieldDocs: {
      name: "Full model tag as installed (`nomic-embed-text:latest`).",
      bare_id: "Tag-stripped model id.",
      digest: "Truncated content digest (12 hex chars).",
      size_bytes: "Packaged model size in bytes.",
      is_embedding_model: "True when a probe returned a non-empty vector.",
    },
  },
  EmbeddingModelInfo: {
    docs: "One row of the `embedding/listModels` response. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { dimensions: "Option<usize>" },
    fieldDocs: {
      provider_id: "Provider registry key (`ollama`, `stub`).",
      model_id: "Human-readable model id.",
      model_version: "Optional opaque version string.",
      dimensions: "Optional dimensionality, when known.",
      recommended: "True when recommended for code embeddings.",
      reachable: "True when the provider was reachable at listing time.",
    },
  },
  FindSimilarInput: {
    docs: "Discriminated input to `duplicates/findSimilar`. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    serdeAttrs: ['tag = "kind"', 'rename_all = "snake_case"'],
    fieldOverrides: {
      path: "PathBuf",
      start_byte: "usize",
      end_byte: "usize",
    },
    variantDocs: {
      OpenRange: "Look up clusters overlapping a byte range in an open file.",
      Snippet: "Parse a snippet against a registered language and look up.",
    },
    fieldDocs: {
      path: "Workspace-relative or absolute path.",
      start_byte: "Inclusive byte offset of the range start.",
      end_byte: "Exclusive byte offset of the range end.",
      snippet: "Source-text snippet to fingerprint.",
      language: "Registered language id (`csharp`, `rust`, `python`).",
    },
  },
  FindSimilarRequest: {
    docs: "Outer envelope for `duplicates/findSimilar` requests. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { max_results: "Option<usize>" },
    fieldDocs: {
      input: "Discriminated input variant.",
      max_results: "Optional cap on returned clusters; `None` means no cap.",
    },
  },
  FindSimilarResult: {
    docs: "Result of `duplicates/findSimilar`. See `docs/models/live-ipc.td`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldDocs: {
      clusters: "Top-N clusters covering the input, worst-first.",
      below_min_nodes:
        "True when every subtree fell below the session's `min_nodes` floor.",
    },
  },
};

const HEADER = `//! Generated wire-format models for the Deslop live IPC surface.
//!
//! Source: \`docs/models/live-ipc.td\` (typeDiagram).
//! Generator: \`scripts/typediagram-gen.mjs\`.
//!
//! DO NOT EDIT BY HAND. Re-run \`make typediagram-gen\` (or any cargo
//! build) to regenerate. This file is gitignored.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::report::ReportCluster;
`;

function runTypediagram() {
  const stdout = execFileSync("typediagram", ["--to", "rust", TD_PATH], {
    encoding: "utf8",
  });
  return stdout;
}

// Splits the bare typediagram output into top-level items. typediagram
// emits one blank line between items; we anchor on the leading `pub `.
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
// satisfy the workspace `missing_docs` lint.
function applyFieldOverrides(item, overrides, fieldDocs) {
  if (!overrides && !fieldDocs) return item;
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
    const variantMatch = line.match(/^(\s*)(\w+)\s*(\{|,|$)/u);
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

function decorateItem(item, config) {
  const before = [];
  before.push(`/// ${config.docs}`);
  if (config.derives && config.derives.length > 0) {
    before.push(`#[derive(${config.derives.join(", ")})]`);
  }
  if (config.serdeAttrs && config.serdeAttrs.length > 0) {
    before.push(`#[serde(${config.serdeAttrs.join(", ")})]`);
  }
  return [...before, item].join("\n");
}

function postprocess(rust) {
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
    item = applyFieldOverrides(item, config.fieldOverrides, config.fieldDocs);
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
  return `${HEADER}\n${decorated.join("\n\n")}\n`;
}

function main() {
  const raw = runTypediagram();
  const out = postprocess(raw);
  mkdirSync(dirname(OUT_PATH), { recursive: true });
  writeFileSync(OUT_PATH, out, "utf8");
  process.stdout.write(`typediagram-gen: wrote ${OUT_PATH}\n`);
}

main();
