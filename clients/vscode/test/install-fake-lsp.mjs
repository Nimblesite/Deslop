// Builds a minimal LSP stub that speaks just enough JSON-RPC for the
// activation + notification + query paths we assert on. Run before test launch.

import { writeFileSync, chmodSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dir = resolve(here, "fixtures", "fake-bin");
mkdirSync(dir, { recursive: true });

const body = `#!/usr/bin/env node
const { createInterface } = require("node:readline");
let buf = Buffer.alloc(0);
let contentLength = -1;
process.stdin.on("data", (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  while (true) {
    if (contentLength < 0) {
      const idx = buf.indexOf("\\r\\n\\r\\n");
      if (idx < 0) return;
      const headers = buf.slice(0, idx).toString("utf8");
      const m = /Content-Length: (\\d+)/i.exec(headers);
      if (!m) process.exit(1);
      contentLength = parseInt(m[1], 10);
      buf = buf.slice(idx + 4);
    }
    if (buf.length < contentLength) return;
    const payload = JSON.parse(buf.slice(0, contentLength).toString("utf8"));
    buf = buf.slice(contentLength);
    contentLength = -1;
    handle(payload);
  }
});
function handle(msg) {
  if (msg.method === "initialize") {
    respond(msg.id, { capabilities: { textDocumentSync: 2 } });
  } else if (msg.method === "initialized") {
    // no-op
  } else if (msg.method === "codededup/reportGet") {
    respond(msg.id, sampleReport());
  } else if (msg.method === "codededup/reportDelta") {
    respond(msg.id, null);
  } else if (msg.method === "codededup/duplicatesFindSimilar") {
    respond(msg.id, sampleReport().clusters);
  } else if (msg.method === "codededup/embeddingListModels") {
    const endpoint = process.env.CODEDEDUP_TEST_OLLAMA === "up"
      ? [
          { provider_id: "ollama", model_id: "nomic-embed-text", model_version: "abc",
            dimensions: 768, size_bytes: 137_000_000, is_embedding_model: true },
        ]
      : [];
    respond(msg.id, endpoint.concat([
      { provider_id: "stub", model_id: "stub", model_version: "0",
        dimensions: 64, size_bytes: null, is_embedding_model: true },
    ]));
  } else if (msg.method === "codededup/embeddingSetModel") {
    respond(msg.id, { provider_id: msg.params.provider_id, model_id: msg.params.model_id,
      model_version: "test", dimensions: 64 });
  } else if (msg.method === "shutdown") {
    respond(msg.id, null);
  } else if (msg.method === "exit") {
    process.exit(0);
  } else if (msg.id !== undefined) {
    respond(msg.id, null);
  }
}
function respond(id, result) {
  const payload = JSON.stringify({ jsonrpc: "2.0", id, result });
  process.stdout.write("Content-Length: " + Buffer.byteLength(payload, "utf8") + "\\r\\n\\r\\n" + payload);
}
function sampleReport() {
  return {
    report_schema_version: 3,
    tool_version: "0.1.0-test",
    min_nodes: 30,
    files_analysed: 2,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 2 },
    metrics: { analysed_loc: 120, duplicated_loc: 30, duplication_percent: 25.0,
      clusters_total: 1, duplicated_files: 2,
      threshold: { percent: 0.0, breached: false, source: "none" } },
    schema_doc: "test schema doc",
    action_hints: [],
    embedding_provenance: { provider_id: "stub", model_id: "stub", model_version: "0", dimensions: 64 },
    clusters: [{
      id: "abc123def4567890",
      weight: 12.3,
      size: 50,
      canonical_node_count: 50,
      signals: { structural: 1.0, token_jaccard: 0.97, embedding_cos: 0.92, fused: 0.96 },
      occurrences: [
        { path: "Alpha.cs", start_byte: 0, end_byte: 60, hidden: false },
        { path: "Beta.cs", start_byte: 0, end_byte: 60, hidden: false },
      ],
      summary: "duplicate",
      interpretation: "Type-1 exact clone between Alpha.cs and Beta.cs",
    }],
  };
}
`;
const lspPath = resolve(dir, "codededup-lsp");
writeFileSync(lspPath, body);
chmodSync(lspPath, 0o755);
writeFileSync(resolve(dir, "codededup-mcp"), body);
chmodSync(resolve(dir, "codededup-mcp"), 0o755);
console.log("fake LSP installed at", lspPath);
