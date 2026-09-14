// The VSIX target this host is. The list of published platforms and the rule
// for reading one off the host live once, next to the release matrix they have
// to agree with; this is the extension's name for it. [DEPLOY-PUBLISH-COMPLETE]
import { currentPlatform } from "../../../scripts/release/vsix-platforms.mjs";

/** The `--target` this host's VSIX is built for. */
export function currentPlatformTarget() {
  return currentPlatform();
}
