public class EarlyExit
{
    public void ApplyStandard(RatePolicy policy)
    {
        var label = "standard";
        policy.SetLabel(label);
        policy.EnableAlerts(label);
        policy.Audit(label);
        if (policy.Rejected(label))
        {
            return;
        }
        policy.Commit(label);
    }

    public void ApplyPremium(RatePolicy policy)
    {
        var label = "premium";
        policy.SetLabel(label);
        policy.EnableAlerts(label);
        policy.Audit(label);
        if (policy.Rejected(label))
        {
            return;
        }
        policy.Commit(label);
    }
}
