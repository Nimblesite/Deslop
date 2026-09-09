// Profile-aware VSIX install contract. [DEPLOY-VSIX-INSTALL-PROFILES]
//
// `_vsix-install-code` deletes the unpacked extension directory before it
// installs, and that directory is the one every VS Code profile points at.
// `code --install-extension` writes only the default profile's registry, so
// every other profile that had Deslop.live was left naming a directory that
// had just been removed: VS Code reported "Unable to read file
// '.../nimblesite.deslop-live-0.34.0-darwin-arm64/package.json'" and the
// extension was dead in four of the developer's profiles at once, after a
// build that reported success.
//
// These tests drive the real script against a fixture home with a fake `code`
// CLI. The developer's own profiles are never addressed: every spawn is given
// the fixture's user-data and extensions directories, and the CLI it calls is
// the recorder staged below. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { platform, tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { recipeBlocks } from "../lib/makefile.mjs";

/** The script under test, and the target that must delegate to it. */
const SCRIPT = resolve(repoRoot, "scripts/repository/install-vsix-profiles.mjs");
const INSTALL_TARGET = "_vsix-install-code";
const SCRIPT_INVOCATION = "node scripts/repository/install-vsix-profiles.mjs";

/** What the target must no longer do: install without naming the profiles. */
const UNPROFILED_INSTALL = "code --install-extension";

/** The extension the script repairs, spelled as VS Code records it. */
const EXTENSION_ID = "nimblesite.deslop-live";

/** Flags the script and the fake CLI agree on. */
const LIST_FLAG = "--list";
const CODE_FLAG = "--code";
const USER_DATA_FLAG = "--user-data";
const EXTENSIONS_FLAG = "--extensions";
const PROFILE_FLAG = "--profile";
const INSTALL_FLAG = "--install-extension";
const FORCE_FLAG = "--force";

/** The three profiles a fixture home carries, and what each one has. */
const BROKEN_PROFILE = { name: "Rust, Python, .NET", location: "aaaa1111" };
const HEALTHY_PROFILE = { name: "Flutter", location: "bbbb2222" };
const UNRELATED_PROFILE = { name: "React", location: "cccc3333" };

/** How the log records an install that named no profile. */
const DEFAULT_PROFILE_LOGGED = null;

/** The packaged artifact the install is asked to hand VS Code. */
const VSIX_NAME = "deslop-live-darwin-arm64.vsix";

/** Directory names for the two unpacked versions in play: the one the
 * registries were written against and the one an install produces. */
const REMOVED_VERSION_DIRECTORY = "nimblesite.deslop-live-0.34.0-darwin-arm64";
const INSTALLED_VERSION_DIRECTORY = "nimblesite.deslop-live-0.0.0-dev";

/** Fake-CLI behaviours: really register the extension, or only record. */
const MODE_INSTALL = "install";
const MODE_RECORD_ONLY = "record-only";

/** Environment the fake CLI reads. */
const LOG_VARIABLE = "FAKE_CODE_LOG";
const MODE_VARIABLE = "FAKE_CODE_MODE";
const INSTALLED_VARIABLE = "FAKE_CODE_INSTALLED_DIRECTORY";
const REGISTRIES_VARIABLE = "FAKE_CODE_REGISTRIES";

const EXECUTABLE_MODE = 0o755;
const WINDOWS = "win32";
const SUCCESS = 0;
const FAILURE = 1;

/** One registry entry, in the shape VS Code writes. */
function registryFor(folder) {
  return JSON.stringify([{ identifier: { id: EXTENSION_ID }, version: "0.34.0", location: { path: folder } }]);
}

/** A registry for a profile that has never had the extension. */
const REGISTRY_WITHOUT_EXTENSION = JSON.stringify([
  { identifier: { id: "rust-lang.rust-analyzer" }, version: "0.3.3033", location: { path: "elsewhere" } },
]);

function writeJsonFile(path, contents) {
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, contents);
}

/** Absolute path of a profile's registry inside a fixture home. */
function profileRegistry(home, profile) {
  return join(home, "userData", "User", "profiles", profile.location, "extensions.json");
}

