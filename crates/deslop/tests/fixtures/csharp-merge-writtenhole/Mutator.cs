public class Mutator
{
    public void GrowStandard(RatePolicy policy)
    {
        int alpha = 100;
        alpha = alpha + 1;
        policy.Apply(alpha);
        policy.Log(alpha);
        policy.Meter(alpha);
        policy.Trace(alpha);
        policy.Commit(alpha);
    }

    public void GrowPremium(RatePolicy policy)
    {
        int beta = policy.Baseline() + 50;
        beta = beta + 1;
        policy.Apply(beta);
        policy.Log(beta);
        policy.Meter(beta);
        policy.Trace(beta);
        policy.Commit(beta);
    }
}
