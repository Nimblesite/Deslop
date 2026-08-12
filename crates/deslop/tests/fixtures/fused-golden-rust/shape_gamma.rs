pub struct TelemetryDescriptor {
    pub stream: String,
    pub window: i64,
    pub sink: String,
}

impl TelemetryDescriptor {
    pub fn build(&self, device: &str) -> String {
        format!("{}/{}/{}?window=gamma", self.sink, device, self.stream)
    }
}
