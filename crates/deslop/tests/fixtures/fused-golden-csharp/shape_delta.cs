namespace Golden.Shapes;

public sealed class AuditDescriptor
{
    public string Ledger { get; } = "delta-audit";
    public int Depth { get; } = 128;
    public string Archive { get; } = "https://delta.example.com/audit";

    public string Render(string actor)
    {
        return Archive + "/" + actor + "/" + Ledger + "?depth=" + Depth;
    }
}
