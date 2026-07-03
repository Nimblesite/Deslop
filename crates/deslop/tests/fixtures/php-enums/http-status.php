<?php

enum HttpStatus: int {
  case Ok = 200;
  case Created = 201;
  case BadRequest = 400;
  case Unauthorized = 401;
  case NotFound = 404;
}

function describeState(HttpStatus $state): string {
  return match ($state) {
    HttpStatus::Ok => "ok",
    HttpStatus::Created => "created",
    HttpStatus::BadRequest => "bad request",
    default => "unknown",
  };
}
