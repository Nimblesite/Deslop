using System.Net.Http.Json;
using Xunit;

namespace ICD10.Api.Tests
{
    public class CodeLookupTests
    {
        private readonly HttpClient _client;

        public CodeLookupTests(HttpClient client)
        {
            _client = client;
        }

        [Fact]
        public async Task GetCodeByCode_ReturnsOk_WhenCodeExists()
        {
            var response = await _client.GetAsync("/api/codes/A00.0");
            response.EnsureSuccessStatusCode();
            var body = await response.Content.ReadAsStringAsync();
            Assert.NotNull(body);
            Assert.NotEmpty(body);
        }

        [Fact]
        public async Task GetCodeByCode_ReturnsOk_WhenCodeMatches()
        {
            var response = await _client.GetAsync("/api/codes/B00.0");
            response.EnsureSuccessStatusCode();
            var body = await response.Content.ReadAsStringAsync();
            Assert.NotNull(body);
            Assert.NotEmpty(body);
        }
    }
}
