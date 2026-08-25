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
