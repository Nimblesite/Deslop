using BenchmarkDotNet.Analyzers.Attributes;
using BenchmarkDotNet.Analyzers.Tests.Fixtures;

namespace BenchmarkDotNet.Analyzers.Tests.AnalyzerTests.Attributes;

public class SetupCleanupAsyncEnumerableAnalyzerTests
{
    public class MustNotReturnAsyncEnumerable : AnalyzerTestFixture<SetupCleanupAsyncEnumerableAnalyzer>
    {
        public MustNotReturnAsyncEnumerable() : base(SetupCleanupAsyncEnumerableAnalyzer.MustNotReturnAsyncEnumerableRule) { }

        [Fact]
        public async Task GlobalSetup_returning_IAsyncEnumerable_should_trigger_diagnostic()
        {
            var testCode = /* lang=c#-test */ """
                using BenchmarkDotNet.Attributes;
                using System.Collections.Generic;
                using System.Threading.Tasks;

                public class BenchmarkClass
                {
                    [GlobalSetup]
                    public async {|#0:IAsyncEnumerable<int>|} Setup()
                    {
                        await Task.Yield();
                        yield return 1;
                    }

                    [Benchmark]
                    public void Benchmark() { }
                }
                """;

            TestCode = testCode;
            AddExpectedDiagnostic(0, "GlobalSetup", "Setup");

            await RunAsync();
        }

        [Fact]
        public async Task GlobalCleanup_returning_IAsyncEnumerable_should_trigger_diagnostic()
        {
            var testCode = /* lang=c#-test */ """
                using BenchmarkDotNet.Attributes;
                using System.Collections.Generic;
                using System.Threading.Tasks;

                public class BenchmarkClass
                {
                    [GlobalCleanup]
                    public async {|#0:IAsyncEnumerable<int>|} Cleanup()
                    {
                        await Task.Yield();
                        yield return 1;
                    }

                    [Benchmark]
                    public void Benchmark() { }
                }
                """;

            TestCode = testCode;
            AddExpectedDiagnostic(0, "GlobalCleanup", "Cleanup");

            await RunAsync();
        }

        [Fact]
        public async Task IterationSetup_returning_IAsyncEnumerable_should_trigger_diagnostic()
        {
            var testCode = /* lang=c#-test */ """
                using BenchmarkDotNet.Attributes;
                using System.Collections.Generic;
                using System.Threading.Tasks;

                public class BenchmarkClass
                {
                    [IterationSetup]
                    public async {|#0:IAsyncEnumerable<int>|} Setup()
                    {
                        await Task.Yield();
                        yield return 1;
                    }

                    [Benchmark]
                    public void Benchmark() { }
                }
                """;

            TestCode = testCode;
            AddExpectedDiagnostic(0, "IterationSetup", "Setup");

            await RunAsync();
        }

        [Fact]
        public async Task IterationCleanup_returning_IAsyncEnumerable_should_trigger_diagnostic()
        {
            var testCode = /* lang=c#-test */ """
                using BenchmarkDotNet.Attributes;
                using System.Collections.Generic;
                using System.Threading.Tasks;

                public class BenchmarkClass
                {
                    [IterationCleanup]
                    public async {|#0:IAsyncEnumerable<int>|} Cleanup()
                    {
                        await Task.Yield();
                        yield return 1;
                    }

                    [Benchmark]
                    public void Benchmark() { }
                }
                """;

            TestCode = testCode;
            AddExpectedDiagnostic(0, "IterationCleanup", "Cleanup");

            await RunAsync();
        }
    }
}
