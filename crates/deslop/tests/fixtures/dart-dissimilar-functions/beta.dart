String describe(int code) {
  if (code == 200) {
    return 'ok';
  }
  if (code >= 500) {
    return 'server error';
  }
  return 'unknown';
}
