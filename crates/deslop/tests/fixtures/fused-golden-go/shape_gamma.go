package golden

type TelemetryDescriptor struct {
	Stream string
	Window int
	Sink   string
}

func (probe TelemetryDescriptor) Build(device string) string {
	return probe.Sink + "/" + device + "/" + probe.Stream + "?window=gamma"
}
