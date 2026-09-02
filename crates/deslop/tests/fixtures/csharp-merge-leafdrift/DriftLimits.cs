public class DriftLimits
{
    public void ApplyStandard(RatePolicy policy)
    {
        var label = "standard";
        var ceiling = 100;
        policy.SetCeiling(label, ceiling);
        policy.EnableAlerts(label);
        policy.Audit(label, ceiling);
        policy.Flush(label);
        policy.Seal(ceiling);
        policy.Commit();
    }

    public void ApplyPremium(RatePolicy policy)
    {
        var label = "premium";
        var ceiling = 250.5;
        policy.SetCeiling(label, ceiling);
        policy.EnableAlerts(label);
        policy.Audit(label, ceiling);
        policy.Flush(label);
        policy.Seal(ceiling);
        policy.Commit();
    }
}
