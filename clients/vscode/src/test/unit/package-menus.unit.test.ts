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

// [VSIX-PAIR-COMPARE] The retired implicit-compare commands. A single tree
// row or hover can never name both endpoints of a pair, so these must stay
// gone from every contribution surface.
const IMPLICIT_COMPARE_COMMANDS = [
  "deslop.compareWithCanonical",
  "deslop.compareOccurrenceWithCanonical",
] as const;

const COMPARE_PAIR_COMMAND = "deslop.comparePair";

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

  test("occurrence context menu exposes no implicit compare, keeps open actions", () => {
    const pkg = extensionPackage();
    const contextItems = pkg.contributes.menus["view/item/context"];
    assert.ok(contextItems, "view/item/context menu must be contributed");
    const occurrenceItems = contextItems.filter(
      (item) => item.when === "viewItem == deslop.occurrence",
    );

    // [VSIX-PAIR-COMPARE] A tree row can only name one occurrence, so no
    // menu on an occurrence row may start a comparison — pair evidence
    // needs two endpoints the user picks in the cluster webview.
    for (const outlawed of IMPLICIT_COMPARE_COMMANDS) {
      assert.ok(
        !occurrenceItems.some((item) => item.command === outlawed),
        `${outlawed} must not appear on occurrence rows — implicit pair compare is retired`,
      );
    }
    assert.ok(!occurrenceItems.some((item) => item.command === "deslop.openOccurrence"));
  });

  test("compare exists only as explicit two-endpoint comparePair (#14)", () => {
    const pkg = extensionPackage();

    // The retired implicit-compare commands must be gone from every
    // contribution surface: commands, palette, and view/item/context.
    const serialized = JSON.stringify(pkg);
    for (const outlawed of IMPLICIT_COMPARE_COMMANDS) {
      assert.ok(
        !serialized.includes(outlawed),
        `${outlawed} must not appear anywhere in package.json`,
      );
    }

    assert.equal(
      commandTitle(pkg, COMPARE_PAIR_COMMAND),
      "Deslop: Compare Selected Occurrences",
    );
    const palette = pkg.contributes.menus.commandPalette ?? [];
    assert.ok(
      palette.some((item) => item.command === COMPARE_PAIR_COMMAND && item.when === "false"),
      "comparePair is reachable only through the webview's two-slot selection, not the palette",
    );
    const contextItems = pkg.contributes.menus["view/item/context"];
    assert.ok(
      !contextItems?.some((item) => item.command === COMPARE_PAIR_COMMAND),
      "comparePair must not hang off tree rows — the two endpoints are picked in the webview",
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

  // [FACET-GROUP-BY-SEVERITY] The grouping toggle cycles all four modes:
  // cluster → file → folder → severity → cluster.
  test("grouping cycle includes the severity mode", () => {
    const pkg = extensionPackage();
    const titleItems = (pkg.contributes.menus["view/title"] ?? []).filter(
      (item) =>
        item.when?.includes("view == deslop.topOffenders") && item.group === "navigation@1",
    );
    const whenOf = (command: string): string =>
      titleItems.find((item) => item.command === command)?.when ?? "";
    assert.ok(whenOf("deslop.topOffenders.showByFile").includes("== 'cluster'"));
    assert.ok(whenOf("deslop.topOffenders.showByFolder").includes("== 'file'"));
    assert.ok(whenOf("deslop.topOffenders.showBySeverity").includes("== 'folder'"));
    assert.ok(whenOf("deslop.topOffenders.showByCluster").includes("== 'severity'"));
    assert.equal(
      commandTitle(pkg, "deslop.topOffenders.showBySeverity"),
      "Deslop: Group Top Offenders by Severity",
    );
    // The clone-type axis is retired: no type-mode toggle may exist.
    assert.equal(
      commandTitle(pkg, "deslop.topOffenders.showByType"),
      undefined,
      "type-mode grouping was removed with the bucket axes",
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
