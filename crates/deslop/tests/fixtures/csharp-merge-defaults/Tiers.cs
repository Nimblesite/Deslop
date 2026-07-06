public class Tiers
{
    public void ApplyBronze(RatePolicy policy)
    {
        var label = "bronze";
        var ceiling = 100;
        policy.SetCeiling(label, ceiling);
        policy.EnableAlerts(label);
        policy.Audit(label, ceiling);
        policy.Flush(label);
        policy.Commit();
    }

    public void ApplySilver(RatePolicy policy)
    {
        var label = "silver";
        var ceiling = 100;
        policy.SetCeiling(label, ceiling);
        policy.EnableAlerts(label);
        policy.Audit(label, ceiling);
        policy.Flush(label);
        policy.Commit();
    }

    public void ApplyGold(RatePolicy policy)
    {
        var label = "gold";
        var ceiling = 250;
        policy.SetCeiling(label, ceiling);
        policy.EnableAlerts(label);
        policy.Audit(label, ceiling);
        policy.Flush(label);
        policy.Commit();
    }
}
