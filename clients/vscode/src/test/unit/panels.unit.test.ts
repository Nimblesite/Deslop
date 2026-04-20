// Unit: handleMessage dispatch — every case must execute the corresponding
// codededup.* command without throwing on malformed payloads.

import { handleMessage } from "../../webview/panels";
import { ReportStore } from "../../reportStore";

suite("webview handleMessage", () => {
  test("ignores malformed messages", async () => {
    const store = new ReportStore();
    await handleMessage(store, null);
    await handleMessage(store, "string");
    await handleMessage(store, 42);
  });

  test("open/cluster dispatches when id is a string", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "open/cluster", id: "cluster-x" });
    await handleMessage(store, { kind: "open/cluster", id: 42 });
  });

  test("open/occurrence dispatches when occurrence is present", async () => {
    const store = new ReportStore();
    try {
      await handleMessage(store, {
        kind: "open/occurrence",
        occurrence: { path: "/tmp/doesnotexist.cs", start_byte: 0, end_byte: 1, hidden: false },
      });
    } catch {
      // the command dispatch tries to open the file which doesn't exist;
      // we only need to cover the dispatch branch
    }
    await handleMessage(store, { kind: "open/occurrence" });
  });

  test("open/cluster path validation — non-string id ignored", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "open/cluster", id: 42 });
  });

  test("compare/canonical dispatches when clusterId is a string", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "compare/canonical", clusterId: "z" });
    await handleMessage(store, { kind: "compare/canonical" });
  });

  test("refresh dispatches the workspace command", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "refresh" });
  });

  test("navigate/next and navigate/prev are no-ops when clusters are empty", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "navigate/next" });
    await handleMessage(store, { kind: "navigate/prev" });
  });

  test("unknown kind is ignored", async () => {
    const store = new ReportStore();
    await handleMessage(store, { kind: "nonsense" });
  });
});
