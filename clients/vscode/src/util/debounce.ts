// Trailing-edge debounce: coalesces a burst of calls into a single invocation
// after `delayMs` of quiet. The extension host runs every report-driven surface
// on the UI thread, so a flood of keystroke/store events must collapse into one
// unit of work rather than one-per-event ([VSIX-PERF]).
//
// The scheduler is injectable so coalescing is testable deterministically —
// production passes nothing and gets `setTimeout`.

/** A schedule starts a deferred callback and returns a cancel handle. */
export type ScheduleFn = (callback: () => void, delayMs: number) => () => void;

/** Debounced callable: invoking it (re)arms the trailing timer; `cancel` disarms. */
export interface Debounced {
  (): void;
  cancel(): void;
}

const scheduleWithTimeout: ScheduleFn = (callback, delayMs) => {
  const handle = setTimeout(callback, delayMs);
  return () => clearTimeout(handle);
};

/** Builds a trailing-edge debounced wrapper around `fn`. */
export function debounce(
  fn: () => void,
  delayMs: number,
  schedule: ScheduleFn = scheduleWithTimeout,
): Debounced {
  let cancelPending: (() => void) | undefined;
  return Object.assign(
    (): void => {
      cancelPending?.();
      cancelPending = schedule(() => {
        cancelPending = undefined;
        fn();
      }, delayMs);
    },
    {
      cancel: (): void => {
        cancelPending?.();
        cancelPending = undefined;
      },
    },
  );
}
