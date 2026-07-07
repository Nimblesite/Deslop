public class MixedDefaults
{
    public void ApplyStandard(RatePolicy policy)
    {
        var marker = 100;
        policy.SetMarker(marker);
        policy.EnableAlerts(marker);
        policy.Audit(marker);
        policy.Flush(marker);
        policy.Seal(marker);
        policy.Commit();
    }

    public void ApplyPremium(RatePolicy policy)
    {
        var marker = "premium";
        policy.SetMarker(marker);
        policy.EnableAlerts(marker);
        policy.Audit(marker);
        policy.Flush(marker);
        policy.Seal(marker);
        policy.Commit();
    }
}
