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

/** A client that answers every `sendRequest` through `reply` and records
 * nothing — for suites asserting the effect of a response, not the traffic. */
export function respondingClient(reply: ClientReply): LanguageClient {
  return recordingClient(reply).client;
}

/** A client whose every `sendRequest` rejects with `message`, so a suite can
 * drive the backend-unavailable branch. */
export function rejectingClient(message: string): LanguageClient {
  return {
    sendRequest: () => Promise.reject(new Error(message)),
  } as unknown as LanguageClient;
}

/** A client whose `sendRequest` throws synchronously rather than returning a
 * rejected promise — the shape a suite uses to prove a call never happens.
 * Kept distinct from `rejectingClient`: a synchronous throw escapes before any
 * `.catch` on the returned promise can see it. */
export function throwingClient(message: string): LanguageClient {
  return {
    sendRequest: () => {
      throw new Error(message);
    },
  } as unknown as LanguageClient;
}

/** A client that captures each `onNotification` handler by name so a suite can
 * drive a server push through `notify`, while `sendRequest` is recorded and
 * answered by `reply`. Replaces the per-suite handler-capture closures. */
export function notifyingClient(reply: ClientReply = () => null): {
  calls: RecordedCall[];
  client: LanguageClient;
  notify: (name: string, payload?: unknown) => void;
  registered: (name: string) => boolean;
} {
  const calls: RecordedCall[] = [];
  const handlers = new Map<string, (payload: unknown) => void>();
  const client = {
    onNotification: (name: string, handler: (payload: unknown) => void) => {
      handlers.set(name, handler);
    },
    sendRequest: (method: string, params: unknown) => {
      calls.push({ method, params });
      return Promise.resolve(reply(method, params));
    },
  } as unknown as LanguageClient;
  return {
    calls,
    client,
    notify: (name, payload) => handlers.get(name)?.(payload),
    registered: (name) => handlers.has(name),
  };
}
