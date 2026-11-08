// [VSIX-LANGUAGE-ICONS] Native tree surfaces consume the shared artwork.
import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { FileNode, FileMetricNode, OccurrenceNode } from "../../tree/nodes";
import { buildLanguageMode } from "../../tree/grouping";
import { languageIconUri } from "../../types/languageIcons";
import { cluster } from "./tree.helpers";

const SOURCE_PATH = "/repo/main.osp";
const PYTHON_PATH = "/repo/model.py";
const PYTHON_FLAVOR = "osprey-python";
const LANGUAGE = "osprey";
const CLUSTER_ID = "osprey-icon";
const MASS = 42;
const ANALYSED_LOC = 20;
const DUPLICATED_LOC = 10;
const PERCENT = 50;
const FIRST = 0;

suite("native Osprey artwork", () => {
  test("engine-selected Python flavor keeps Osprey artwork in every native view", () => {
    const finding = cluster(CLUSTER_ID, MASS, PYTHON_PATH);
    const occurrence = finding.occurrences[FIRST];
    assert.ok(occurrence);
    occurrence.language = PYTHON_FLAVOR;
    const nodes = [new FileNode(PYTHON_PATH, [finding], MASS), new OccurrenceNode(occurrence),
      buildLanguageMode([finding])[FIRST], new FileMetricNode({ path: PYTHON_PATH,
        language: PYTHON_FLAVOR, analysed_loc: ANALYSED_LOC, duplicated_loc: DUPLICATED_LOC,
        duplication_percent: PERCENT })];
    for (const node of nodes) {
      assert.ok(node?.iconPath instanceof vscode.Uri);
      assert.equal(node.iconPath.toString(), vscode.Uri.parse(languageIconUri(LANGUAGE)).toString());
    }
  });
  test("language groups, files, occurrences and metrics share the official Osprey icon", () => {
    const finding = cluster(CLUSTER_ID, MASS, SOURCE_PATH);
    const occurrence = finding.occurrences[FIRST];
    assert.ok(occurrence);
    const nodes = [new FileNode(SOURCE_PATH, [finding], MASS),
      new OccurrenceNode(occurrence), buildLanguageMode([finding])[FIRST],
      new FileMetricNode({ path: SOURCE_PATH, analysed_loc: ANALYSED_LOC,
        duplicated_loc: DUPLICATED_LOC, duplication_percent: PERCENT })];
    for (const node of nodes) {
      assert.ok(node);
      assert.ok(node.iconPath instanceof vscode.Uri);
      assert.equal(node.iconPath.toString(), vscode.Uri.parse(languageIconUri(LANGUAGE)).toString());
    }
  });
});
