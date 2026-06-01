// Unit: package contribution checks for VS Code menus. Reads package.json
// through JSON.parse so structured contributions stay validated as data.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";

interface PackageContribution {
  name: string;
  publisher: string;
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
  test("extension id stays aligned with the released VSIX id", () => {
    const pkg = extensionPackage();
    assert.equal(pkg.publisher, "nimblesite");
    assert.equal(pkg.name, "deslop-live");
  });

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

  test("compare with canonical menus are hidden for canonical-only rows (#14)", () => {
    const pkg = extensionPackage();
    const contextItems = pkg.contributes.menus["view/item/context"];
    assert.ok(contextItems, "view/item/context menu must be contributed");

    const clusterCompareItems = contextItems.filter(
      (item) =>
        item.when === "viewItem == deslop.clusterComparable" &&
        item.command === "deslop.compareWithCanonical",
    );
    assert.equal(commandTitle(pkg, "deslop.compareWithCanonical"), "Compare With Canonical");
    assert.deepEqual(
      clusterCompareItems.map((item) => item.group),
      ["navigation@4"],
    );
    assert.ok(
      !contextItems.some(
        (item) =>
          item.command === "deslop.compareWithCanonical" &&
          item.when?.includes("clusterSingle"),
      ),
      "single-occurrence cluster rows must not expose compare with canonical",
    );
    assert.ok(
      !contextItems.some(
        (item) =>
          item.command === "deslop.compareOccurrenceWithCanonical" &&
          item.when?.includes("occurrenceCanonical"),
      ),
      "canonical occurrence rows must not expose compare with canonical",
    );
  });

  test("CPU report command is contributed for issue #29 diagnostics", () => {
    const pkg = extensionPackage();
    assert.equal(
      commandTitle(pkg, "deslop.revealCpuReport"),
      "Deslop: Reveal CPU Report",
    );
  });
});
