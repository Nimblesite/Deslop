// Putting a file somewhere, including where the directory does not exist yet.
//
// Node has no "write this file, making the folders on the way" call, so every
// script that stages a fixture tree writes the same two statements: create the
// parent, then write into it. Named once because the pair is easy to get half
// right — a write whose parent is missing throws ENOENT with the *file* named,
// which reads as a broken test rather than a missing `mkdir`.

import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

/**
 * Writes `content` to `path`, creating every directory leading to it.
 *
 * @param {string} path where the file goes
 * @param {string | Buffer} content what it holds
 * @param {object} [options] any `writeFileSync` options, such as `mode`
 * @returns {string} `path`
 */
export function writeFileAt(path, content, options) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content, options);
  return path;
}

/**
 * Copies `source` to `destination`, creating every directory leading to it.
 *
 * @param {string} source the file to copy
 * @param {string} destination where the copy goes
 * @returns {string} `destination`
 */
export function copyFileAt(source, destination) {
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(source, destination);
  return destination;
}
