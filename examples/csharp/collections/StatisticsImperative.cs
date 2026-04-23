using System.Collections.Generic;

namespace Examples.Collections
{
    // Imperative statistics. All three methods have a matching
    // functional twin in StatisticsFunctional — cross-file same-behavior,
    // different-code clusters that only the embedding pass surfaces.
    public static class StatisticsImperative
    {
        public static double Mean(IReadOnlyList<double> values)
        {
            if (values.Count == 0)
            {
                return 0.0;
            }

            double total = 0.0;
            for (int index = 0; index < values.Count; index = index + 1)
            {
                total = total + values[index];
            }

            return total / values.Count;
        }

        public static double Variance(IReadOnlyList<double> values)
        {
            if (values.Count < 2)
            {
                return 0.0;
            }

            double average = Mean(values);
            double squaredDeviation = 0.0;
            for (int index = 0; index < values.Count; index = index + 1)
            {
                double delta = values[index] - average;
                squaredDeviation = squaredDeviation + delta * delta;
            }

            return squaredDeviation / (values.Count - 1);
        }

        public static double Max(IReadOnlyList<double> values)
        {
            double best = double.NegativeInfinity;
            for (int index = 0; index < values.Count; index = index + 1)
            {
                if (values[index] > best)
                {
                    best = values[index];
                }
            }

            return best;
        }
    }
}
