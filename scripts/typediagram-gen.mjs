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
const OUT_RUST = resolve(
  REPO_ROOT,
  "crates/deslop-core/src/wire_generated.rs",
);
const OUT_TS = resolve(
  REPO_ROOT,
  "clients/vscode/src/types/wire-generated.ts",
);
// External TypeScript types (defined in clients/vscode/src/types/report.ts)
// re-imported by the generated TS file when referenced.
const EXTERNAL_TS_TYPES = {
  ReportCluster: "./report",
  EmbeddingProvenance: "./report",
};

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
  FileReport: {
    docs: "File-scoped subset of a report; returned by `report/forFile`.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { path: "PathBuf" },
    fieldDocs: {
      path: "Path the report covers, workspace-relative when possible.",
      clusters: "Clusters whose occurrences touch `path`, byte-range sorted.",
    },
  },
  SessionConfig: {
    docs: "Snapshot of the session's resolved configuration.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: {
      workspace_root: "PathBuf",
      min_nodes: "u32",
      exclusion_config_path: "Option<PathBuf>",
      cache_root: "PathBuf",
    },
    fieldDocs: {
      workspace_root: "Workspace root pinned at session creation.",
      min_nodes: "Subtree-size floor used throughout the session.",
      languages: "Languages with registered parsers in the session.",
      embedding_provenance: "Currently-active embedding provenance, if any.",
      exclusion_config_path: "Optional explicit exclusion-config path.",
      cache_root: "Cache root (`<workspace>/.deslop-cache`).",
      incremental: "Whether the session was created with the incremental cache on.",
    },
  },
  ChangeSummary: {
    docs: "Compact summary of a `ReportDelta` for push notifications.",
    derives: ["Debug", "Clone", "Default", "Serialize", "Deserialize"],
    fieldOverrides: {
      clusters_added: "usize",
      clusters_removed: "usize",
      clusters_updated: "usize",
    },
    fieldDocs: {
      clusters_added: "Number of clusters newly present in the latest generation.",
      clusters_removed: "Number of clusters removed in the latest generation.",
      clusters_updated: "Number of clusters whose payload changed.",
      worst_weight: "Worst (highest) weight in the latest generation, `0.0` when empty.",
    },
  },
  ReportChangedNotification: {
    docs: "Wire payload for the `report/changed` notification.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { generation: "u64" },
    fieldDocs: {
      generation: "New generation that produced the change.",
      summary: "Compact summary suitable for status indicators.",
    },
  },
  EmbeddingPhase: {
    docs: "Phase of the embedding pass surfaced via `deslop/embeddingProgress`.",
    derives: ["Debug", "Clone", "Copy", "Serialize", "Deserialize", "PartialEq", "Eq"],
    serdeAttrs: ['rename_all = "snake_case"'],
    variantDocs: {
      Queued: "User selected a model and the low-priority pass is queued.",
      Starting: "Pass has just begun. `done` is `0`, `total` is populated.",
      Running: "Pass is actively working through provider batches.",
      Complete: "Pass finished successfully. `done == total`.",
      Failed: "Pass aborted with `message`. `done` reflects work before the failure.",
    },
  },
  EmbeddingProgress: {
    docs: "Wire payload for the `deslop/embeddingProgress` notification.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    fieldOverrides: { done: "u64", total: "u64" },
    fieldDocs: {
      phase: "Lifecycle phase.",
      provider_id: "Provider id the swap targets (`ollama`, `stub`).",
      model_id: "Model id the swap targets.",
      done: "Subtrees embedded so far.",
      total: "Total subtrees in the current corpus.",
      message: "Diagnostic message populated only when `phase == Failed`.",
    },
  },
  AnalysisState: {
    docs: "Wire payload for the `analysis/state` notification.",
    derives: ["Debug", "Clone", "Serialize", "Deserialize"],
    serdeAttrs: ['tag = "state"', 'rename_all = "snake_case"'],
    fieldOverrides: { started_at_ms: "u64" },
    variantDocs: {
      Idle: "Scheduler is idle — no pass in flight.",
      Running: "Scheduler is processing a pass started at `started_at_ms`.",
      Errored: "Scheduler is parked on an error; `message` carries the diagnostic.",
    },
    fieldDocs: {
      started_at_ms: "Millisecond timestamp the pass started.",
      message: "Human-readable diagnostic.",
    },
  },
};

// Maps an external type name (referenced from the .td but not defined
// in it) to the `use` path the post-processor must inject. Imports are
// emitted only when the generated code actually references the type so
// `unused_imports` warnings stay quiet.
const EXTERNAL_TYPES = {
  ReportCluster: "crate::report::ReportCluster",
  EmbeddingProvenance: "crate::report::EmbeddingProvenance",
};

const HEADER_PRELUDE = `//! Generated wire-format models for the Deslop live IPC surface.
//!
//! Source: \`docs/models/live-ipc.td\` (typeDiagram).
//! Generator: \`scripts/typediagram-gen.mjs\`.
//!
//! DO NOT EDIT BY HAND. Re-run \`make typediagram-gen\` (or any cargo
//! build) to regenerate. This file is gitignored.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
`;

