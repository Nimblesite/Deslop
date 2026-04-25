using System.Net;
using System.Net.Http;
using System.Net.Http.Json;
using System.Threading.Tasks;
using Xunit;

namespace AiCms.Tests
{
    public class EndpointWorkflowTests : IClassFixture<PostgresFixture>
    {
        private readonly PostgresFixture _fixture;

        public EndpointWorkflowTests(PostgresFixture fixture)
        {
            _fixture = fixture;
        }

        [Fact]
        public async Task Full_Endpoint_Workflow_Creates_Updates_And_Deletes_Site()
        {
            using var client = _fixture.CreateClient();
            var createResponse = await client.PostAsJsonAsync("/sites", new { Name = "Acme" });
            Assert.Equal(HttpStatusCode.Created, createResponse.StatusCode);
            var created = await createResponse.Content.ReadFromJsonAsync<SiteDto>();
            Assert.NotNull(created);

            var getResponse = await client.GetAsync($"/sites/{created!.Id}");
            Assert.Equal(HttpStatusCode.OK, getResponse.StatusCode);
            var fetched = await getResponse.Content.ReadFromJsonAsync<SiteDto>();
            Assert.NotNull(fetched);
            Assert.Equal("Acme", fetched!.Name);

            var updateResponse = await client.PutAsJsonAsync(
                $"/sites/{created.Id}",
                new { Name = "Acme Renamed" });
            Assert.Equal(HttpStatusCode.OK, updateResponse.StatusCode);
            var updated = await updateResponse.Content.ReadFromJsonAsync<SiteDto>();
            Assert.NotNull(updated);
            Assert.Equal("Acme Renamed", updated!.Name);

            var deleteResponse = await client.DeleteAsync($"/sites/{created.Id}");
            Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

            var missingResponse = await client.GetAsync($"/sites/{created.Id}");
            Assert.Equal(HttpStatusCode.NotFound, missingResponse.StatusCode);
        }
    }
}
