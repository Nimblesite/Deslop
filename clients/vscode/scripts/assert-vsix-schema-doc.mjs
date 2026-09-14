import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { openArchive } from "../../../scripts/lib/zip.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const vsixRoot = path.resolve(here, "..");
const repoRoot = path.resolve(vsixRoot, "..", "..");
const sourcePath = path.resolve(repoRoot, "docs", "specs", "REPORTING-CONTEXT.md");
const vsixArg = process.argv[2] ?? "deslop-live.vsix";
const vsixPath = path.isAbsolute(vsixArg) ? vsixArg : path.resolve(vsixRoot, vsixArg);
const entryPath = "extension/dist/schema_doc.md";

const archive = openArchive(vsixPath);
if (!archive.names.includes(entryPath)) {
  throw new Error(`Missing ${entryPath} in ${vsixPath}`);
}

const source = readFileSync(sourcePath, "utf8");
const packaged = archive.readText(entryPath);
if (packaged !== source) {
  throw new Error(`${entryPath} in ${vsixPath} does not match ${sourcePath}`);
}

console.log(`Verified ${entryPath} in ${vsixPath}`);
