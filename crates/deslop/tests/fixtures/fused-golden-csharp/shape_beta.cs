namespace Golden.Shapes;

public sealed class BillingDescriptor
{
    public string Region { get; } = "beta-billing";
    public int Attempts { get; } = 9;
    public string Origin { get; } = "https://beta.example.com/billing";

    public string Assemble(string account)
    {
        return Origin + "/" + account + "/" + Region + "?attempts=" + Attempts;
    }
}
