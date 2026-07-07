public class Prefix
{
    public void ApplyStandard(RatePolicy policy)
    {
        var label = "standard";
        var ticket = policy.Load(label);
        policy.Stage(ticket);
        policy.Validate(ticket);
        policy.Record(ticket);
        policy.Publish(ticket);
        policy.Send(ticket);
    }

    public void ApplyPremium(RatePolicy policy)
    {
        var label = "premium";
        var ticket = policy.Load(label);
        policy.Stage(ticket);
        policy.Validate(ticket);
        policy.Record(ticket);
        policy.Publish(ticket);
        if (policy.ShouldArchive())
        {
            policy.Archive(ticket);
        }
    }
}
