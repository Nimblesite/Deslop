// [VSIX-SUITE-EXECUTES] Every entry point that launches the VS Code suite must
// compile it first. `vscode-test` globs `out/**/*.test.js`; an uncompiled `out/`
// matches nothing, so Mocha reports "0 passing" and exits 0 — a green light over
// a suite that never ran. npm only fires `pre<name>` for the EXACT script name,
// so a `precoverage` hook does not guard `npm run coverage:collect`: that is the
// hole this pins. It hid all 472 extension-host assertions from CI, whose vsix
// job runs `make _vsix-coverage` -> `npm run coverage:collect` and nothing else.
//
// Spec: docs/specs/vsix.md [VSIX-SUITE-EXECUTES]. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PACKAGE_JSON_PATH = path.join(HERE, "..", "package.json");

/// The binary that launches a real VS Code and runs the compiled suite.
const SUITE_RUNNER = "vscode-test";
/// The `tsc -p ./` script that populates `out/`.
const COMPILE_SCRIPT = "compile";
/// npm's own lifecycle prefix — `pre<name>` runs before `<name>`.
const PRE_HOOK_PREFIX = "pre";
/// The token sequence that delegates to another npm script.
const NPM_RUN = ["npm", "run"];
/// Shell operators that separate one command from the next in a script body.
const SHELL_SEPARATORS = ["&&", "||", ";", "|", "\n"];
/// Every script known to launch the suite. Pinned so a rename cannot empty the
/// discovered set and leave this file asserting nothing.
const EXPECTED_SUITE_SCRIPTS = ["test", "test:ollama", "coverage:collect"];

const scripts = () => JSON.parse(readFileSync(PACKAGE_JSON_PATH, "utf8")).scripts;

/// Split a script body into bare tokens, so `vscode-test` is matched as a whole
/// command and never as a prefix of `vscode-test-user-data-dir`.
const tokens = (body) => {
  let text = String(body ?? "");
  for (const separator of SHELL_SEPARATORS) text = text.split(separator).join(" ");
  return text.split(" ").filter((token) => token.length > 0);
};

const launchesSuite = (body) => tokens(body).includes(SUITE_RUNNER);

const delegatesTo = (body, target) => {
  const parts = tokens(body);
  const wanted = [...NPM_RUN, target];
  return parts.some((_, index) =>
    wanted.every((word, offset) => parts[index + offset] === word),
  );
};

/// The scripts that invoke the runner themselves. A script that merely delegates
/// to one of these inherits its hook, so only these need one of their own.
const suiteScripts = (all) => Object.keys(all).filter((name) => launchesSuite(all[name]));

test("the suite-launching scripts are exactly the ones this contract knows about", () => {
  const found = suiteScripts(scripts()).sort();
  assert.deepEqual(
    found,
    [...EXPECTED_SUITE_SCRIPTS].sort(),
    `a script now launches ${SUITE_RUNNER} without being pinned here — add it to ` +
      "EXPECTED_SUITE_SCRIPTS and give it a compile hook",
  );
  assert.ok(found.length > 0, "discovered no suite scripts — this test would assert nothing");
});

for (const name of EXPECTED_SUITE_SCRIPTS) {
  test(`\`npm run ${name}\` compiles the suite before launching it`, () => {
    const all = scripts();
    assert.ok(all[name], `package.json has no \`${name}\` script`);

    const hook = `${PRE_HOOK_PREFIX}${name}`;
    assert.ok(
      all[hook],
      `\`${name}\` launches ${SUITE_RUNNER} with no \`${hook}\` hook, so \`out/\` is ` +
        "never built and the run reports 0 passing while exiting 0",
    );
    assert.ok(
      delegatesTo(all[hook], COMPILE_SCRIPT),
      `\`${hook}\` does not run \`npm run ${COMPILE_SCRIPT}\`, so nothing populates \`out/\``,
    );
  });
}

test("the compile script is the one that emits out/, not a type-check", () => {
  const compile = scripts()[COMPILE_SCRIPT];
  assert.ok(compile, `package.json has no \`${COMPILE_SCRIPT}\` script`);
  assert.ok(
    !tokens(compile).includes("--noEmit"),
    `\`${COMPILE_SCRIPT}\` runs with --noEmit, so it type-checks without producing out/`,
  );
});
