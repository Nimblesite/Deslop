export enum HttpStatus {
  Ok = 200,
  Created = 201,
  BadRequest = 400,
  Unauthorized = 401,
  NotFound = 404,
}

export function describeStatus(status: HttpStatus): string {
  switch (status) {
    case HttpStatus.Ok:
      return "ok";
    case HttpStatus.Created:
      return "created";
    case HttpStatus.BadRequest:
      return "bad request";
    default:
      return "unknown";
  }
}
