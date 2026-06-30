export enum TaskState {
  Queued = 200,
  Running = 201,
  Failed = 400,
  Blocked = 401,
  Missing = 404,
}

export function describeState(state: TaskState): string {
  switch (state) {
    case TaskState.Queued:
      return "queued";
    case TaskState.Running:
      return "running";
    case TaskState.Failed:
      return "failed";
    default:
      return "unknown";
  }
}
