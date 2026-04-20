// E2E entry point: downloads a sandbox VS Code, installs the extension, runs
// the mocha-style tests in ./suite against fixture workspaces.

import { runTests } from "@vscode/test-electron";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const extensionDevelopmentPath = resolve(here, "..");
const extensionTestsPath = resolve(here, "suite", "index.js");
const fixture = resolve(here, "fixtures", "csharp-small");

const exitCode = await runTests({
  extensionDevelopmentPath,
  extensionTestsPath,
  launchArgs: ["--disable-extensions", "--new-window", fixture],
  extensionTestsEnv: {
    CODEDEDUP_TEST_FIXTURE: fixture,
    CODEDEDUP_BINARY_DIR: resolve(here, "fixtures", "fake-bin"),
  },
});
process.exit(exitCode);
