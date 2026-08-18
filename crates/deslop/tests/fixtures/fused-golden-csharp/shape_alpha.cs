namespace Golden.Shapes;

public sealed class InventoryDescriptor
{
    public string Channel { get; } = "alpha-inventory";
    public int Retries { get; } = 3;
    public string Endpoint { get; } = "https://alpha.example.com/inventory";

    public string Compose(string tenant)
    {
        return Endpoint + "/" + tenant + "/" + Channel + "?retries=" + Retries;
    }
}
