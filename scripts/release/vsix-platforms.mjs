// [DEPLOY-PUBLISH-COMPLETE] The platform VSIXes a release must publish.
//
// One list, named once. The release workflow's build matrix produces exactly
// these `vsix_target` legs, and scripts/release/test-release-publish-contract.mjs
// asserts the two agree — so adding a sixth platform to the matrix without
// updating this list fails in CI rather than shipping a five-of-six release.

/** Every `vsix_target` the release build matrix produces, sorted. */
export const VSIX_PLATFORMS = [
  "darwin-arm64",
  "darwin-x64",
  "linux-arm64",
  "linux-x64",
  "win32-x64",
];

/** The workflow uploads each platform's VSIX under `vsix-<platform>/`. */
export const VSIX_ARTIFACT_PREFIX = "vsix-";

/** The build matrix key whose values are the platforms above. */
export const VSIX_MATRIX_KEY = "vsix_target";

/** The one platform whose executables carry a file extension. */
const EXECUTABLE_SUFFIXES = new Map([["win32", ".exe"]]);

/**
 * How `platform` spells the executable called `name`. [DEPLOY-BINARY-FILE-NAME]
 *
 * Named once because it is a contract, not a convenience: the file a verifier
 * looks for, the file a package must bundle, and the file `make build` leaves
 * in `target/release` are the same file, and a copy of this rule that drifts —
 * or, as happened to the action gate, is simply left out — turns into a gate
 * that reports a missing binary on the one platform that has it.
 *
 * @param {string} name the binary's name, without any extension
 * @param {string} platform a `<os>-<arch>` target from the list above
 * @returns {string}
 */
export function executableName(name, platform) {
  return `${name}${EXECUTABLE_SUFFIXES.get(platform.split("-")[0]) ?? ""}`;
}

/**
 * The published platform this host is, refusing to guess when it is not one.
 *
 * @returns {string} a member of `VSIX_PLATFORMS`
 */
export function currentPlatform() {
  const target = `${process.platform}-${process.arch}`;
  if (!VSIX_PLATFORMS.includes(target)) throw new Error(`unsupported platform ${target}`);
  return target;
}
