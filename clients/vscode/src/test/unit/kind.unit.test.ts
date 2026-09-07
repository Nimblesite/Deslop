// Unit: clone kind parity ([CLONE-KIND-LABELS], [CLONE-KIND-COLOR]). The
// engine owns the kind registry and folds every cluster's kind
// ([CLONE-KIND-FOLD]); the extension carries the titles and the paint so
// it can label and colour a cluster without a round trip. A drifted copy
// shows one word in the tree and another in the CLI, or paints one kind
// two colours across the editor and the HTML report. This suite holds
// every copy to what the bundled CLI renders: the wire labels its JSON and
// text reports carry, the titles and taxonomy names its HTML prints, the
// `--kind-*` colours its stylesheet declares, and the contributed
// `deslop.kind.*` theme colours the tree paints icons with.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import { scanWithBundledCli, stagedFixturePath, type ScannedFixture } from "../cli.helpers";
import { KIND_COLOR, KIND_THEME_COLOR } from "../../design";
import { CLUSTER_KINDS, kindTaxonomy, kindTitle, type ClusterKind } from "../../types/report";
import { extensionPackage } from "./package.helpers";

/** The class suffix the HTML report keys each kind's colour on —
 * `ClusterKind::labels().css_suffix` in the engine. */
const KIND_CSS_SUFFIX: Record<ClusterKind, string> = {
  identical: "identical",
  nearly_identical: "nearly-identical",
  same_behavior: "same-behavior",
  structural_only: "structural-only",
  loosely_similar: "loosely-similar",
};

/** The kinds the parity corpus must fold, so no HTML assertion can pass on
 * an empty report: the staged C# fixture is a renamed pair, and the Rust
 * pair below is byte-identical. */
const IDENTICAL_KIND: ClusterKind = "identical";
const NEARLY_IDENTICAL_KIND: ClusterKind = "nearly_identical";
const CORPUS_KINDS: readonly ClusterKind[] = [IDENTICAL_KIND, NEARLY_IDENTICAL_KIND];

/** Two byte-identical copies of one Rust function. Shaped around a `for`
 * loop over a slice so it can never join the C# pair's cluster. */
const IDENTICAL_RUST_FN = [
  "pub fn checksum(values: &[i64]) -> i64 {",
  "    let mut hash = 7;",
  "    for value in values {",
  "        hash = hash * 31 + value;",
  "        if hash > 1000000 {",
  "            hash %= 1000003;",
  "        }",
  "    }",
  "    hash",
  "}",
  "",
].join("\n");
const IDENTICAL_RUST_FILES = ["exact_a.rs", "exact_b.rs"] as const;
/** Low enough for the small Rust pair to clear the node floor. */
const MIN_NODES_ARGS = ["--min-nodes", "8"] as const;
const TEMP_DIR_PREFIX = "deslop-kind-parity-";
const THEME_COLOR_PREFIX = "deslop.kind.";
const CLI_TEXT_KIND_KEY = " kind=";
const RETIRED_NEUTRAL_TITLE = "Duplicate code";

/** The staged fixture plus the byte-identical Rust pair, in a scratch root. */
function parityCorpus(): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), TEMP_DIR_PREFIX));
  fs.cpSync(stagedFixturePath(), root, { recursive: true });
  for (const file of IDENTICAL_RUST_FILES) {
    fs.writeFileSync(path.join(root, file), IDENTICAL_RUST_FN);
  }
  return root;
}

/** The card title the HTML report prints for a kind. */
function cardTitle(kind: ClusterKind): string {
  return `<h3 class="cluster-card__title" title="${kindTaxonomy(kind)}">${kindTitle(kind)}</h3>`;
}

/** The opening of the kind's collapsible group in the HTML report. */
function groupSummary(kind: ClusterKind): string {
  return (
    `<details class="clone-group clone-group--${KIND_CSS_SUFFIX[kind]}" open>` +
    `<summary title="${kindTaxonomy(kind)}">${kindTitle(kind)} — `
  );
}

/** The CSS variable the HTML report declares for a kind's colour. */
function cssColour(kind: ClusterKind): string {
  return `--kind-${KIND_CSS_SUFFIX[kind]}:${KIND_COLOR[kind]}`;
}

/** The text-report row for one cluster. */
function textRow(text: string, clusterId: string): string {
  const row = text.split("\n").find((line) => line.includes(`[${clusterId}]`));
  assert.ok(row, `the text report must print a row for ${clusterId}:\n${text}`);
  return row;
}

suite("clone kind parity with the engine", () => {
  let scanned: ScannedFixture;

  suiteSetup(() => {
    scanned = scanWithBundledCli(parityCorpus(), MIN_NODES_ARGS);
  });

  test("the corpus folds a byte-identical and a renamed cluster, on wire labels the extension knows", () => {
    const kinds = new Set(scanned.clusters.map((cluster) => cluster.kind));
    for (const kind of CORPUS_KINDS) {
      assert.ok(kinds.has(kind), `the corpus must fold a ${kind} cluster: ${JSON.stringify(scanned.clusters)}`);
    }
    for (const kind of kinds) {
      assert.ok(CLUSTER_KINDS.includes(kind), `the engine folded a kind the extension cannot title: ${kind}`);
    }
  });

  test("the text report prints every cluster's wire label", () => {
    for (const cluster of scanned.clusters) {
      const row = textRow(scanned.text, cluster.id);
      assert.ok(
        row.endsWith(`${CLI_TEXT_KIND_KEY}${cluster.kind}`),
        `the row must close with the cluster's kind: ${row}`,
      );
    }
  });

  test("the HTML report titles and groups every cluster by the extension's words", () => {
    for (const cluster of scanned.clusters) {
      assert.ok(scanned.html.includes(cardTitle(cluster.kind)), `card title for ${cluster.kind} must render`);
      assert.ok(scanned.html.includes(groupSummary(cluster.kind)), `group for ${cluster.kind} must render`);
    }
    assert.ok(!scanned.html.includes(RETIRED_NEUTRAL_TITLE), "the neutral title is retired");
    assert.ok(!scanned.text.includes(RETIRED_NEUTRAL_TITLE), "the neutral title is retired");
  });

  test("the HTML report declares every kind colour the extension paints with", () => {
    for (const kind of CLUSTER_KINDS) {
      assert.ok(scanned.html.includes(cssColour(kind)), `${kind} must be painted ${KIND_COLOR[kind]} in the report`);
    }
  });

  test("package.json contributes every kind colour with the paint table's value", () => {
    const contributed = extensionPackage().contributes.colors.filter((colour) =>
      colour.id.startsWith(THEME_COLOR_PREFIX),
    );
    assert.equal(contributed.length, CLUSTER_KINDS.length, "one theme colour per kind, no strays");
    for (const kind of CLUSTER_KINDS) {
      const colour = contributed.find((candidate) => candidate.id === KIND_THEME_COLOR[kind]);
      assert.ok(colour, `${kind} must be contributed as ${KIND_THEME_COLOR[kind]}`);
      const defaults = Object.values(colour.defaults);
      assert.ok(defaults.length > 0, `${colour.id} must declare theme defaults`);
      for (const value of defaults) {
        assert.equal(value, KIND_COLOR[kind], `${colour.id} must default to the paint table's ${KIND_COLOR[kind]}`);
      }
      assert.ok(colour.description.startsWith(kindTitle(kind)), `${colour.id} must be described by its title`);
    }
  });
});
