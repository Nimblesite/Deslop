// Test observer for `ReportStore` mutations, built on the same `effect()`
// every production subscriber uses.
import { effect } from "@preact/signals-core";
import * as vscode from "vscode";

import { ReportState, ReportStore } from "../reportStore";

/** Calls `observe` with the store state after each mutation that follows
 * the subscription. The effect's own first run only registers the
 * dependency, so a test counts mutations, not the subscription. A store
 * owned by the activated extension is observed with that extension's
 * `effect` (`ExtensionApi.effect`): signals track only within the runtime
 * that created them. */
export function observeStoreChanges(
  store: ReportStore,
  observe: (state: ReportState) => void,
  run: typeof effect = effect,
): vscode.Disposable {
  let subscribed = false;
  const dispose = run(() => {
    const state = store.current;
    if (subscribed) observe(state);
    subscribed = true;
  });
  return { dispose };
}
