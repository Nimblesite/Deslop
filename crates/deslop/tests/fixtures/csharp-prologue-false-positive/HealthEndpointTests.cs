using System.Net.Http.Json;
using System.Text.Json;
using Generated;
using Nimblesite.Sql.Model;
using Npgsql;
using Outcome;

namespace ICD10.TestSupport;

public class HealthEndpointTests
{
    public async Task HealthOk(HttpClient client)
    {
        var resp = await client.GetAsync("/health");
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadAsStringAsync();
        var doc = JsonDocument.Parse(json);
        var status = doc.RootElement.GetProperty("status").GetString();
        if (status != "healthy")
        {
            throw new Exception("not healthy");
        }
    }
}
