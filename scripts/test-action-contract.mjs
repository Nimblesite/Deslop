// Proof suite for the GitHub Marketplace action. [ACTION-TESTS]
//
// Covers the pieces a hosted runner cannot cheaply prove on every PR: the
// runner -> release-asset mapping, version derivation from `github.action_ref`,
// checksum rejection, report-output extraction, and the static shape of
// action.yml. The runner-side behaviour (download, extract, gate) is proven
// end-to-end by .github/workflows/action-selftest.yml.
//
// The checks live in two side-effect modules sharing one counter, so this
// entry point stays a table of contents and the printed total counts every
// module — a module that silently stops importing would zero the total.

import "./action-contract-scripts-checks.mjs";
import "./action-contract-shape-checks.mjs";

import assert from "node:assert/strict";

import { total } from "./action-contract-harness.mjs";

assert.ok(total() >= 39, `the suite ran ${total()} checks — a module stopped importing`);
console.log(`\naction contract: ${total()} checks passed`);
