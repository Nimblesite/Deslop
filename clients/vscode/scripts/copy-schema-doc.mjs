import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const vsixRoot = path.resolve(here, "..");
const repoRoot = path.resolve(vsixRoot, "..", "..");
const sourcePath = path.resolve(repoRoot, "docs", "specs", "REPORTING-CONTEXT.md");
const distDir = path.resolve(vsixRoot, "dist");
const outputPath = path.resolve(distDir, "schema_doc.md");

const source = await readFile(sourcePath, "utf8");
await mkdir(distDir, { recursive: true });

let existing = "";
try {
  existing = await readFile(outputPath, "utf8");
} catch {}

if (existing !== source) {
  await writeFile(outputPath, source, "utf8");
}
