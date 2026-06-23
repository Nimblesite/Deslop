// Unit: thresholdStatus — the shared duplication-gate formatter rendered
// by both the sidebar Duplication tree and the report webview, so the two
// panels stay identical ([VSIX-METRICS-PANEL]).

import * as assert from "node:assert/strict";
import { thresholdStatus } from "../../tree/threshold";

suite("thresholdStatus", () => {
  test("renders nothing when no gate is configured", () => {
    const status = thresholdStatus({ percent: 0, breached: false, source: "none" });
    assert.equal(status.configured, false);
    assert.equal(status.breached, false);
    assert.equal(status.label, "");
  });

  test("flags an over-budget gate with a warning", () => {
    const status = thresholdStatus({ percent: 20, breached: true, source: "config" });
    assert.equal(status.configured, true);
    assert.equal(status.breached, true);
    assert.equal(status.label, "⚠ over 20.0% gate");
  });

  test("shows a within-budget gate with a checkmark", () => {
    const status = thresholdStatus({ percent: 20, breached: false, source: "cli" });
    assert.equal(status.configured, true);
    assert.equal(status.breached, false);
    assert.equal(status.label, "✓ within 20.0% gate");
  });
});
