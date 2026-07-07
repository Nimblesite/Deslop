public class Router
{
    public void WireStandard(RatePolicy policy)
    {
        int standardLimit = 100;
        policy.Apply(standardLimit);
        policy.Log(standardLimit);
        policy.Meter(standardLimit);
        policy.Trace(standardLimit);
        policy.Flush(standardLimit);
        policy.Commit(standardLimit);
    }

    public void WirePremium(RatePolicy policy)
    {
        int premiumLimit = policy.Baseline() + 50;
        policy.Apply(premiumLimit);
        policy.Log(premiumLimit);
        policy.Meter(premiumLimit);
        policy.Trace(premiumLimit);
        policy.Flush(premiumLimit);
        policy.Commit(premiumLimit);
    }
}
