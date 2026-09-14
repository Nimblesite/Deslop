//! Shared temp-fixture helper for unit suites: a fresh temp directory
//! plus one file path inside it. The two-line `mkdtemp` + `join` pair
//! every test used to copy was the largest scaffolding cluster in the
//! TypeScript corpus ([CI-DESLOP] ledger, gh #397).

import * as fs from "fs";
import * as os from "os";
import * as path from "path";

const UTF8_ENCODING = "utf8";

/** Creates a fresh temp directory and returns it with `fileName`
 * resolved inside, ready for the test to write its fixture. */
export function tempFile(
  prefix: string,
  fileName: string,
): { dir: string; file: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  return { dir, file: path.join(dir, fileName) };
}

/** Runs `body` against a fresh temp directory and removes the directory
 * afterwards — including when `body` throws, so a failed assertion never
 * leaks the fixture into the next test. */
export async function withTempDir<T>(
  prefix: string,
  body: (dir: string) => Promise<T> | T,
): Promise<T> {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  try {
    return await body(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

/** `withTempDir` plus one written file per `[fileName, contents]` entry.
 * `body` resolves any of those names back to its absolute path. */
export function withTempFiles<T>(
  prefix: string,
  entries: readonly (readonly [fileName: string, contents: string])[],
  body: (file: (fileName: string) => string, dir: string) => Promise<T> | T,
): Promise<T> {
  return withTempDir(prefix, (dir) => {
    for (const [fileName, contents] of entries) {
      fs.writeFileSync(path.join(dir, fileName), contents, UTF8_ENCODING);
    }
    return body((fileName) => path.join(dir, fileName), dir);
  });
}

/** The single-file shape most suites need: write one fixture file, hand
 * its path to `body`, then clean up. */
export function withTempFile<T>(
  prefix: string,
  fileName: string,
  contents: string,
  body: (file: string, dir: string) => Promise<T> | T,
): Promise<T> {
  return withTempFiles(prefix, [[fileName, contents]], (file, dir) => body(file(fileName), dir));
}
