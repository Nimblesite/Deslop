public class BenchmarkClassAnalyzerTests
{
    public class SingleNullArgumentToBenchmarkCategoryAttributeNotAllowed : AnalyzerTestFixture<BenchmarkClassAnalyzer>
    {
        public SingleNullArgumentToBenchmarkCategoryAttributeNotAllowed() : base(BenchmarkClassAnalyzer.SingleNullArgumentToBenchmarkCategoryAttributeNotAllowedRule)
        {
        }

        [Fact]
        public async Task A_null_argument_to_a_derived_category_attribute_does_not_report_error()
        {
            // The rule is about a null *category*. A derived attribute's single argument is whatever its own
            // constructor takes, and the category it forwards to base(...) is not null at all.
            var testCode = /* lang=c#-test */ """
                using BenchmarkDotNet.Attributes;

                public class LabelledCategoryAttribute : BenchmarkCategoryAttribute
                {
                    public LabelledCategoryAttribute(string label) : base("fixed") { }
                }

                public class BenchmarkClass
                {
                    [Benchmark]
                    [LabelledCategory(null)]
                    public void Run() { }
                }
                """;

            TestCode = testCode;
            DisableCompilerDiagnostics();
            await RunAsync();
        }
    }

    public class OnlyOneMethodCanBeBaselinePerCategory : AnalyzerTestFixture<BenchmarkClassAnalyzer>
    {
        public OnlyOneMethodCanBeBaselinePerCategory() : base(BenchmarkClassAnalyzer.OnlyOneMethodCanBeBaselinePerCategoryRule) { }

        [Fact]
        public async Task Derived_category_attributes_sharing_an_argument_do_not_report_error()
        {
            // Harvesting a derived attribute's own arguments as the categories reports the wrong set rather than an
            // unknown one: these two share a label but forward different categories, so neither baseline collides.
            var testCode = /* lang=c#-test */ """
                using BenchmarkDotNet.Attributes;

                public class ACategoryAttribute : BenchmarkCategoryAttribute
                {
                    public ACategoryAttribute(string label) : base("A") { }
                }

                public class BCategoryAttribute : BenchmarkCategoryAttribute
                {
                    public BCategoryAttribute(string label) : base("B") { }
                }

                public class BenchmarkClass
                {
                    [Benchmark(Baseline = true)]
                    [ACategory("same")]
                    public void First() { }

                    [Benchmark(Baseline = true)]
                    [BCategory("same")]
                    public void Second() { }
                }
                """;

            TestCode = testCode;
            DisableCompilerDiagnostics();
            await RunAsync();
        }
    }
}
