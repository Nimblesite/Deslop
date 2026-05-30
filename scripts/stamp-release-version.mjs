// Stamps every project-owned version file in this workspace to one explicit
// version. Release jobs use the tag version; tests can pass any semver to prove
// the deployment graph is stamped consistently without committing the result.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";

const workspacePackages = new Set(["deslop", "deslop-core", "deslop-lsp", "deslop-mcp"]);
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

const { root, version } = parseArgs(process.argv.slice(2));
stampReleaseVersion(root, version);
console.log(`${root}: stamped release version ${version}`);

export function stampReleaseVersion(rootPath, versionValue) {
  assertSemver(versionValue);
  stampCargoToml(join(rootPath, "Cargo.toml"), versionValue);
  stampCargoLock(join(rootPath, "Cargo.lock"), versionValue);
  stampDeploymentManifest(join(rootPath, "shipwright.json"), versionValue);
  for (const manifest of stagedDeploymentManifests) {
    const manifestPath = join(rootPath, manifest);
    if (existsSync(manifestPath)) stampDeploymentManifest(manifestPath, versionValue);
  }
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
  let currentPackage = "";
  const seen = new Set();
  const lines = text.split("\n").map((line) => {
    if (line === "[[package]]") currentPackage = "";
    const name = line.match(/^name = "([^"]+)"$/)?.[1];
    if (name) currentPackage = name;
    if (workspacePackages.has(currentPackage) && /^version = "/.test(line)) {
      seen.add(currentPackage);
      return `version = "${versionValue}"`;
    }
    return line;
  });
  for (const packageName of workspacePackages) {
    if (!seen.has(packageName)) throw new Error(`Cargo.lock is missing ${packageName}`);
  }
  return lines.join("\n");
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
