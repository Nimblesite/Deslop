export enum TaskState {
  Queued = 200,
  Running = 201,
  Failed = 400,
  Blocked = 401,
  Missing = 404,
}

export function describe(code: TaskState): string {
  let label = "unknown";
  let retries = 0;
  switch (code) {
    case TaskState.Queued:
      label = "accepted";
      retries = retries + 1;
      break;
    case TaskState.Running:
      label = "stored";
      retries = retries + 2;
      break;
    case TaskState.Failed:
      label = "rejected";
      retries = retries + 3;
      break;
    default:
      label = "unmapped";
  }
  return label + retries;
}
