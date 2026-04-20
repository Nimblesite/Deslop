// Runs mocha directly against compiled unit tests (no VS Code needed).

import Mocha from "mocha";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { globSync } from "glob";

const here = dirname(fileURLToPath(import.meta.url));
const compiled = resolve(here, "..", "out", "test", "test", "unit");
const files = globSync("**/*.unit.test.js", { cwd: compiled, absolute: true });
if (files.length === 0) {
  console.error(`no unit tests compiled at ${compiled}`);
  process.exit(1);
}

const mocha = new Mocha({ ui: "tdd", color: true, timeout: 15_000 });
for (const file of files) mocha.addFile(file);
await new Promise((resolvePromise, reject) => {
  mocha.run((failures) => (failures > 0 ? reject(failures) : resolvePromise(undefined)));
}).catch((n) => {
  console.error(`${n} unit tests failed`);
  process.exit(1);
});
