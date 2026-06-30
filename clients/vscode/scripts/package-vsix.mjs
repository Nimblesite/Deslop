import { spawnSync } from "node:child_process";

import { currentPlatformTarget } from "./platform.mjs";

const target = argValue("--target") ?? currentPlatformTarget();
const output = argValue("--out") ?? `deslop-live-${target}.vsix`;

run("npm", ["run", "build"]);
run("npx", ["vsce", "package", "--no-dependencies", "--target", target, "-o", output]);
run("node", ["./scripts/assert-vsix-schema-doc.mjs", output]);
run("node", ["./scripts/verify-vsix-package.mjs", output, target]);

function argValue(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return undefined;
  const value = process.argv[index + 1];
  if (!value || value.startsWith("--")) throw new Error(`${name} requires a value`);
  return value;
}

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit", shell: process.platform === "win32" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
