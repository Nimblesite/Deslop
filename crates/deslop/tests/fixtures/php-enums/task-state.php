<?php

enum TaskState: int {
  case Queued = 200;
  case Running = 201;
  case Failed = 400;
  case Blocked = 401;
  case Missing = 404;
}

function describeState(TaskState $state): string {
  return match (state) {
    TaskState::Queued => "queued",
    TaskState::Running => "running",
    TaskState::Failed => "failed",
    default => "unknown",
  };
}
