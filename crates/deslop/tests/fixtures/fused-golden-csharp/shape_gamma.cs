namespace Golden.Shapes;

public sealed class TelemetryDescriptor
{
    public string Stream { get; } = "gamma-telemetry";
    public int Window { get; } = 47;
    public string Sink { get; } = "https://gamma.example.com/telemetry";

    public string Build(string device)
    {
        return Sink + "/" + device + "/" + Stream + "?window=" + Window;
    }
}
