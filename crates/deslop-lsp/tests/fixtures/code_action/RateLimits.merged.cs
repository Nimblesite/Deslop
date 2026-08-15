public class RateLimits
{
    private static void MergedFromCluster_5f24f5(RatePolicy policy, string arg0, int arg1)
    {
        var label = arg0;
        var ceiling = arg1;
        policy.SetCeiling(label, ceiling);
        policy.EnableAlerts(label);
        policy.Audit(label, ceiling);
        policy.Flush(label);
        policy.Seal(ceiling);
        policy.Commit();
    }

    public void ApplyStandard(RatePolicy policy)
    {
        MergedFromCluster_5f24f5(policy, "standard", 100);
    }

    public void ApplyPremium(RatePolicy policy)
    {
        MergedFromCluster_5f24f5(policy, "premium", 250);
    }
}