function runTypediagram(target) {
  const stdout = execFileSync("typediagram", ["--to", target, TD_PATH], {
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
  const body = decorated.join("\n\n");
  const externalImports = collectExternalImports(body);
  const header = externalImports.length > 0
    ? `${HEADER_PRELUDE}\n${externalImports.join("\n")}\n`
    : HEADER_PRELUDE;
  return `${header}\n${body}\n`;
}

// Scans the post-processed body for whole-word references to each
// external type and returns the matching `use` lines (sorted, no dups).
// Keeps the generated file's import block free of `unused_imports`
// warnings without forcing the caller to declare imports up front.
function collectExternalImports(body) {
  const imports = new Set();
  for (const [type, path] of Object.entries(EXTERNAL_TYPES)) {
    const word = new RegExp(`\\b${type}\\b`, "u");
    if (word.test(body)) {
      imports.add(`use ${path};`);
    }
  }
  return [...imports].sort();
}

// Snake-cases an UpperCamelCase identifier (`OpenRange` -> `open_range`).
// Mirrors serde's `rename_all = "snake_case"` rule for variant names.
function toSnakeCase(name) {
  return name
    .replace(/([A-Z])([A-Z][a-z])/gu, "$1_$2")
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase();
}

// Returns the configured serde tag name (e.g. `"state"` for AnalysisState)
// when the type's serdeAttrs declares one, otherwise null.
function tagNameOf(config) {
  if (!config?.serdeAttrs) return null;
  for (const attr of config.serdeAttrs) {
    const match = attr.match(/^tag\s*=\s*"(\w+)"$/u);
    if (match) return match[1];
  }
  return null;
}

function hasSnakeCaseRename(config) {
  if (!config?.serdeAttrs) return false;
  return config.serdeAttrs.some((attr) =>
    /^rename_all\s*=\s*"snake_case"$/u.test(attr),
  );
}

// Post-processes typediagram's TypeScript output: fixes the broken
// `undefined<T>` syntax, rewrites discriminator field names + variant
// values to match what serde emits on the Rust side, and collapses
// unit-only enums into wire-accurate string literal unions.
function postprocessTs(ts) {
  let out = ts;
  out = out.replace(/undefined<([^>]+)>/gu, "$1 | null");
  out = rewriteUnions(out);
  return out;
}

// Rewrites each `export type X = ...` discriminated-union block based on
// the matching TYPE_CONFIG entry. Walks the source line-by-line so the
// rewrite respects multi-line union declarations.
function rewriteUnions(ts) {
  const lines = ts.split("\n");
  const out = [];
  let blockName = null;
  let blockLines = [];
  for (const line of lines) {
    const startMatch = line.match(/^export type (\w+)\s*=\s*$/u);
    if (startMatch) {
      blockName = startMatch[1];
      blockLines = [line];
      continue;
    }
    if (blockName) {
      blockLines.push(line);
      if (line.trim().endsWith(";")) {
        out.push(rewriteUnionBlock(blockName, blockLines));
        blockName = null;
        blockLines = [];
      }
      continue;
    }
    out.push(line);
  }
  if (blockName) out.push(blockLines.join("\n"));
  return out.join("\n");
}

function rewriteUnionBlock(name, blockLines) {
  const config = TYPE_CONFIG[name];
  if (!config) return blockLines.join("\n");
  const tag = tagNameOf(config) ?? "kind";
  const snake = hasSnakeCaseRename(config);
  const variants = blockLines
    .slice(1)
    .map((line) => line.match(/\|\s*\{\s*kind:\s*"(\w+)"([^}]*)\}/u))
    .filter(Boolean);
  if (variants.length === 0) return blockLines.join("\n");
  const isUnitOnly = variants.every((m) => m[2].trim() === "");
  if (isUnitOnly && tag === "kind" && snake) {
    const literals = variants
      .map((m) => `"${snake ? toSnakeCase(m[1]) : m[1]}"`)
      .join(" | ");
    return `export type ${name} = ${literals};`;
  }
  const rewritten = [`export type ${name} =`];
  for (const match of variants) {
    const [, variant, payload] = match;
    const tagValue = snake ? toSnakeCase(variant) : variant;
    rewritten.push(`  | { ${tag}: "${tagValue}"${payload}}`);
  }
  return `${rewritten.join("\n")};`;
}

const TS_HEADER = `// @generated by scripts/typediagram-gen.mjs from docs/models/live-ipc.td
// DO NOT EDIT BY HAND. Re-run \`make typediagram-gen\` to regenerate.
// Per CLAUDE.md the generated wire types are gitignored; the .td file is
// the single source of truth for shapes shared with the Rust transports.
`;

function tsImports(body) {
  const imports = [];
  const grouped = new Map();
  for (const [type, mod] of Object.entries(EXTERNAL_TS_TYPES)) {
    const word = new RegExp(`\\b${type}\\b`, "u");
    if (word.test(body)) {
      if (!grouped.has(mod)) grouped.set(mod, new Set());
      grouped.get(mod).add(type);
    }
  }
  for (const [mod, types] of [...grouped].sort(([a], [b]) => a.localeCompare(b))) {
    imports.push(`import type { ${[...types].sort().join(", ")} } from "${mod}";`);
  }
  return imports;
}

function generateTs() {
  const raw = runTypediagram("typescript");
  const body = postprocessTs(raw);
  const imports = tsImports(body);
  const importBlock = imports.length > 0 ? `${imports.join("\n")}\n\n` : "";
  return `${TS_HEADER}\n${importBlock}${body}`;
}

function main() {
  const rust = postprocess(runTypediagram("rust"));
  mkdirSync(dirname(OUT_RUST), { recursive: true });
  writeFileSync(OUT_RUST, rust, "utf8");
  process.stdout.write(`typediagram-gen: wrote ${OUT_RUST}\n`);

  const ts = generateTs();
  mkdirSync(dirname(OUT_TS), { recursive: true });
  writeFileSync(OUT_TS, ts, "utf8");
  process.stdout.write(`typediagram-gen: wrote ${OUT_TS}\n`);
}

main();
