using System.Collections.Generic;
using System.Linq;

namespace Examples.Collections
{
    // LINQ twin of GroupByImperative. `GroupBy` + `ToDictionary`.
    public static class GroupByLinq
    {
        public static Dictionary<string, decimal> TotalByCategory(
            IReadOnlyList<(string category, decimal amount)> transactions) =>
            transactions
                .GroupBy(tx => tx.category)
                .ToDictionary(group => group.Key, group => group.Sum(tx => tx.amount));

        public static Dictionary<string, int> CountByCategory(
            IReadOnlyList<(string category, decimal amount)> transactions) =>
            transactions
                .GroupBy(tx => tx.category)
                .ToDictionary(group => group.Key, group => group.Count());
    }
}
