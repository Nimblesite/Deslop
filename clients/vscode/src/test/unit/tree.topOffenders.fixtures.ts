// Shared fixtures for the Top Offenders tests.
import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { TopOffendersProvider } from "../../tree/providers";
import { kindTitle, ReportCluster, ReportOccurrence } from "../../types/report";
import { withGroupBy } from "./tree.helpers";
import { FIXTURE_KIND } from "../cluster.helpers";

export const DEFAULT_OCCURRENCE_END_BYTE = 20;

export const FIXTURE_KIND_TITLE = kindTitle(FIXTURE_KIND);

export const IDENTICAL_KIND = "identical";

export const STRUCTURAL_ONLY_KIND = "structural_only";

export const HIGHEST_CLUSTER_MASS = 100;

export const HIGH_CLUSTER_MASS = 80;

export const MEDIUM_CLUSTER_MASS = 60;

export const TIED_CLUSTER_MASS = 50;

export const FILE_GROUPING_MODE = "file";

export const MISSING_LABEL = "<missing>";

export const MIXED_FILE_PATH = "/repo/Mixed.cs";

export const REPO_A_PATH = "/repo/A.cs";

export const REPO_B_PATH = "/repo/B.cs";

export const ALPHA_FILE_PATH = "/repo/src/a/Alpha.cs";

export const SORT_BY_SETTING = "topOffenders.sortBy";

export const PATH_SORT_MODE = "path";

export const ANALYSING_PHASE = "analysing";

export const FIRST_CLUSTER_ID = "c1";

export const SECOND_CLUSTER_ID = "c2";

export const BETA_FILE_PATH = "/repo/src/b/Beta.cs";

export const FIRST_FIXTURE_PATH = "/f1";

export const CLUSTER_ROOT_REQUIRED = "cluster root must exist";

export const CANONICAL_OCCURRENCE_CONTEXT = "deslop.occurrenceCanonical";

export const HEAVY_CLUSTER_ID = "heavy";

export const WHOLE_NUMBER_MASS = 527;

export const TWO_DECIMAL_MASS = "527.00";

export const FOLDER_GROUPING_MODE = "folder";

export const HEAVY_FILE_PATH = "/repo/z.cs";

export const LIGHT_CLUSTER_ID = "light";

export const LIGHT_FILE_PATH = "/repo/a.cs";

export const TEST_TWO = 2;

export const TEST_THREE = 3;

export const THIRD_ITEM_INDEX = TEST_TWO;

export const FOURTH_ITEM_INDEX = TEST_THREE;

export const PAIR_COUNT = TEST_TWO;

export const FILE_ROOT_COUNT = TEST_THREE;

export const ZERO_BASED_FOURTH_LINE = TEST_THREE;

export const SECOND_GENERATION = TEST_TWO;

export const CACHE_HIT_COUNT = TEST_TWO;

export const EXPECTED_TREE_REFRESH_COUNT = TEST_TWO;

export const MIN_VISIBLE_NODE_COUNT = TEST_TWO;

export const TEST_TEN = 10;

export const LOW_CLUSTER_WEIGHT = TEST_TEN;

export const DIRTY_OCCURRENCE_START_BYTE = TEST_TEN;

export const DIRTY_FILE_PATH = "/repo/Dirty.cs";

export function reportOccurrence(
  occurrencePath: string,
  startByte = 0,
  endByte = DEFAULT_OCCURRENCE_END_BYTE,
): ReportOccurrence {
  return { path: occurrencePath, start_byte: startByte, end_byte: endByte, start_line: 1, end_line: 2, hidden: false };
}

export function withOccurrences(
  base: ReportCluster,
  occurrences: ReportOccurrence[],
): ReportCluster {
  return {
    ...base,
    canonical_node_count: occurrences.length,
    occurrences_total: occurrences.length,
    occurrence_count: occurrences.length,
    occurrences,
  };
}

export async function firstRootIn(
  provider: TopOffendersProvider,
  mode: "file" | "folder",
): Promise<vscode.TreeItem> {
  const roots: vscode.TreeItem[] = [];
  await withGroupBy(mode, () => {
    roots.push(...provider.getChildren());
  });
  const [root] = roots;
  assert.ok(root, `${mode} mode must expose a root row`);
  return root;
}
