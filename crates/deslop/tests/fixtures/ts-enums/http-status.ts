export enum HttpStatus {
  Ok = 200,
  Created = 201,
  BadRequest = 400,
  Unauthorized = 401,
  NotFound = 404,
}

export function describe(code: HttpStatus): string {
  let label = "unknown";
  let retries = 0;
  switch (code) {
    case HttpStatus.Ok:
      label = "accepted";
      retries = retries + 1;
      break;
    case HttpStatus.Created:
      label = "stored";
      retries = retries + 2;
      break;
    case HttpStatus.BadRequest:
      label = "rejected";
      retries = retries + 3;
      break;
    default:
      label = "unmapped";
  }
  return label + retries;
}
