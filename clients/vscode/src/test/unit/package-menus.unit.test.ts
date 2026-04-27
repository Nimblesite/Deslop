// Unit: package contribution checks for VS Code menus. Reads package.json
// through JSON.parse so structured contributions stay validated as data.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";

interface PackageContribution {
  activationEvents: string[];
  contributes: {
    commands: CommandContribution[];
    menus: Record<string, MenuContribution[]>;
  };
}

interface CommandContribution {
  command: string;
  title: string;
}

interface MenuContribution {
  command: string;
  when?: string;
  group?: string;
}

function extensionPackage(): PackageContribution {
  const packagePath = path.resolve(__dirname, "../../..", "package.json");
  const text = fs.readFileSync(packagePath, "utf8");
  return JSON.parse(text) as PackageContribution;
}

function commandTitle(pkg: PackageContribution, command: string): string | undefined {
  return pkg.contributes.commands.find((item) => item.command === command)?.title;
}

suite("package menu contributions", () => {
  test("activationEvents includes onStartupFinished so analysis begins at VS Code startup", () => {
    const pkg = extensionPackage();
    assert.ok(
      pkg.activationEvents.includes("onStartupFinished"),
      `activationEvents must include "onStartupFinished" — analysis must not wait for panel open. Got: ${JSON.stringify(pkg.activationEvents)}`,
    );
  });

  test("occurrence context menu compares with canonical instead of opening", () => {
    const pkg = extensionPackage();
    const contextItems = pkg.contributes.menus["view/item/context"];
    assert.ok(contextItems, "view/item/context menu must be contributed");
    const occurrenceItems = contextItems.filter(
      (item) => item.when === "viewItem == deslop.occurrence",
    );

    assert.deepEqual(
      occurrenceItems
        .filter((item) => item.group === "navigation@1")
        .map((item) => item.command),
      ["deslop.compareOccurrenceWithCanonical"],
    );
    assert.equal(
      commandTitle(pkg, "deslop.compareOccurrenceWithCanonical"),
      "Compare With Canonical",
    );
    assert.ok(!occurrenceItems.some((item) => item.command === "deslop.openOccurrence"));
  });
});
