import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const vsixRoot = path.resolve(here, "..");
const repoRoot = path.resolve(vsixRoot, "..", "..");
const sourcePath = path.resolve(repoRoot, "docs", "specs", "REPORTING-CONTEXT.md");
const vsixArg = process.argv[2] ?? "deslop-vscode.vsix";
const vsixPath = path.isAbsolute(vsixArg) ? vsixArg : path.resolve(vsixRoot, vsixArg);
const entryPath = "extension/dist/schema_doc.md";

function unzipText(args) {
  try {
    return execFileSync("unzip", args, { encoding: "utf8" });
  } catch (error) {
    throw new Error(`Failed to inspect ${vsixPath} with unzip: ${String(error)}`);
  }
}

const entries = unzipText(["-Z1", vsixPath]).split("\n");
if (!entries.includes(entryPath)) {
  throw new Error(`Missing ${entryPath} in ${vsixPath}`);
}

const source = readFileSync(sourcePath, "utf8");
const packaged = unzipText(["-p", vsixPath, entryPath]);
if (packaged !== source) {
  throw new Error(`${entryPath} in ${vsixPath} does not match ${sourcePath}`);
}

console.log(`Verified ${entryPath} in ${vsixPath}`);
