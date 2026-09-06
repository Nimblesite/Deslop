const PLATFORM_TARGETS = new Map([
  ["darwin-arm64", "darwin-arm64"],
  ["darwin-x64", "darwin-x64"],
  ["linux-x64", "linux-x64"],
  ["linux-arm64", "linux-arm64"],
  ["win32-x64", "win32-x64"],
]);

export function currentPlatformTarget() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_TARGETS.get(key);
  if (!target) throw new Error(`unsupported platform ${key}`);
  return target;
}
