#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import { OUT_RUST, OUT_TS, TD_PATH } from "./typediagram-gen/paths.mjs";
import { postprocess } from "./typediagram-gen/rust-postprocess.mjs";
import { postprocessTs, TS_HEADER, tsImports } from "./typediagram-gen/ts-postprocess.mjs";

function runTypediagram(target) {
  const stdout = execFileSync("typediagram", ["--to", target, TD_PATH], {
    encoding: "utf8",
  });
  return stdout;
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
