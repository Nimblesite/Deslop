namespace Ledger.SameFile;

public sealed class AlphaLedger
{
    private readonly decimal _alphaCeiling = 250m;

    public decimal Reconcile(decimal opening, decimal[] entries)
    {
        var balance = opening;
        foreach (var entry in entries)
        {
            if (entry < 0)
            {
                balance -= System.Math.Abs(entry);
            }
            else
            {
                balance += entry;
            }
        }
        return balance;
    }

    public bool WithinCeiling(decimal balance) => balance <= _alphaCeiling;
}

public sealed class BetaLedger
{
    private int _reconciliations;

    public decimal Reconcile(decimal opening, decimal[] entries)
    {
        var balance = opening;
        foreach (var entry in entries)
        {
            if (entry < 0)
            {
                balance -= System.Math.Abs(entry);
            }
            else
            {
                balance += entry;
            }
        }
        return balance;
    }

    public void Reset()
    {
        _reconciliations = 0;
    }
}
