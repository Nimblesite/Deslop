// Unit: package contribution checks for VS Code menus. Reads package.json
// through JSON.parse so structured contributions stay validated as data.

import * as assert from "node:assert/strict";

import { extensionPackage, PackageContribution } from "./package.helpers";

function commandTitle(pkg: PackageContribution, command: string): string | undefined {
  return pkg.contributes.commands.find((item) => item.command === command)?.title;
}

function navigationOrder(group?: string): number {
  const match = /navigation@(\d+)/.exec(group ?? "");
  return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
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

  test("Expand All and Collapse All are adjacent Top Offenders title actions", () => {
    // [VSIX-TOP-OFFENDERS-TOOLBAR] The two bulk actions must sit next to each
    // other in the title bar, with no other action between them.
    const pkg = extensionPackage();
    const titleItems = (pkg.contributes.menus["view/title"] ?? []).filter((item) =>
      item.when?.includes("view == deslop.topOffenders"),
    );
    const ordered = titleItems
      .filter((item) => navigationOrder(item.group) !== Number.MAX_SAFE_INTEGER)
      .slice()
      .sort((left, right) => navigationOrder(left.group) - navigationOrder(right.group))
      .map((item) => item.command);

    const expandIndex = ordered.indexOf("deslop.topOffenders.expandAll");
    const collapseIndex = ordered.indexOf("deslop.topOffenders.collapseAll");
    assert.ok(expandIndex >= 0, "Expand All must be a Top Offenders title action");
    assert.ok(collapseIndex >= 0, "Collapse All must be a Top Offenders title action");
    assert.equal(
      collapseIndex,
      expandIndex + 1,
      `Expand All and Collapse All must be adjacent in the title bar, got order: ${ordered.join(", ")}`,
    );
    assert.equal(
      commandTitle(pkg, "deslop.topOffenders.collapseAll"),
      "Deslop: Collapse All Top Offenders",
    );
    assert.equal(
      commandTitle(pkg, "deslop.topOffenders.expandAll"),
      "Deslop: Expand All Top Offenders",
    );
  });

  // [FACET-TESTING] Pin the [VSIX-TOP-OFFENDERS-TOOLBAR] order exactly:
  // grouping/sort/split @1–@3, Choose Filter @4, then Expand All /
  // Collapse All / Refresh adjacent at @5/@6/@7.
  test("Choose Filter sits at navigation@4, ahead of expand/collapse/refresh", () => {
    const pkg = extensionPackage();
    const titleItems = (pkg.contributes.menus["view/title"] ?? []).filter((item) =>
      item.when?.includes("view == deslop.topOffenders"),
    );
    const at = (command: string): number[] =>
      titleItems.filter((item) => item.command === command).map((item) => navigationOrder(item.group));
    assert.deepEqual(at("deslop.topOffenders.chooseFilter"), [4]);
    assert.deepEqual(
      at("deslop.topOffenders.chooseFilterActive"),
      [4],
      "the active-filter icon variant shares slot @4",
    );
    const inactive = titleItems.find((i) => i.command === "deslop.topOffenders.chooseFilter");
    const active = titleItems.find((i) => i.command === "deslop.topOffenders.chooseFilterActive");
    assert.ok(
      inactive?.when?.includes("!deslop.topOffendersFiltered"),
      "plain filter button renders only while the filter is inactive",
    );
    assert.ok(
      active?.when?.includes("deslop.topOffendersFiltered") &&
        !active.when.includes("!deslop.topOffendersFiltered"),
      "filled filter button renders only while the filter is active",
    );
    assert.deepEqual(at("deslop.topOffenders.expandAll"), [5]);
    assert.deepEqual(at("deslop.topOffenders.collapseAll"), [6]);
    assert.deepEqual(at("deslop.refresh"), [7]);
  });

  test("Open HTML Report title action waits until a report is ready", () => {
    const pkg = extensionPackage();
    const titleItems = pkg.contributes.menus["view/title"] ?? [];
    const item = titleItems.find(
      (candidate) => candidate.command === "deslop.openHtmlReport",
    );

    assert.ok(item, "Open HTML Report must be a contributed title action");
    assert.ok(
      item.when?.includes("view == deslop.topOffenders"),
      `Open HTML Report must stay scoped to Top Offenders, got: ${item.when ?? ""}`,
    );
    assert.ok(
      item.when?.includes("deslop.reportReady"),
      `Open HTML Report must stay hidden until a report exists, got: ${item.when ?? ""}`,
    );
  });

  // [FACET-GROUP-BY-TYPE] The grouping toggle cycles all four modes:
  // cluster → file → folder → type → cluster.
  test("grouping cycle includes the type mode", () => {
    const pkg = extensionPackage();
    const titleItems = (pkg.contributes.menus["view/title"] ?? []).filter(
      (item) =>
        item.when?.includes("view == deslop.topOffenders") && item.group === "navigation@1",
    );
    const whenOf = (command: string): string =>
      titleItems.find((item) => item.command === command)?.when ?? "";
    assert.ok(whenOf("deslop.topOffenders.showByFile").includes("== 'cluster'"));
    assert.ok(whenOf("deslop.topOffenders.showByFolder").includes("== 'file'"));
    assert.ok(whenOf("deslop.topOffenders.showByType").includes("== 'folder'"));
    assert.ok(whenOf("deslop.topOffenders.showByCluster").includes("== 'type'"));
    assert.equal(
      commandTitle(pkg, "deslop.topOffenders.showByType"),
      "Deslop: Group Top Offenders by Type",
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
