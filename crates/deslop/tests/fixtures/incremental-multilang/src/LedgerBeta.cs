// LedgerBeta.cs — the pasted copy of the C# reconciliation routine.

namespace Ledger;

public sealed record LedgerBetaCursor(int Offset);

public sealed class LedgerBeta
{
    public static long ReconcileEntries(long[] entries, long floor)
    {
        long balance = 0;
        foreach (var entry in entries)
        {
            if (entry > floor)
            {
                balance += entry * 2;
            }
            else
            {
                balance -= entry / 2;
            }
        }
        return balance;
    }
}
