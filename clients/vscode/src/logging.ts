// Structured logging via pino, fanned out to the Deslop output channel.
// One logger instance, one output channel, no println-style debugging allowed.

import * as vscode from "vscode";
import pino, { Logger, LogDescriptor } from "pino";

let channel: vscode.LogOutputChannel | undefined;
let rootLogger: Logger | undefined;

// A LogOutputChannel (created with `{ log: true }`) is what the
// LanguageClient's `outputChannel` option now requires; it is a superset
// of OutputChannel, so the raw `append`/`show` writes below still work.
export function initOutputChannel(): vscode.LogOutputChannel {
  channel ??= vscode.window.createOutputChannel("Deslop", { log: true });
  return channel;
}

export function logger(): Logger {
  if (rootLogger) return rootLogger;
  const out = initOutputChannel();
  rootLogger = pino(
    {
      name: "deslop-vscode",
      level: process.env.DESLOP_LOG_LEVEL ?? "debug",
      base: null,
      timestamp: pino.stdTimeFunctions.isoTime,
      formatters: { level: (label) => ({ level: label }) },
    },
    {
      write(chunk: string): void {
        out.append(chunk);
      },
    },
  );
  return rootLogger;
}

export function log(message: string, fields?: Record<string, unknown>): void {
  logger().info(fields ?? {}, message);
}

export function logWarn(message: string, fields?: Record<string, unknown>): void {
  logger().warn(fields ?? {}, message);
}

export function logError(err: unknown, context: string): void {
  const payload: LogDescriptor =
    err instanceof Error
      ? { err: { name: err.name, message: err.message, stack: err.stack }, context }
      : { err: String(err), context };
  logger().error(payload, `error in ${context}`);
  initOutputChannel().show(true);
}
