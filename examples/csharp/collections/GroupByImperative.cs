using System.Collections.Generic;

namespace Examples.Collections
{
    // Imperative group-by over transactions. Dictionary + foreach.
    // Pairs with GroupByLinq below (Type-4), and with GroupByStreaming
    // (Type-3 / Type-4 depending on the subtree).
    public static class GroupByImperative
    {
        public static Dictionary<string, decimal> TotalByCategory(
            IReadOnlyList<(string category, decimal amount)> transactions)
        {
            var totals = new Dictionary<string, decimal>();
            foreach (var tx in transactions)
            {
                if (totals.ContainsKey(tx.category))
                {
                    totals[tx.category] = totals[tx.category] + tx.amount;
                }
                else
                {
                    totals[tx.category] = tx.amount;
                }
            }

            return totals;
        }

        public static Dictionary<string, int> CountByCategory(
            IReadOnlyList<(string category, decimal amount)> transactions)
        {
            var counts = new Dictionary<string, int>();
            foreach (var tx in transactions)
            {
                if (counts.ContainsKey(tx.category))
                {
                    counts[tx.category] = counts[tx.category] + 1;
                }
                else
                {
                    counts[tx.category] = 1;
                }
            }

            return counts;
        }
    }
}
