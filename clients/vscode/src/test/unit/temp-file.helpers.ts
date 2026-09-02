//! Shared temp-fixture helper for unit suites: a fresh temp directory
//! plus one file path inside it. The two-line `mkdtemp` + `join` pair
//! every test used to copy was the largest scaffolding cluster in the
//! TypeScript corpus ([CI-DESLOP] ledger, gh #397).

import * as fs from "fs";
import * as os from "os";
import * as path from "path";

/** Creates a fresh temp directory and returns it with `fileName`
 * resolved inside, ready for the test to write its fixture. */
export function tempFile(
  prefix: string,
  fileName: string,
): { dir: string; file: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  return { dir, file: path.join(dir, fileName) };
}
