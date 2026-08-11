class TelemetryDescriptor:
    stream = "gamma-telemetry"
    window = 47
    sink = "https://gamma.example.com/telemetry"

    def build(self, device):
        return self.sink + "/" + device + "/" + self.stream + "?window=" + str(self.window)
