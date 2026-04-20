using System.Collections.Generic;
using System.Linq;

namespace Examples.Collections
{
    // Functional twin of StatisticsImperative. Same three semantics
    // expressed via LINQ. Structural signal = 0; token Jaccard close to
    // 0; embedding cosine high — the only way to catch these as clones.
    public static class StatisticsFunctional
    {
        public static double Mean(IReadOnlyList<double> values) =>
            values.Count == 0 ? 0.0 : values.Average();

        public static double Variance(IReadOnlyList<double> values)
        {
            if (values.Count < 2)
            {
                return 0.0;
            }

            var average = values.Average();
            return values.Sum(value => (value - average) * (value - average)) / (values.Count - 1);
        }

        public static double Max(IReadOnlyList<double> values) =>
            values.DefaultIfEmpty(double.NegativeInfinity).Max();
    }
}