/** The fake `code`: records every invocation, and — in install mode —
 * registers the new directory for exactly the profile it was addressed with,
 * which is the behaviour the real CLI has and the reason the bug existed. */
function stageFakeCode(home) {
  const source = join(home, "fake-code.mjs");
  writeFileSync(source, FAKE_CODE_SOURCE);
  if (platform() !== WINDOWS) {
    const shim = join(home, "fake-code");
    writeFileSync(shim, `#!/usr/bin/env node\nimport(${JSON.stringify(source)});\n`);
    chmodSync(shim, EXECUTABLE_MODE);
    return shim;
  }
  const shim = join(home, "fake-code.cmd");
  writeFileSync(shim, `@echo off\r\nnode "${source}" %*\r\n`);
  return shim;
}

const FAKE_CODE_SOURCE = `
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
const argv = process.argv.slice(2);
const flag = (name) => { const at = argv.indexOf(name); return at < 0 ? null : argv[at + 1] ?? null; };
const invocation = {
  profile: flag(${JSON.stringify(PROFILE_FLAG)}),
  vsix: flag(${JSON.stringify(INSTALL_FLAG)}),
  forced: argv.includes(${JSON.stringify(FORCE_FLAG)}),
};
appendFileSync(process.env.${LOG_VARIABLE}, JSON.stringify(invocation) + "\\n");
if (process.env.${MODE_VARIABLE} === ${JSON.stringify(MODE_INSTALL)}) {
  const directory = process.env.${INSTALLED_VARIABLE};
  mkdirSync(directory, { recursive: true });
  const registries = JSON.parse(process.env.${REGISTRIES_VARIABLE});
  const registry = registries[invocation.profile ?? ""];
  if (registry !== undefined) {
    writeFileSync(registry, JSON.stringify([
      { identifier: { id: ${JSON.stringify(EXTENSION_ID)} }, version: "0.0.0-dev", location: { path: directory } },
    ]));
  }
}
`;

/**
 * A fixture home: a default profile and three named profiles, two of which
 * have the extension and one of which does not. Every registry that has the
 * extension names `REMOVED_VERSION_DIRECTORY`; `healthyExists` decides whether
 * that directory is on disk for the healthy profile.
 */
function stageHome() {
  const home = mkdtempSync(join(tmpdir(), "deslop-install-profiles-"));
  const extensions = join(home, "extensions");
  const removed = join(home, "unpacked", REMOVED_VERSION_DIRECTORY);
  writeJsonFile(join(extensions, "extensions.json"), registryFor(removed));
  writeJsonFile(profileRegistry(home, BROKEN_PROFILE), registryFor(removed));
  writeJsonFile(profileRegistry(home, HEALTHY_PROFILE), registryFor(removed));
  writeJsonFile(profileRegistry(home, UNRELATED_PROFILE), REGISTRY_WITHOUT_EXTENSION);
  writeJsonFile(
    join(home, "userData", "User", "globalStorage", "storage.json"),
    JSON.stringify({ userDataProfiles: [BROKEN_PROFILE, HEALTHY_PROFILE, UNRELATED_PROFILE] }),
  );
  return { home, extensions, log: join(home, "code.log"), installed: join(home, "unpacked", INSTALLED_VERSION_DIRECTORY) };
}

/** Every registry the fake CLI is allowed to rewrite, keyed by profile flag. */
function registryMap(fixture) {
  return JSON.stringify({
    "": join(fixture.extensions, "extensions.json"),
    [BROKEN_PROFILE.name]: profileRegistry(fixture.home, BROKEN_PROFILE),
    [HEALTHY_PROFILE.name]: profileRegistry(fixture.home, HEALTHY_PROFILE),
    [UNRELATED_PROFILE.name]: profileRegistry(fixture.home, UNRELATED_PROFILE),
  });
}

