// Shared reader for the extension manifest under test.
// Non-`.test.ts` so the Mocha glob does not load this as a suite.
//
// Every unit suite that validates a package.json contribution parses the
// manifest through this one function, so the read path (and the resolve
// from `out/test/unit` back to the extension root) exists exactly once.

import * as fs from "node:fs";
import * as path from "node:path";

/** A `contributes.commands` entry. */
export interface CommandContribution {
  command: string;
  title: string;
}

/** A `contributes.menus` entry. */
export interface MenuContribution {
  command: string;
  when?: string;
  group?: string;
}

/** A `contributes.configuration.properties` entry. */
export interface ConfigurationProperty {
  default?: unknown;
  description?: string;
  enum?: unknown[];
  type?: string;
}

/** The subset of `package.json` the unit suites assert against. */
export interface PackageContribution {
  name: string;
  publisher: string;
  keywords: string[];
  activationEvents: string[];
  contributes: {
    commands: CommandContribution[];
    menus: Record<string, MenuContribution[]>;
    configuration: {
      properties: Record<string, ConfigurationProperty>;
    };
  };
}

/** Parses the extension's `package.json` as structured data. */
export function extensionPackage(): PackageContribution {
  const packagePath = path.resolve(__dirname, "../../..", "package.json");
  return JSON.parse(fs.readFileSync(packagePath, "utf8")) as PackageContribution;
}
