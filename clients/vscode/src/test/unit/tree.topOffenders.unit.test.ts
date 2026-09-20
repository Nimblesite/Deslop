import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import { tempFile } from "../unit/temp-file.helpers";
import * as vscode from "vscode";
import { openOccurrence } from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { INFORMATIONAL_FINDING, kindTitle, ReportOccurrence } from "../../types/report";
import { cluster, iconColorId, labelText, report, storeWith, tooltipText, topOffenders } from "./tree.helpers";
import { FIXTURE_KIND } from "../cluster.helpers";
import { KIND_COLOR, KIND_ICON, KIND_THEME_COLOR } from "../../design";
import { DEFAULT_OCCURRENCE_END_BYTE, FIXTURE_KIND_TITLE, IDENTICAL_KIND, STRUCTURAL_ONLY_KIND, HIGHEST_CLUSTER_MASS, HIGH_CLUSTER_MASS, MEDIUM_CLUSTER_MASS, MISSING_LABEL, ALPHA_FILE_PATH, BETA_FILE_PATH, FIRST_FIXTURE_PATH, CLUSTER_ROOT_REQUIRED, CANONICAL_OCCURRENCE_CONTEXT, THIRD_ITEM_INDEX, FOURTH_ITEM_INDEX, PAIR_COUNT, ZERO_BASED_FOURTH_LINE, LOW_CLUSTER_WEIGHT, reportOccurrence, withOccurrences } from "./tree.topOffenders.fixtures";

