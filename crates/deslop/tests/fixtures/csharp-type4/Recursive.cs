using System;

namespace Type4Fixture
{
    // Recursive factorial. Semantically equivalent to the iterative
    // variant in `Iterative.cs` but syntactically different — only
    // the embedding pass can see the similarity.
    public static class Recursive
    {
        public static long Factorial(int n)
        {
            if (n <= 1)
            {
                return 1L;
            }

            return n * Factorial(n - 1);
        }

        public static long Fibonacci(int n)
        {
            if (n < 2)
            {
                return n;
            }

            return Fibonacci(n - 1) + Fibonacci(n - 2);
        }

        public static long SumToN(int n)
        {
            if (n <= 0)
            {
                return 0L;
            }

            return n + SumToN(n - 1);
        }
    }
}
