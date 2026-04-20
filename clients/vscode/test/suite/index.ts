// Test harness — discovers every *.test.ts under this directory.
import * as path from "node:path";
import { glob } from "tinyglobby";
import Mocha from "mocha";

export async function run(): Promise<void> {
  const mocha = new Mocha({ ui: "tdd", color: true, timeout: 60_000 });
  const testsRoot = path.resolve(__dirname);
  const files = await glob("**/*.test.js", { cwd: testsRoot, absolute: true });
  for (const file of files) mocha.addFile(file);
  await new Promise<void>((done, reject) => {
    mocha.run((failures) => (failures > 0 ? reject(new Error(`${failures} failed`)) : done()));
  });
}
