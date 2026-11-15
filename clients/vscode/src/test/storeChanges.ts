// Test observer for `ReportStore` mutations, built on the same `effect()`
// every production subscriber uses.
import { effect } from "@preact/signals-core";
import * as vscode from "vscode";

import { ReportState, ReportStore } from "../reportStore";

/** Calls `observe` with the store state after each mutation that follows
 * the subscription. The effect's own first run only registers the
 * dependency, so a test counts mutations, not the subscription. */
export function observeStoreChanges(store: ReportStore, observe: (state: ReportState) => void): vscode.Disposable {
  let subscribed = false;
  const dispose = effect(() => {
    const state = store.current;
    if (subscribed) observe(state);
    subscribed = true;
  });
  return { dispose };
}