suite("TopOffendersProvider", () => {
  test("cluster mode (default) lists clusters worst-first with global ranks", () => {
    // [VSIX-TOP-OFFENDERS-CLUSTER-MODE] No file-keyed reordering.
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] rank #N lives in the grey description;
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] the stable short id leads the bold label.
    const store = storeWith(
      report([
        cluster("1111aaaabbbbcccc", HIGHEST_CLUSTER_MASS, BETA_FILE_PATH),
        cluster("2222aaaabbbbcccc", HIGH_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("3333aaaabbbbcccc", MEDIUM_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("4444aaaabbbbcccc", 40, "/repo/src/c/Gamma.cs"),
      ]),
    );
    const provider = topOffenders(store);

    const nodes = provider.getChildren();
    const labels = nodes.map(labelText);
    const descriptions = nodes.map((node) => String(node.description ?? ""));

    assert.equal(nodes.length, 4, "one top-level row must render per cluster");
    assert.ok(labels[0]?.startsWith("1111aaa "), `worst row leads with its slug, got: ${labels[0] ?? MISSING_LABEL}`);
    assert.ok(labels[1]?.startsWith("2222aaa "), `row 2 leads with its slug, got: ${labels[1] ?? MISSING_LABEL}`);
    assert.ok(labels[THIRD_ITEM_INDEX]?.startsWith("3333aaa "), `row 3 leads with its slug, got: ${labels[THIRD_ITEM_INDEX] ?? MISSING_LABEL}`);
    assert.ok(labels[FOURTH_ITEM_INDEX]?.startsWith("4444aaa "), `row 4 leads with its slug, got: ${labels[FOURTH_ITEM_INDEX] ?? MISSING_LABEL}`);
    assert.match(labels[0] ?? "", /Beta\.cs/, "row label must show the file");
    assert.match(labels[1] ?? "", /Alpha\.cs/);
    assert.match(labels[THIRD_ITEM_INDEX] ?? "", /Alpha\.cs/);
    assert.match(labels[FOURTH_ITEM_INDEX] ?? "", /Gamma\.cs/);
    assert.match(descriptions[0] ?? "", /\brank\s+#1\b/, "row 1 carries rank #1 in its description");
    assert.match(descriptions[1] ?? "", /\brank\s+#2\b/, "row 2 carries rank #2 in its description");
    assert.match(descriptions[THIRD_ITEM_INDEX] ?? "", /\brank\s+#3\b/, "row 3 carries rank #3 in its description");
    assert.match(descriptions[FOURTH_ITEM_INDEX] ?? "", /\brank\s+#4\b/, "row 4 carries rank #4 in its description");
    assert.ok(
      descriptions.every((d) => /\b\d+ copies\b/.test(d)),
      `cluster descriptions must keep the copy count; got: ${JSON.stringify(descriptions)}`,
    );
    const first = nodes[0];
    assert.ok(first, "first row must exist");
    assert.equal(first.command?.command, "deslop.openCluster");
    assert.deepEqual(
      first.command?.arguments,
      ["1111aaaabbbbcccc"],
      "command argument keeps the full 16-hex id; only the display is shortened",
    );
    assert.equal(provider.getChildren(first).length, PAIR_COUNT);
  });

  test("cluster row label leads with the stable slug, not the volatile #N rank", () => {
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] / [VSIX-TOP-OFFENDERS-CLUSTER-MODE]
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] The stable cluster identifier is the
    // 16-hex hash; the rank #N is a volatile array-index that flips on every
    // snapshot. Putting #N in the bold label makes humans (and AI agents
    // reading the rendered tree) treat the rank as the row's identity.
    // Cluster slug leads, rank moves to the grey description with the
    // literal word "rank". Slug length is shared with the hover bubble
    // (see clusterHover.ts::clusterSlug).
    const store = new ReportStore();
    const clusterId = "1802186da488862f";
    store.setSnapshot(
      report([
        cluster(clusterId, HIGHEST_CLUSTER_MASS, "/repo/src/Worst.cs"),
        cluster("c0ffee1234567890", HIGH_CLUSTER_MASS, "/repo/src/Next.cs"),
      ]),
      0,
    );
    const provider = topOffenders(store);
    const [first, second] = provider.getChildren();
    assert.ok(first, "first cluster row must render");
    assert.ok(second, "second cluster row must render");

    const firstLabel = labelText(first);
    const firstDescription = String(first.description ?? "");
    const firstTooltip = tooltipText(first);
    const firstA11y = first.accessibilityInformation?.label ?? "";

    assert.ok(
      firstLabel.startsWith("1802186 "),
      `label must lead with the stable slug (first 7 hex chars), got: ${firstLabel}`,
    );
    assert.doesNotMatch(
      firstLabel,
      /^#\d/,
      `label must not lead with the volatile #N rank, got: ${firstLabel}`,
    );
    assert.doesNotMatch(
      firstLabel,
      /#1\b/,
      `rank #N must not appear in the bold label at all, got: ${firstLabel}`,
    );
    assert.match(
      firstDescription,
      /\brank\s+#1\b/,
      `description must spell out the word "rank" so AI consumers can't confuse it for an id, got: ${firstDescription}`,
    );
    assert.match(
      firstDescription,
      /\b2 copies\b/,
      `description must keep the copy count, got: ${firstDescription}`,
    );
    assert.match(
      firstTooltip,
      /\brank\s+#1\b/,
      `tooltip must use the word "rank", got: ${firstTooltip}`,
    );
    assert.match(
      firstA11y,
      /\brank\s+#?1\b/,
      `accessibility label must spell out "rank", got: ${firstA11y}`,
    );
    assert.match(
      firstTooltip,
      /cluster id:\s+`1802186da488862f`/,
      "tooltip must still expose the full 16-hex id for AI/cross-reference",
    );

    const secondLabel = labelText(second);
    const secondDescription = String(second.description ?? "");
    assert.ok(
      secondLabel.startsWith("c0ffee1 "),
      `second row's label must also lead with its own slug, got: ${secondLabel}`,
    );
    assert.match(
      secondDescription,
      /\brank\s+#2\b/,
      `second row's description must carry "rank #2", got: ${secondDescription}`,
    );

    assert.equal(
      first.command?.command,
      "deslop.openCluster",
      "row still navigates to the cluster",
    );
    assert.deepEqual(
      first.command?.arguments,
      [clusterId],
      "command argument keeps the full 16-hex id; display truncation is presentation-only",
    );
  });

  test("issue_47_cluster_tooltip_keeps_labeled_cluster_id_after_human_description", () => {
    const store = new ReportStore();
    const clusterId = "1802186da488862f";
    store.setSnapshot(
      report([cluster(clusterId, 48_936.95, "/repo/src/ICD10/CliE2ETests.cs")]),
      0,
    );
    const provider = topOffenders(store);
    const [node] = provider.getChildren();
    assert.ok(node, "cluster row must render");
    assert.notEqual(
      String(node.description ?? ""),
      clusterId,
      "row description must not use the hex cluster id as the human anchor",
    );
    assert.match(
      tooltipText(node),
      /cluster id:\s+`1802186da488862f`/,
      "tooltip must keep the machine id discoverable behind a labeled cluster id field",
    );
  });

  test("renders the clone kind as title, icon and colour on Top Offenders rows", () => {
    // [CLONE-KIND-COLOR] The clone kind drives the icon and its colour; the
    // title names it. Rank chooses neither.
    const store = storeWith(
      report([
        cluster("exact", HIGHEST_CLUSTER_MASS, "/repo/src/a/Exact.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "error", 1, IDENTICAL_KIND),
        cluster("near", 90, "/repo/src/b/Near.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "warning", 2),
      ]),
    );
    const provider = topOffenders(store);

    const [exact, near] = provider.getChildren();
    assert.ok(exact, "identical row must render");
    assert.ok(near, "nearly identical row must render");
    assert.ok(exact.iconPath instanceof vscode.ThemeIcon);
    assert.ok(near.iconPath instanceof vscode.ThemeIcon);
    assert.equal(iconColorId(exact), KIND_THEME_COLOR[IDENTICAL_KIND]);
    assert.equal(iconColorId(near), KIND_THEME_COLOR[FIXTURE_KIND]);
    assert.equal(exact.iconPath.id, KIND_ICON[IDENTICAL_KIND]);
    assert.equal(near.iconPath.id, KIND_ICON[FIXTURE_KIND]);
    assert.match(labelText(exact), new RegExp(kindTitle(IDENTICAL_KIND)));
    assert.match(labelText(near), new RegExp(FIXTURE_KIND_TITLE));
    assert.match(labelText(exact), /Exact\.cs/);
    assert.match(labelText(near), /Near\.cs/);
    assert.match(exact.accessibilityInformation?.label ?? "", new RegExp(kindTitle(IDENTICAL_KIND)));
    assert.match(near.accessibilityInformation?.label ?? "", new RegExp(FIXTURE_KIND_TITLE));
    assert.match(exact.accessibilityInformation?.label ?? "", /Exact\.cs/);
    assert.match(near.accessibilityInformation?.label ?? "", /Near\.cs/);
    assert.match(tooltipText(exact), /\/repo\/src\/a\/Exact\.cs/);
    assert.match(tooltipText(near), /\/repo\/src\/b\/Near\.cs/);
    assert.match(tooltipText(exact), /Type-1 exact clone/, "the tooltip names the taxonomy");
  });

  test("shape-only findings stay informational regardless of supplied rank", () => {
    // [CLONE-KIND-COLOR] The predicted failure of colouring by rank: a
    // family that only shares shape ranks first by mass, and a genuinely
    // identical cluster sits below it. Colour must follow the evidence.
    const store = storeWith(
      report([
        cluster("shape-giant", HIGHEST_CLUSTER_MASS, "/repo/src/Shape.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "error", 1, STRUCTURAL_ONLY_KIND),
        cluster("proven", LOW_CLUSTER_WEIGHT, "/repo/src/Proven.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "hint", 2, IDENTICAL_KIND),
      ]),
    );
    const provider = topOffenders(store);
    const [shapeGiant, proven] = provider.getChildren();
    assert.ok(shapeGiant && proven, "both rows must render");
    assert.equal(
      iconColorId(shapeGiant),
      KIND_THEME_COLOR[STRUCTURAL_ONLY_KIND],
      "the rank-1 shape-only family wears the muted structural-only colour",
    );
    assert.equal(
      iconColorId(proven),
      KIND_THEME_COLOR[IDENTICAL_KIND],
      "the byte-identical cluster is crimson wherever it ranks",
    );
    assert.notEqual(iconColorId(shapeGiant), iconColorId(proven));
    assert.match(labelText(shapeGiant), new RegExp(kindTitle(STRUCTURAL_ONLY_KIND)));
    assert.match(labelText(proven), new RegExp(kindTitle(IDENTICAL_KIND)));
    assert.equal(shapeGiant.description, INFORMATIONAL_FINDING);
    assert.equal(String(shapeGiant.description).includes("rank #1"), false, "shape-only findings claim no duplication rank");
  });

  test("every clone kind paints a distinct icon and a distinct colour, and none is green", () => {
    // [CLONE-KIND-COLOR] Five kinds, five icons, five colours — a kind that
    // shared either would be indistinguishable on screen. Green implies the
    // code is in good shape; duplicates never are.
    const icons = Object.values(KIND_ICON);
    const themeIds = Object.values(KIND_THEME_COLOR);
    const hexes = Object.values(KIND_COLOR);
    assert.equal(new Set(icons).size, icons.length, "icons must be distinct");
    assert.equal(new Set(themeIds).size, themeIds.length, "theme colour ids must be distinct");
    assert.equal(new Set(hexes).size, hexes.length, "hex colours must be distinct");
    for (const [kind, id] of Object.entries(KIND_THEME_COLOR)) {
      assert.doesNotMatch(id, /green/i, `${kind} must not paint green`);
    }
  });

  test("expanding a cluster node yields OccurrenceNode children", () => {
    const store = new ReportStore();
    const c = cluster("a", LOW_CLUSTER_WEIGHT, "/f1");
    store.setSnapshot(report([c]), 0);
    const provider = topOffenders(store);
    const roots = provider.getChildren();
    const kids = provider.getChildren(roots[0]);
    assert.equal(kids.length, c.occurrences.length);
  });

  test("occurrence node tooltip shows parent cluster rank, kind, and position (#47)", () => {
    const store = storeWith(report([cluster("a", LOW_CLUSTER_WEIGHT, FIRST_FIXTURE_PATH)]));
    const provider = topOffenders(store);
    const [root] = provider.getChildren();
    assert.ok(root, CLUSTER_ROOT_REQUIRED);
    const [first, second] = provider.getChildren(root);
    assert.ok(first, "first occurrence node must exist");
    assert.ok(second, "second occurrence node must exist");
    const tip1 = tooltipText(first);
    const tip2 = tooltipText(second);
    assert.match(tip1, /\brank\s+#1\b/, "tooltip must spell out the parent cluster rank");
    assert.match(tip1, new RegExp(FIXTURE_KIND_TITLE), "tooltip must name the parent's clone kind");
    assert.match(tip1, /occurrence 1 of 2/, "tooltip must show position in cluster");
    assert.match(tip2, /occurrence 2 of 2/, "second occurrence tooltip must reflect its index");
  });

  test("compare with canonical context values hide non-actionable rows (#14)", () => {
    const store = new ReportStore();
    const singleOccurrence = withOccurrences(cluster("single", 5, "/single"), [
      reportOccurrence("/single"),
    ]);
    store.setSnapshot(
      report([cluster("multi", LOW_CLUSTER_WEIGHT, FIRST_FIXTURE_PATH), singleOccurrence]),
      0,
    );
    const provider = topOffenders(store);
    const [multi, single] = provider.getChildren();
    assert.ok(multi, "multi-occurrence cluster root must exist");
    assert.ok(single, "single-occurrence cluster root must exist");
    assert.equal(multi.contextValue, "deslop.clusterComparable");
    assert.equal(single.contextValue, "deslop.clusterSingle");

    const [canonical, comparable] = provider.getChildren(multi);
    assert.ok(canonical, "canonical occurrence row must exist");
    assert.ok(comparable, "comparable occurrence row must exist");
    assert.equal(canonical.contextValue, CANONICAL_OCCURRENCE_CONTEXT);
    assert.equal(comparable.contextValue, "deslop.occurrence");
  });

  test("occurrence row reports and opens the exact file, line, and column", async () => {
    // [VSIX-ACTIVITY-BAR] Issue #8: tree occurrence rows must show
    // path:line:column, not machine-oriented start_byte..end_byte.
    const { dir, file: occurrencePath } = tempFile("deslop-issue-8-tree-", "ChatProtocol.cs");
    const source = "namespace Demo;\n\npublic sealed class ChatProtocol {\n    void Send() {}\n}\n";
    const startByte = Buffer.byteLength(source.slice(0, source.indexOf("void Send")), "utf8");
    const endByte = startByte + Buffer.byteLength("void Send", "utf8");
    fs.writeFileSync(occurrencePath, source, "utf8");

    try {
      const store = storeWith(report([cluster("issue-8", LOW_CLUSTER_WEIGHT, occurrencePath, startByte, endByte)]));
      const provider = topOffenders(store);
      const [root] = provider.getChildren();
      assert.ok(root, CLUSTER_ROOT_REQUIRED);

      const [occurrence] = provider.getChildren(root);
      assert.ok(occurrence, "occurrence child must exist");
      const label = typeof occurrence.label === "string"
        ? occurrence.label
        : occurrence.label?.label ?? "";
      const description = String(occurrence.description ?? "");
      const rendered = `${label} ${description}`;

      assert.ok(occurrence.command, "occurrence row must be tappable");
      assert.equal(occurrence.command.command, "deslop.openOccurrence");
      const commandArguments = occurrence.command.arguments;
      assert.ok(commandArguments, "occurrence command must carry arguments");
      const argument = commandArguments[0] as ReportOccurrence | undefined;
      assert.ok(argument, "occurrence command must carry the occurrence payload");

      await openOccurrence(argument);

      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "tapping the occurrence must open an editor");
      assert.equal(editor.document.uri.fsPath, occurrencePath);
      assert.equal(editor.selection.start.line, ZERO_BASED_FOURTH_LINE, "cursor should move to line 4");
      assert.equal(editor.selection.start.character, 4, "cursor should move to column 5");
      assert.equal(editor.selection.end.character, 13, "selection should cover the occurrence");

      assert.deepEqual(
        {
          hasFileName: /ChatProtocol\.cs/.test(rendered),
          hasLineAndColumn: /ChatProtocol\.cs:4:5/.test(rendered) ||
            /line\s+4,\s*column\s+5/i.test(rendered),
          exposesRawByteRange: new RegExp(`\\b${startByte}\\.\\.${endByte}\\b`).test(rendered),
          usesByteTerminology: /\bbytes?\b/i.test(rendered),
        },
        {
          hasFileName: true,
          hasLineAndColumn: true,
          exposesRawByteRange: false,
          usesByteTerminology: false,
        },
        `occurrence row must report the same human target it navigates to, got: ${rendered}`,
      );
    } finally {
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });
});
