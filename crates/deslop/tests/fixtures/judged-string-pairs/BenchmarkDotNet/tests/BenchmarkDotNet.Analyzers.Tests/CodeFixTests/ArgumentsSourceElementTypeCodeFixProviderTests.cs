public class ArgumentsSourceValueTupleCodeFixProviderTests
{
    private static string Code(string values, bool reported)
        => /* lang=c#-test */ $$"""
            using System.Collections.Generic;
            using BenchmarkDotNet.Attributes;

            public class BenchmarkClass
            {
            {{values}}
                [Benchmark]
                [ArgumentsSource({{(reported ? "{|#0:nameof(Values)|}" : "nameof(Values)")}})]
                public void Run(int number, string text) { }
            }
            """.ReplaceLineEndings();

    // Both halves are rewritten: the declaration and every value it hands back. Retyping one without the other
    // would leave the source not compiling, which is why the fix is only offered when all of them can be.
    [Fact]
    public async Task CodeFix_retypes_an_object_array_source_to_a_tuple()
    {
        Source(
            """
                public static IEnumerable<object[]> Values()
                {
                    yield return new object[] { 1, "one" };
                    yield return new object[] { 2, "two" };
                }

            """,
            """
                public static IEnumerable<(int, string)> Values()
                {
                    yield return (1, "one");
                    yield return (2, "two");
                }

            """);

        await RunAsync();
    }
} public class ArgumentsSourceByRefLikeUnwrapCodeFixProviderTests {
    private static string Code(string values, bool reported)
        => /* lang=c#-test */ $$"""
            using System;
            using System.Collections.Generic;
            using BenchmarkDotNet.Attributes;

            public class BenchmarkClass
            {
            {{values}}
                [Benchmark]
                [ArgumentsSource({{(reported ? "{|#0:nameof(Values)|}" : "nameof(Values)")}})]
                public void Run(ReadOnlySpan<byte> bytes) { }
            }
            """.ReplaceLineEndings();

    // No source can yield a ref struct, so the diagnostic has no type to name - but the rows do. What they hold
    // is what the source should be declared to yield, provided the parameter can be reached from it.
    [Fact]
    public async Task CodeFix_unwraps_a_row_feeding_a_by_ref_like_parameter()
    {
        Source(
            """
                public static IEnumerable<object[]> Values()
                {
                    yield return new object[] { new byte[] { 1, 2, 3 } };
                }

            """,
            """
                public static IEnumerable<byte[]> Values()
                {
                    yield return new byte[] { 1, 2, 3 };
                }

            """);

        await RunAsync();
    }
}
