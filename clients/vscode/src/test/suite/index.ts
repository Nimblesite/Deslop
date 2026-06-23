// E2E harness — discovers every *.e2e.test.js (compiled) under this directory.
import * as path from "node:path";
import { globSync } from "glob";
import Mocha from "mocha";

// [VSIX-TESTING] Coarse E2E harness: discovers and runs the compiled
// *.e2e.test.js suites against fixture workspaces.
export async function run(): Promise<void> {
  const mocha = new Mocha({ ui: "tdd", color: true, timeout: 60_000 });
  const testsRoot = path.resolve(__dirname);
  const files = globSync("**/*.e2e.test.js", { cwd: testsRoot, absolute: true });
  for (const file of files) mocha.addFile(file);
  await new Promise<void>((resolveP, reject) => {
    mocha.run((failures) => (failures > 0 ? reject(new Error(`${failures} failed`)) : resolveP()));
  });
}
