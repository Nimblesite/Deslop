using System.Net.Http.Json;
using System.Text.Json;
using Generated;
using Nimblesite.Sql.Model;
using Npgsql;
using Outcome;

namespace ICD10.TestSupport;

public class AchiEndpointTests
{
    private readonly HttpClient _client;

    public AchiEndpointTests(HttpClient c)
    {
        _client = c;
    }

    public async Task AchiOk()
    {
        var response = await _client.PostAsync("/achi", new StringContent("hello"));
        response.EnsureSuccessStatusCode();
    }
}
