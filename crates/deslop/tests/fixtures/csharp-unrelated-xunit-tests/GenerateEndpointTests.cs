using System.IO;
using System.Threading.Tasks;
using Xunit;

namespace AiCms.Tests
{
    public class GenerateEndpointTests : IClassFixture<PostgresFixture>
    {
        private readonly PostgresFixture _fixture;

        public GenerateEndpointTests(PostgresFixture fixture)
        {
            _fixture = fixture;
        }

        [Fact]
        public async Task GenerateSiteAsync_Renders_Static_Index_Html()
        {
            var generator = new SiteGenerator(_fixture.Database);
            var outputDir = Path.Combine(Path.GetTempPath(), "site-out");
            Directory.CreateDirectory(outputDir);
            await generator.GenerateSiteAsync(siteId: 1, outputDir: outputDir);
            var indexPath = Path.Combine(outputDir, "index.html");
            Assert.True(File.Exists(indexPath));
            var html = await File.ReadAllTextAsync(indexPath);
            Assert.Contains("<html", html);
            Assert.Contains("</html>", html);
        }

        [Fact]
        public async Task GenerateSiteAsync_Skips_Sites_That_Have_No_Pages()
        {
            var generator = new SiteGenerator(_fixture.Database);
            var outputDir = Path.Combine(Path.GetTempPath(), "site-empty");
            Directory.CreateDirectory(outputDir);
            await generator.GenerateSiteAsync(siteId: 999, outputDir: outputDir);
            Assert.False(File.Exists(Path.Combine(outputDir, "index.html")));
        }

        [Fact]
        public async Task GenerateSiteAsync_Writes_One_Html_File_Per_Page()
        {
            var generator = new SiteGenerator(_fixture.Database);
            var outputDir = Path.Combine(Path.GetTempPath(), "site-pages");
            Directory.CreateDirectory(outputDir);
            await generator.GenerateSiteAsync(siteId: 2, outputDir: outputDir);
            var pageFiles = Directory.GetFiles(outputDir, "*.html");
            Assert.Equal(3, pageFiles.Length);
        }
    }
}
