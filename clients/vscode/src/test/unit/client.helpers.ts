// A recording stand-in for the LSP client. Every suite that asserts
// "which RPCs did the extension dispatch, with what params" needs the
// same stub, so it lives here once rather than being retyped per test.

import type { LanguageClient } from "vscode-languageclient/node";

/** One `sendRequest` the recording client observed, in dispatch order. */
export interface RecordedCall {
  method: string;
  params: unknown;
}

/** What a recorded `sendRequest` resolves to. */
export type ClientReply = (method: string, params: unknown) => unknown;

/** A recording `LanguageClient`: each `sendRequest` is appended to `calls`
 * and resolves to whatever `reply` returns for that method. The default
 * reply resolves `undefined`, matching a fire-and-forget notification. */
export function recordingClient(reply: ClientReply = () => undefined): {
  calls: RecordedCall[];
  client: LanguageClient;
} {
  const calls: RecordedCall[] = [];
  const client = {
    sendRequest: (method: string, params: unknown) => {
      calls.push({ method, params });
      return Promise.resolve(reply(method, params));
    },
  } as unknown as LanguageClient;
  return { calls, client };
}
