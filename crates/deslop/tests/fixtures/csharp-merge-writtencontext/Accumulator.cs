public class Accumulator
{
    public void GrowStandard(RatePolicy policy)
    {
        int total = policy.Baseline();
        total = total + 1;
        policy.Apply(total, 5);
        policy.Log(total);
        policy.Meter(total);
        policy.Trace(total);
        policy.Commit(total);
    }

    public void GrowPremium(RatePolicy policy)
    {
        int total = 0;
        total = total + 1;
        policy.Apply(total, 9);
        policy.Log(total);
        policy.Meter(total);
        policy.Trace(total);
        policy.Commit(total);
    }
}
