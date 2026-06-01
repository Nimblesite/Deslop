import { spawnSync } from "node:child_process";

const target = argValue("--target") ?? currentTarget();
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

function currentTarget() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32" && process.arch === "x64") return "win32-x64";
  throw new Error(`unsupported platform ${process.platform}-${process.arch}`);
}
