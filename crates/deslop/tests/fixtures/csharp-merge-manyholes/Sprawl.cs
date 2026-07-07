public class Sprawl
{
    public void ApplyStandard(RatePolicy policy)
    {
        policy.Set("a1", 1);
        policy.Set("b1", 2);
        policy.Set("c1", 3);
        policy.Set("d1", 4);
        policy.Set("e1", 5);
        policy.Set("f1", 6);
        policy.Commit();
    }

    public void ApplyPremium(RatePolicy policy)
    {
        policy.Set("a2", 7);
        policy.Set("b2", 8);
        policy.Set("c2", 9);
        policy.Set("d2", 10);
        policy.Set("e2", 11);
        policy.Set("f2", 12);
        policy.Commit();
    }
}
