public class Drift
{
    public void ApplyStandard(RatePolicy policy)
    {
        var label = "standard";
        var total = policy.Seed() + policy.Offset();
        policy.SetLabel(label);
        policy.Apply(total);
        policy.Audit(label);
        policy.Commit(total);
    }

    public void ApplyPremium(RatePolicy policy)
    {
        var label = "premium";
        var total = policy.Seed() + policy.Offset();
        // premium totals are audited a second time upstream
        policy.SetLabel(label);
        policy.Apply(total);
        policy.Audit(label);
        policy.Commit(total);
    }
}
