// Stamps every project-owned version file in this workspace to one explicit
// version. Release jobs use the tag version; tests can pass any semver to prove
// the deployment graph is stamped consistently without committing the result.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const nodeProjects = [
  "clients/vscode",
  "clients/vscode/webview-ui",
  "site",
];
// The VS Code Marketplace forbids SemVer prerelease/build suffixes in the
// extension version field, so the VSIX package (clients/vscode/package.json
// and its lockfile) carries the core MAJOR.MINOR.PATCH only. Every other
// project keeps the full version; prerelease status is conveyed by the
// `--pre-release` flag at publish time. Spec: [DEPLOY-VSCE-MARKETPLACE].
const marketplaceProjects = new Set(["clients/vscode"]);
const stagedDeploymentManifests = ["clients/vscode/shipwright.json"];
// GitHub renders README.md as the body of the Marketplace listing, and the
// action derives the CLI version from the ref it is pinned to — so an unstamped
// `uses:` pin hands every listing visitor a workflow that installs an older
// release, or fails outright against a tag predating action.yml. The pin is a
// project-owned version reference like any other. Every published surface that
// shows a copy-pasteable pin is listed here, in both locales — a doc page that
// drifts is the same defect as a README that drifts. [ACTION-VERSION]
const actionPinPrefix = "uses: Nimblesite/Deslop@v";
// Every character SemVer permits in a version. A pin is closed by the first
// character outside this set — a space in a YAML snippet, but a backtick where
// the Action doc page quotes a pin inline in prose. Reading to the first space
// instead swallowed the backtick into the version token and dropped it on
// stamping, unterminating the code span. [ACTION-VERSION]
const semverCharacters = new Set("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-+");
/** Published surfaces carrying a copy-pasteable `uses:` pin, in stamp order. */
export const actionPinDocs = [
  "README.md",
  "site/src/docs/github-action.md",
  "site/src/zh/docs/github-action.md",
];

// Importable by the contract test without stamping anything: the pin list and
// its parser are the checkable half of [ACTION-VERSION], and a second copy of
// either would drift from the copy that actually rewrites the files.
if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  const { root, version } = parseArgs(process.argv.slice(2));
  stampReleaseVersion(root, version);
  console.log(`${root}: stamped release version ${version}`);
}

export function stampReleaseVersion(rootPath, versionValue) {
  assertSemver(versionValue);
  stampCargoToml(join(rootPath, "Cargo.toml"), versionValue);
  stampCargoLock(join(rootPath, "Cargo.lock"), versionValue);
  stampDeploymentManifest(join(rootPath, "shipwright.json"), versionValue);
  for (const manifest of stagedDeploymentManifests) {
    const manifestPath = join(rootPath, manifest);
    if (existsSync(manifestPath)) stampDeploymentManifest(manifestPath, versionValue);
  }
  for (const doc of actionPinDocs) stampActionPin(join(rootPath, doc), versionValue);
  for (const project of nodeProjects) {
    const projectVersion = marketplaceProjects.has(project)
      ? marketplaceVersion(versionValue)
      : versionValue;
    stampNodeProject(join(rootPath, project), projectVersion);
  }
}

function stampCargoToml(filePath, versionValue) {
  writeFileSync(
    filePath,
    replaceWorkspaceVersion(readFileSync(filePath, "utf8"), versionValue),
  );
}

function stampCargoLock(filePath, versionValue) {
  writeFileSync(filePath, replaceLockVersions(readFileSync(filePath, "utf8"), versionValue));
}

function stampDeploymentManifest(filePath, versionValue) {
  const manifest = readJson(filePath);
  manifest.product.version = versionValue;
  for (const component of manifest.components ?? []) component.expectedVersion = versionValue;
  writeJson(filePath, manifest);
}

function stampNodeProject(projectRoot, versionValue) {
  stampPackageJson(join(projectRoot, "package.json"), versionValue);
  stampPackageLock(join(projectRoot, "package-lock.json"), versionValue);
}

function stampPackageJson(filePath, versionValue) {
  const packageJson = readJson(filePath);
  packageJson.version = versionValue;
  writeJson(filePath, packageJson);
}

function stampPackageLock(filePath, versionValue) {
  const packageLock = readJson(filePath);
  packageLock.version = versionValue;
  if (packageLock.packages?.[""]) packageLock.packages[""].version = versionValue;
  writeJson(filePath, packageLock);
}

