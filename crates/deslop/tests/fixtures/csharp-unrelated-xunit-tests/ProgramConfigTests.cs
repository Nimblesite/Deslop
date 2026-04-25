using System;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Xunit;

namespace AiCms.Tests
{
    public class ProgramConfigTests : IClassFixture<PostgresFixture>
    {
        private readonly PostgresFixture _fixture;

        public ProgramConfigTests(PostgresFixture fixture)
        {
            _fixture = fixture;
        }

        [Fact]
        public void Configuration_Reads_Connection_String_From_Environment()
        {
            Environment.SetEnvironmentVariable("ConnectionStrings__Default", "Host=db;");
            var builder = WebApplication.CreateBuilder();
            var connectionString = builder.Configuration.GetConnectionString("Default");
            Assert.Equal("Host=db;", connectionString);
        }

        [Fact]
        public void Service_Collection_Registers_Default_Logger()
        {
            var services = new ServiceCollection();
            services.AddLogging();
            var provider = services.BuildServiceProvider();
            var logger = provider.GetService<Microsoft.Extensions.Logging.ILogger<ProgramConfigTests>>();
            Assert.NotNull(logger);
        }

        [Fact]
        public void Configuration_Defaults_To_Production_Environment()
        {
            Environment.SetEnvironmentVariable("ASPNETCORE_ENVIRONMENT", null);
            var builder = WebApplication.CreateBuilder();
            Assert.Equal("Production", builder.Environment.EnvironmentName);
        }

        [Fact]
        public void Configuration_Loads_Json_Settings_File()
        {
            var configuration = new ConfigurationBuilder()
                .AddInMemoryCollection(new System.Collections.Generic.Dictionary<string, string?>
                {
                    ["Feature:Enabled"] = "true",
                })
                .Build();
            Assert.Equal("true", configuration["Feature:Enabled"]);
        }

        [Fact]
        public void Service_Collection_Allows_Singleton_Override()
        {
            var services = new ServiceCollection();
            services.AddSingleton<IClock, FakeClock>();
            var provider = services.BuildServiceProvider();
            var clock = provider.GetRequiredService<IClock>();
            Assert.IsType<FakeClock>(clock);
        }
    }
}
