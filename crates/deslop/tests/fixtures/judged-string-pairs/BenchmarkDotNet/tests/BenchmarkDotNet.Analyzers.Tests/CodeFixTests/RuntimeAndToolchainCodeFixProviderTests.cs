public class Runtimeandtoolchaincodefixprovidertests
{
    [Fact]
    public async Task CodeFix_removes_runtime_from_chain_runtime_then_toolchain()
    {
        TestCode = /* lang=c#-test */ """
            using BenchmarkDotNet.Environments;
            using BenchmarkDotNet.Jobs;
            using BenchmarkDotNet.Toolchains.InProcess.Emit;

            public class Config
            {
                public Job Setup() => Job.Dry.{|#0:WithRuntime|}(CoreRuntime.Core80).WithToolchain(InProcessEmitToolchain.Default);
            }
            """.ReplaceLineEndings();

        FixedCode = /* lang=c#-test */ """
            using BenchmarkDotNet.Environments;
            using BenchmarkDotNet.Jobs;
            using BenchmarkDotNet.Toolchains.InProcess.Emit;

            public class Config
            {
                public Job Setup() => Job.Dry.WithToolchain(InProcessEmitToolchain.Default);
            }
            """.ReplaceLineEndings();

        AddExpectedDiagnostic(0);
        await RunAsync();
    }

    [Fact]
    public async Task CodeFix_removes_runtime_from_chain_toolchain_then_runtime()
    {
        TestCode = /* lang=c#-test */ """
            using BenchmarkDotNet.Environments;
            using BenchmarkDotNet.Jobs;
            using BenchmarkDotNet.Toolchains.InProcess.Emit;

            public class Config
            {
                public Job Setup() => Job.Dry.WithToolchain(InProcessEmitToolchain.Default).{|#0:WithRuntime|}(CoreRuntime.Core80);
            }
            """.ReplaceLineEndings();

        FixedCode = /* lang=c#-test */ """
            using BenchmarkDotNet.Environments;
            using BenchmarkDotNet.Jobs;
            using BenchmarkDotNet.Toolchains.InProcess.Emit;

            public class Config
            {
                public Job Setup() => Job.Dry.WithToolchain(InProcessEmitToolchain.Default);
            }
            """.ReplaceLineEndings();

        AddExpectedDiagnostic(0);
        await RunAsync();
    }
}
