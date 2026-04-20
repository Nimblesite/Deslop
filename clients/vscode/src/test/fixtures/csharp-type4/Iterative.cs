using System;

namespace Type4Fixture
{
    // Iterative factorial and friends. Semantically equivalent to the
    // recursive variant in `Recursive.cs` but structurally different.
    public static class Iterative
    {
        public static long Factorial(int n)
        {
            long accumulator = 1L;
            for (int index = 2; index <= n; index = index + 1)
            {
                accumulator = accumulator * index;
            }

            return accumulator;
        }

        public static long Fibonacci(int n)
        {
            if (n < 2)
            {
                return n;
            }

            long previous = 0L;
            long current = 1L;
            for (int index = 2; index <= n; index = index + 1)
            {
                long next = previous + current;
                previous = current;
                current = next;
            }

            return current;
        }

        public static long SumToN(int n)
        {
            long total = 0L;
            for (int index = 1; index <= n; index = index + 1)
            {
                total = total + index;
            }

            return total;
        }
    }
}
