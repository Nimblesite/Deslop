#!/usr/bin/env node
// Re-register the packaged VSIX in every VS Code profile that had it.
// [DEPLOY-VSIX-INSTALL-PROFILES]
//
// VS Code unpacks an extension once, into a single directory under
// `~/.vscode/extensions`, and every profile that has the extension points at
// that one directory. `_vsix-install-code` deletes those directories before it
// installs, so a higher Marketplace version cannot keep winning — and
// `code --install-extension` then writes the new directory into the *default*
// profile's registry alone. Every other profile is left naming a directory the
// delete removed, so VS Code reports "Unable to read file
// '.../nimblesite.deslop-live-<version>/package.json'" and the extension is
// dead there until someone reinstalls it by hand. The install has to name
// every profile that had the extension, which is what this script does.
//
// Profiles that never had Deslop.live are left alone: repairing an install is
// not a licence to add one where the developer did not ask for it.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir, platform } from "node:os";
import { isAbsolute, join } from "node:path";
import { parseArgs } from "node:util";

/** The extension whose install this script repairs. */
const EXTENSION_ID = "nimblesite.deslop-live";

/** VS Code CLI flags. */
const INSTALL_FLAG = "--install-extension";
const FORCE_FLAG = "--force";
const PROFILE_FLAG = "--profile";

/** Where VS Code records what each profile has installed. */
const REGISTRY_NAME = "extensions.json";
const PROFILES_DIRECTORY = join("User", "profiles");
const STORAGE_FILE = join("User", "globalStorage", "storage.json");
const STORAGE_PROFILES_KEY = "userDataProfiles";

/** How the default profile prints; it takes no `--profile` flag. */
const DEFAULT_PROFILE_LABEL = "default";

/** Per-platform home of the VS Code user data directory. */
const USER_DATA_BY_PLATFORM = {
  darwin: join("Library", "Application Support", "Code"),
  win32: join("AppData", "Roaming", "Code"),
};
const USER_DATA_FALLBACK = join(".config", "Code");

/** The extensions directory VS Code unpacks into, under the home directory. */
const EXTENSIONS_DIRECTORY = join(".vscode", "extensions");

const WINDOWS = "win32";
const FAILURE = 1;

/** Parsed JSON at `path`, or undefined when it is absent or unreadable. */
function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
}

/** The directory the registry at `path` records for Deslop.live, if any. */
function recordedFolder(path) {
  const registry = readJson(path);
  if (!Array.isArray(registry)) return undefined;
  const entry = registry.find((candidate) => candidate?.identifier?.id === EXTENSION_ID);
  const folder = entry?.location?.path;
  return typeof folder === "string" ? folder : undefined;
}

/** Every named profile VS Code knows about, as `{name, registry}`. */
function namedProfiles(userData) {
  const storage = readJson(join(userData, STORAGE_FILE));
  const profiles = storage?.[STORAGE_PROFILES_KEY];
  if (!Array.isArray(profiles)) return [];
  return profiles.flatMap((profile) => {
    const { name, location } = profile ?? {};
    if (typeof name !== "string" || typeof location !== "string") return [];
    const root = isAbsolute(location) ? location : join(userData, PROFILES_DIRECTORY, location);
    return [{ name, registry: join(root, REGISTRY_NAME) }];
  });
}

/**
 * The profiles this install must write to: the default one always, plus every
 * named profile whose registry already lists the extension.
 */
function installTargets(options) {
  const fallback = { name: undefined, registry: join(options.extensions, REGISTRY_NAME) };
  const named = namedProfiles(options.userData)
    .filter((profile) => recordedFolder(profile.registry) !== undefined);
  return [fallback, ...named];
}

/** How a profile prints, and the flags that address it. */
function label(profile) {
  return profile.name ?? DEFAULT_PROFILE_LABEL;
}

function profileFlags(profile) {
  return profile.name === undefined ? [] : [PROFILE_FLAG, profile.name];
}

/** Install `vsix` into one profile; true when the CLI reported success. */
function install(options, profile) {
  const args = [...profileFlags(profile), INSTALL_FLAG, options.vsix, FORCE_FLAG];
  const result = spawnSync(options.code, args, {
    stdio: "inherit",
    shell: platform() === WINDOWS,
  });
  return result.status === 0;
}

/** Every profile whose recorded directory is not on disk. */
function dangling(options) {
  return installTargets(options)
    .map((profile) => ({ profile, folder: recordedFolder(profile.registry) }))
    .filter((state) => state.folder !== undefined && !existsSync(state.folder));
}

/** Report what each profile records without changing anything. */
function report(options) {
  const broken = dangling(options);
  for (const state of broken) {
    console.log(`${label(state.profile)}: names a directory that is not there`);
  }
  if (broken.length === 0) console.log("every profile with Deslop.live resolves to a real directory");
  return broken.length === 0;
}

/** Install into every target, then prove no profile is left dangling. */
function installEverywhere(options) {
  const failed = installTargets(options).filter((profile) => !install(options, profile));
  for (const profile of failed) {
    console.error(`FAIL: ${options.code} could not install into profile ${label(profile)}`);
  }
  const broken = dangling(options);
  for (const state of broken) {
    console.error(`FAIL: profile ${label(state.profile)} still names a directory that is not there`);
  }
  return failed.length === 0 && broken.length === 0;
}

function options() {
  const home = homedir();
  const { values, positionals } = parseArgs({
    options: {
      list: { type: "boolean", default: false },
      code: { type: "string", default: "code" },
      "user-data": { type: "string", default: join(home, USER_DATA_BY_PLATFORM[platform()] ?? USER_DATA_FALLBACK) },
      extensions: { type: "string", default: join(home, EXTENSIONS_DIRECTORY) },
    },
    allowPositionals: true,
  });
  return {
    vsix: positionals[0],
    list: values.list,
    code: values.code,
    userData: values["user-data"],
    extensions: values.extensions,
  };
}

function main() {
  const parsed = options();
  if (!parsed.list && parsed.vsix === undefined) {
    console.error("FAIL: name the .vsix to install, or pass --list");
    process.exit(FAILURE);
  }
  if (!(parsed.list ? report(parsed) : installEverywhere(parsed))) process.exit(FAILURE);
}

main();