/** Run the script against one fixture home. */
function run(fixture, mode, extraArguments) {
  return spawnSync(
    process.execPath,
    [
      SCRIPT,
      ...extraArguments,
      CODE_FLAG, stageFakeCode(fixture.home),
      USER_DATA_FLAG, join(fixture.home, "userData"),
      EXTENSIONS_FLAG, fixture.extensions,
    ],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        [LOG_VARIABLE]: fixture.log,
        [MODE_VARIABLE]: mode,
        [INSTALLED_VARIABLE]: fixture.installed,
        [REGISTRIES_VARIABLE]: registryMap(fixture),
      },
    },
  );
}

/** Every invocation the fake CLI recorded, in order. */
function invocations(fixture) {
  if (!existsSync(fixture.log)) return [];
  return readFileSync(fixture.log, "utf8").split("\n").filter((line) => line.length > 0).map((line) => JSON.parse(line));
}

/** What a registry now names, and whether that is on disk. */
function recorded(path) {
  const folder = JSON.parse(readFileSync(path, "utf8"))
    .find((entry) => entry.identifier.id === EXTENSION_ID)?.location.path;
  return { folder, exists: folder !== undefined && existsSync(folder) };
}

test("the install names every profile that had the extension, and no profile that did not", () => {
  const fixture = stageHome();
  const result = run(fixture, MODE_INSTALL, [join(fixture.home, VSIX_NAME)]);

  assert.equal(result.status, SUCCESS, result.stderr);
  const addressed = invocations(fixture);
  assert.deepEqual(
    addressed.map((invocation) => invocation.profile),
    [DEFAULT_PROFILE_LOGGED, BROKEN_PROFILE.name, HEALTHY_PROFILE.name],
  );
  assert.ok(addressed.every((invocation) => invocation.vsix === join(fixture.home, VSIX_NAME)));
  assert.ok(addressed.every((invocation) => invocation.forced));
  assert.ok(!addressed.some((invocation) => invocation.profile === UNRELATED_PROFILE.name));

  assert.deepEqual(recorded(join(fixture.extensions, "extensions.json")), { folder: fixture.installed, exists: true });
  assert.deepEqual(recorded(profileRegistry(fixture.home, BROKEN_PROFILE)), { folder: fixture.installed, exists: true });
  assert.deepEqual(recorded(profileRegistry(fixture.home, HEALTHY_PROFILE)), { folder: fixture.installed, exists: true });
  assert.deepEqual(JSON.parse(readFileSync(profileRegistry(fixture.home, UNRELATED_PROFILE), "utf8")),
    JSON.parse(REGISTRY_WITHOUT_EXTENSION));
});

test("an install that leaves a profile naming a directory that is not there fails", () => {
  const fixture = stageHome();
  const result = run(fixture, MODE_RECORD_ONLY, [join(fixture.home, VSIX_NAME)]);

  assert.equal(result.status, FAILURE);
  assert.ok(result.stderr.includes(BROKEN_PROFILE.name), result.stderr);
  assert.ok(result.stderr.includes(HEALTHY_PROFILE.name), result.stderr);
  assert.ok(!result.stderr.includes(UNRELATED_PROFILE.name), result.stderr);
  assert.equal(invocations(fixture).length, 3);
});

test("--list reports the profiles that are broken and installs nothing", () => {
  const fixture = stageHome();
  const result = run(fixture, MODE_INSTALL, [LIST_FLAG]);

  assert.equal(result.status, FAILURE);
  assert.ok(result.stdout.includes(BROKEN_PROFILE.name), result.stdout);
  assert.deepEqual(invocations(fixture), []);
});

test("--list passes once every profile with the extension resolves", () => {
  const fixture = stageHome();
  mkdirSync(join(fixture.home, "unpacked", REMOVED_VERSION_DIRECTORY), { recursive: true });
  const result = run(fixture, MODE_INSTALL, [LIST_FLAG]);

  assert.equal(result.status, SUCCESS, result.stderr);
  assert.deepEqual(invocations(fixture), []);
});

test("the install target delegates to this script instead of installing unprofiled", () => {
  const blocks = recipeBlocks(INSTALL_TARGET);
  assert.equal(blocks.length, 1);
  assert.ok(blocks[0].body.includes(SCRIPT_INVOCATION), blocks[0].body);
  assert.ok(!blocks[0].body.includes(UNPROFILED_INSTALL), blocks[0].body);
});