function replaceWorkspaceVersion(text, versionValue) {
  let inWorkspacePackage = false;
  let replaced = false;
  const lines = text.split("\n").map((line) => {
    if (/^\[.*\]$/.test(line)) inWorkspacePackage = line === "[workspace.package]";
    if (inWorkspacePackage && /^version = "/.test(line)) {
      replaced = true;
      return `version = "${versionValue}"`;
    }
    return line;
  });
  if (!replaced) throw new Error("Cargo.toml is missing [workspace.package] version");
  return lines.join("\n");
}

function replaceLockVersions(text, versionValue) {
  // A workspace/path crate is a `[[package]]` block with no `source =` line
  // (registry/git crates carry one). Deriving the set from the lock — rather
  // than a hardcoded crate list — means a newly added workspace crate can never
  // silently desync Cargo.lock from the stamped Cargo.toml and break the
  // release's `cargo build --locked` (the deslop-test-support regression, #248).
  const segments = text.split("[[package]]");
  let stamped = 0;
  const out = segments.map((segment, index) => {
    if (index === 0 || /\nsource = /.test(segment)) return segment;
    const replaced = segment.replace(/(\nversion = ")[^"]+(")/, `$1${versionValue}$2`);
    if (replaced !== segment) stamped++;
    return replaced;
  });
  if (stamped === 0) throw new Error("Cargo.lock has no workspace crates to stamp");
  return out.join("[[package]]");
}

function stampActionPin(filePath, versionValue) {
  writeFileSync(filePath, replaceActionPins(readFileSync(filePath, "utf8"), versionValue));
}

// Rewrites only the version token, preserving anything trailing it on the line
// (a comment, a `with:` continuation), so a documented pin can never be
// truncated by the stamp. Prose that *states* a version alongside its pin —
// action.yml's own `version` doc, release.md [ACTION-VERSION] — is deliberately
// not listed here: stamping those would leave the sentence contradicting itself.
function replaceActionPins(text, versionValue) {
  let stamped = 0;
  const lines = text.split("\n").map((line) => {
    const pin = splitActionPin(line);
    if (pin === undefined) return line;
    stamped++;
    return `${pin.head}${actionPinPrefix}${versionValue}${pin.rest}`;
  });
  if (stamped === 0) throw new Error(`no ${actionPinPrefix} pin to stamp`);
  return lines.join("\n");
}

/**
 * Splits a documented `uses:` line around its pinned version.
 * @param {string} line
 * @returns {{ head: string, version: string, rest: string } | undefined} undefined when the line carries no pin
 */
function splitActionPin(line) {
  const marker = line.indexOf(actionPinPrefix);
  if (marker < 0) return undefined;
  const tail = line.slice(marker + actionPinPrefix.length);
  // `split("")` cuts on UTF-16 code units, so the index addresses `tail`
  // directly even on the localised pages.
  const closed = tail.split("").findIndex((character) => !semverCharacters.has(character));
  const stop = closed < 0 ? tail.length : closed;
  return { head: line.slice(0, marker), version: tail.slice(0, stop), rest: tail.slice(stop) };
}

/**
 * Every version pinned by a `uses:` line in one documented surface.
 * @param {string} text
 * @returns {string[]}
 */
export function readActionPins(text) {
  return text.split("\n").flatMap((line) => splitActionPin(line)?.version ?? []);
}

function parseArgs(args) {
  const versionIndex = args.findIndex((arg) => !arg.startsWith("--"));
  const versionValue = versionIndex >= 0 ? args[versionIndex] : undefined;
  if (!versionValue) throw new Error("usage: stamp-release-version.mjs <version> [--root <path>]");
  const rootFlag = args.indexOf("--root");
  const rootValue = rootFlag >= 0 ? args[rootFlag + 1] : ".";
  if (!rootValue) throw new Error("--root requires a path");
  return { root: isAbsolute(rootValue) ? rootValue : resolve(rootValue), version: versionValue };
}

function readJson(filePath) {
  if (!existsSync(filePath)) throw new Error(`missing ${filePath}`);
  return JSON.parse(readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function marketplaceVersion(versionValue) {
  return versionValue.split(/[-+]/, 1)[0];
}

function assertSemver(value) {
  if (!/^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value)) {
    throw new Error(`${value} is not a semantic version`);
  }
}
