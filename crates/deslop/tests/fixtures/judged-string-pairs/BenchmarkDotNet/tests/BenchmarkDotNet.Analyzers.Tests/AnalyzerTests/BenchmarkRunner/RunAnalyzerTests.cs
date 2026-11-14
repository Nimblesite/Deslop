public class Runanalyzertests
{
        public async Task Invoking_with_a_public_nonabstract_unsealed_nongeneric_type_argument_class_having_only_one_and_public_method_annotated_with_the_benchmark_attribute_should_not_trigger_diagnostic(
            bool isGenericInvocation)
        {
            const string classWithOneBenchmarkMethodName = "ClassWithOneBenchmarkMethod";

            var invocationExpression = isGenericInvocation ? $"<{classWithOneBenchmarkMethodName}>()" : $"(typeof({classWithOneBenchmarkMethodName}))";

            var testCode = /* lang=c#-test */ $$"""
                using BenchmarkDotNet.Running;

                public class Program
                {
                    public static void Main(string[] args) {
                        BenchmarkRunner.Run{{invocationExpression}};
                    }
                }
                """;

            const string benchmarkClassDocument = /* lang=c#-test */ $$"""
                using BenchmarkDotNet.Attributes;

                public class {{classWithOneBenchmarkMethodName}}
                {
                    [Benchmark]
                    public void BenchmarkMethod()
                    {

                    }
                }
                """;

            TestCode = testCode;
            AddSource(benchmarkClassDocument);

            await RunAsync();
        }

        public async Task Invoking_with_type_argument_class_having_at_least_one_public_method_annotated_with_the_benchmark_attribute_should_not_trigger_diagnostic(bool isGenericInvocation)
        {
            const string classWithOneBenchmarkMethodName = "ClassWithOneBenchmarkMethod";

            var invocationExpression = isGenericInvocation ? $"<{classWithOneBenchmarkMethodName}>()" : $"(typeof({classWithOneBenchmarkMethodName}))";

            var testCode = /* lang=c#-test */ $$"""
                using BenchmarkDotNet.Running;
                                               
                public class Program
                {
                    public static void Main(string[] args) {
                        BenchmarkRunner.Run{{invocationExpression}};
                    }
                }
                """;

            const string benchmarkClassDocument = /* lang=c#-test */ $$"""
                using BenchmarkDotNet.Attributes;

                public class {{classWithOneBenchmarkMethodName}}
                {
                    [Benchmark]
                    public void BenchmarkMethod()
                    {

                    }

                    public void BenchmarkMethod2()
                    {

                    }

                    private void BenchmarkMethod3()
                    {

                    }
                }
                """;

            TestCode = testCode;
            AddSource(benchmarkClassDocument);

            await RunAsync();
        }
}
