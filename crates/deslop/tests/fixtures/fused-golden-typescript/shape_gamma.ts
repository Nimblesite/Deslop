export class TelemetryDescriptor {
  readonly stream = "gamma-telemetry";
  readonly window = 47;
  readonly sink = "https://gamma.example.com/telemetry";

  build(device: string): string {
    return this.sink + "/" + device + "/" + this.stream + "?window=" + String(this.window);
  }
}
